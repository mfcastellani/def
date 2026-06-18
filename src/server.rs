use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

use crate::error::{DefError, DefResult};
use crate::interpreter::Interpreter;
use crate::lexer::Lexer;
use crate::parser::Parser as DefParser;
use crate::value::MockValue;

pub fn serve(file: &str, port: u16) -> DefResult<()> {
    let source = fs::read_to_string(file)
        .map_err(|e| DefError::Runtime(format!("failed to read '{file}': {e}")))?;

    let tokens = Lexer::new(&source).tokenize().map_err(|e| e.in_file(file))?;
    let program = DefParser::new(tokens).parse_program().map_err(|e| e.in_file(file))?;

    let base_dir = Path::new(file).parent().unwrap_or(Path::new("."));
    let mut interpreter = Interpreter::with_base_dir(base_dir).with_source_file(file);
    interpreter.interpret(&program)?;

    let mut mocks = interpreter.mocks();
    // Stable order: method then path
    mocks.sort_by(|(_, a), (_, b)| {
        a.method
            .cmp(&b.method)
            .then_with(|| extract_path(&a.url).cmp(extract_path(&b.url)))
    });

    validate_mocks(&mocks)?;

    println!("Def mock server running at http://localhost:{port}");
    println!("Loaded mocks:");
    for (_, mock) in &mocks {
        let path = extract_path(&mock.url);
        println!("  {} {} -> {}", mock.method.to_ascii_uppercase(), path, mock.status);
    }
    println!();

    let addr = format!("0.0.0.0:{port}");
    let server = tiny_http::Server::http(&addr)
        .map_err(|e| DefError::Runtime(format!("failed to start server on {addr}: {e}")))?;

    println!("Listening on {addr} — press Ctrl+C to stop");
    println!();

    for request in server.incoming_requests() {
        handle_request(request, &mocks);
    }

    Ok(())
}

// ── validation ────────────────────────────────────────────────────────────────

fn validate_mocks(mocks: &[(String, MockValue)]) -> DefResult<()> {
    for (name, mock) in mocks {
        if !mock.configured {
            return Err(DefError::Runtime(format!(
                "mock '{name}' ({} {}) has no configured response — add .reply() or .fail()",
                mock.method.to_ascii_uppercase(),
                extract_path(&mock.url),
            )));
        }
    }

    let mut seen: HashMap<(String, String), String> = HashMap::new();
    for (name, mock) in mocks {
        let key = (
            mock.method.to_ascii_uppercase(),
            extract_path(&mock.url).to_string(),
        );
        if let Some(first) = seen.get(&key) {
            return Err(DefError::Runtime(format!(
                "duplicate mock: {} {} is defined by both '{}' and '{}'",
                key.0, key.1, first, name,
            )));
        }
        seen.insert(key, name.clone());
    }

    Ok(())
}

// ── request handling ──────────────────────────────────────────────────────────

fn handle_request(request: tiny_http::Request, mocks: &[(String, MockValue)]) {
    let method = request.method().to_string();
    let raw_url = request.url().to_string();
    let path = strip_query(&raw_url).to_string();
    let remote = request
        .remote_addr()
        .map(|a| a.to_string())
        .unwrap_or_else(|| "-".to_string());

    let start = Instant::now();

    let (status, body, headers) = match find_mock(&method, &path, mocks) {
        Some(mock) if mock.snapshot_path.is_some() => {
            // TODO: read snapshot file and replay its body/headers/status
            let snap = mock.snapshot_path.as_deref().unwrap_or("");
            (
                501u16,
                format!("snapshot serving not yet implemented (path: {snap})"),
                vec![],
            )
        }
        Some(mock) => {
            if mock.delay_ms > 0 {
                thread::sleep(Duration::from_millis(mock.delay_ms));
            }
            (mock.status as u16, mock.body.clone(), mock.headers.clone())
        }
        None => (
            404u16,
            format!("mock not found: {method} {path}"),
            vec![],
        ),
    };

    let elapsed_ms = start.elapsed().as_millis();
    println!("[{remote}] {method} {path} -> {status} ({elapsed_ms}ms)");

    let mut response = tiny_http::Response::from_string(body)
        .with_status_code(tiny_http::StatusCode(status))
        .with_header(
            tiny_http::Header::from_bytes(&b"Server"[..], &b"DefLang Mock Server"[..]).unwrap(),
        );

    for (name, value) in &headers {
        if let Ok(header) = tiny_http::Header::from_bytes(name.as_bytes(), value.as_bytes()) {
            response = response.with_header(header);
        }
    }

    if let Err(e) = request.respond(response) {
        eprintln!("error sending response: {e}");
    }
}

// ── matching ──────────────────────────────────────────────────────────────────

fn find_mock<'a>(method: &str, path: &str, mocks: &'a [(String, MockValue)]) -> Option<&'a MockValue> {
    mocks
        .iter()
        .find(|(_, mock)| {
            mock.method.eq_ignore_ascii_case(method) && extract_path(&mock.url) == path
        })
        .map(|(_, mock)| mock)
}

/// Extracts the path component from a URL.
/// `https://api.example.com/users` → `/users`
/// `/users` → `/users`
fn extract_path(url: &str) -> &str {
    for prefix in &["https://", "http://"] {
        if let Some(rest) = url.strip_prefix(prefix) {
            return if let Some(pos) = rest.find('/') {
                &rest[pos..]
            } else {
                "/"
            };
        }
    }
    if url.is_empty() { "/" } else { url }
}

/// Strips the query string from a request path.
/// `/users?page=1` → `/users`
fn strip_query(path: &str) -> &str {
    match path.find('?') {
        Some(pos) => &path[..pos],
        None => path,
    }
}
