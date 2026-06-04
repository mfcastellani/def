use std::{
    fs,
    path::{Path, PathBuf},
    thread,
    time::Duration,
};

use regex::Regex;
use scraper::{Html, Selector};

use crate::error::{DefError, DefResult};
use crate::value::{BackoffStrategy, RequestValue, ResponseValue, Value};

pub(super) fn new_request_value(method: &str) -> Value {
    Value::Request(RequestValue {
        method: method.to_ascii_uppercase(),
        path: None,
        status: None,
        headers: Vec::new(),
        query_strings: Vec::new(),
        body: None,
        vars: Vec::new(),
        retries: 0,
        backoff: BackoffStrategy::None,
        timeout_ms: None,
        timeout_message: None,
        mocks: Vec::new(),
        snapshot: false,
        mock_with_snapshot: false,
    })
}

pub(super) enum RequestMethodResult {
    Request,
    Value(Value),
}

pub(super) fn apply_request_method(
    request: &mut RequestValue,
    name: &str,
    args: Vec<Value>,
    base_dir: &Path,
) -> DefResult<RequestMethodResult> {
    match name {
        "path" => {
            if args.len() != 1 {
                return Err(DefError::Runtime(format!(
                    "request.path expects 1 argument, got {}",
                    args.len()
                )));
            }

            let Value::String(path) = &args[0] else {
                return Err(DefError::Runtime(
                    "request.path expects a string URL".to_string(),
                ));
            };

            request.path = Some(path.clone());
            Ok(RequestMethodResult::Request)
        }
        "header" => {
            let (name, value) = request_header_from_args(args)?;
            set_request_header(&mut request.headers, &name, &value)?;
            Ok(RequestMethodResult::Request)
        }
        "headers_from" => {
            if args.len() != 1 {
                return Err(DefError::Runtime(format!(
                    "request.headers_from expects 1 argument, got {}",
                    args.len()
                )));
            }

            let Value::String(path) = &args[0] else {
                return Err(DefError::Runtime(
                    "request.headers_from expects a string path".to_string(),
                ));
            };

            for (name, value) in read_headers_file(base_dir, path)? {
                let value = render_template(&value, &request.vars);
                set_request_header(&mut request.headers, &name, &value)?;
            }

            Ok(RequestMethodResult::Request)
        }
        "query_string" => {
            let (name, value) = request_query_string_from_args(args)?;
            set_request_query_string(&mut request.query_strings, &name, &value)?;
            Ok(RequestMethodResult::Request)
        }
        "query_string_from" => {
            if args.len() != 1 {
                return Err(DefError::Runtime(format!(
                    "request.query_string_from expects 1 argument, got {}",
                    args.len()
                )));
            }

            let Value::String(path) = &args[0] else {
                return Err(DefError::Runtime(
                    "request.query_string_from expects a string path".to_string(),
                ));
            };

            for (name, value) in read_query_string_file(base_dir, path)? {
                let value = render_template(&value, &request.vars);
                set_request_query_string(&mut request.query_strings, &name, &value)?;
            }

            Ok(RequestMethodResult::Request)
        }
        "body_from" => {
            if args.len() != 1 {
                return Err(DefError::Runtime(format!(
                    "request.body_from expects 1 argument, got {}",
                    args.len()
                )));
            }

            let Value::String(path) = &args[0] else {
                return Err(DefError::Runtime(
                    "request.body_from expects a string path".to_string(),
                ));
            };

            let raw = read_body_file(base_dir, path)?;
            request.body = Some(render_template(&raw, &request.vars));

            let already_has_content_type = request
                .headers
                .iter()
                .any(|(k, _)| k.to_ascii_lowercase() == "content-type");

            if !already_has_content_type {
                let inferred = if path.ends_with(".jdef") {
                    Some("application/json")
                } else if path.ends_with(".tdef") {
                    Some("text/plain")
                } else {
                    None
                };
                if let Some(ct) = inferred {
                    set_request_header(&mut request.headers, "Content-Type", ct)?;
                }
            }

            Ok(RequestMethodResult::Request)
        }
        "form_from" => {
            if args.len() != 1 {
                return Err(DefError::Runtime(format!(
                    "request.form_from expects 1 argument, got {}",
                    args.len()
                )));
            }
            let Value::String(path) = &args[0] else {
                return Err(DefError::Runtime(
                    "request.form_from expects a string path".to_string(),
                ));
            };

            let fields = read_form_file(base_dir, path)?;
            let encoded: String = fields
                .iter()
                .map(|(k, v)| {
                    let k = render_template(k, &request.vars);
                    let v = render_template(v, &request.vars);
                    format!("{}={}", url_encode(&k), url_encode(&v))
                })
                .collect::<Vec<_>>()
                .join("&");

            request.body = Some(encoded);
            set_request_header(
                &mut request.headers,
                "Content-Type",
                "application/x-www-form-urlencoded",
            )?;

            Ok(RequestMethodResult::Request)
        }
        "type" => {
            if args.len() != 1 {
                return Err(DefError::Runtime(format!(
                    "request.type expects 1 argument (JSON or TEXT), got {}",
                    args.len()
                )));
            }

            let Value::String(body_type) = &args[0] else {
                return Err(DefError::Runtime(
                    "request.type expects JSON or TEXT".to_string(),
                ));
            };

            let content_type = match body_type.to_ascii_uppercase().as_str() {
                "JSON" => "application/json",
                "TEXT" => "text/plain",
                other => {
                    return Err(DefError::Runtime(format!(
                        "request.type expects JSON or TEXT, got '{other}'"
                    )))
                }
            };

            set_request_header(&mut request.headers, "Content-Type", content_type)?;
            Ok(RequestMethodResult::Request)
        }
        "with_var" => {
            let (name, value) = request_var_from_args(args)?;
            set_request_var(&mut request.vars, &name, &value)?;
            apply_request_vars_to_headers(&mut request.headers, &request.vars);
            apply_request_vars_to_query_strings(&mut request.query_strings, &request.vars);
            apply_request_vars_to_body(&mut request.body, &request.vars);
            Ok(RequestMethodResult::Request)
        }
        "retries" => {
            if args.len() != 1 {
                return Err(DefError::Runtime(format!(
                    "request.retries expects 1 argument, got {}",
                    args.len()
                )));
            }

            let Value::Integer(times) = args[0] else {
                return Err(DefError::Runtime(
                    "request.retries expects a non-negative integer".to_string(),
                ));
            };

            if times < 0 {
                return Err(DefError::Runtime(
                    "request.retries expects a non-negative integer".to_string(),
                ));
            }

            request.retries = times as u32;
            Ok(RequestMethodResult::Request)
        }
        "fixed_backoff" => {
            let ms = parse_backoff_ms("fixed_backoff", &args)?;
            request.backoff = BackoffStrategy::Fixed(ms);
            Ok(RequestMethodResult::Request)
        }
        "linear_backoff" => {
            let ms = parse_backoff_ms("linear_backoff", &args)?;
            request.backoff = BackoffStrategy::Linear(ms);
            Ok(RequestMethodResult::Request)
        }
        "exponential_backoff" => {
            let ms = parse_backoff_ms("exponential_backoff", &args)?;
            request.backoff = BackoffStrategy::Exponential(ms);
            Ok(RequestMethodResult::Request)
        }
        "timeout" => {
            match args.as_slice() {
                [Value::Integer(ms), Value::String(message)] => {
                    if *ms < 0 {
                        return Err(DefError::Runtime(
                            "request.timeout expects a non-negative integer".to_string(),
                        ));
                    }
                    request.timeout_ms = Some(*ms as u64);
                    request.timeout_message = Some(message.clone());
                    Ok(RequestMethodResult::Request)
                }
                [Value::Integer(ms)] => {
                    if *ms < 0 {
                        return Err(DefError::Runtime(
                            "request.timeout expects a non-negative integer".to_string(),
                        ));
                    }
                    request.timeout_ms = Some(*ms as u64);
                    Ok(RequestMethodResult::Request)
                }
                _ => Err(DefError::Runtime(
                    "request.timeout expects timeout(ms) or timeout(ms, \"message\")".to_string(),
                )),
            }
        }
        "inspect" => {
            if !args.is_empty() {
                return Err(DefError::Runtime(format!(
                    "request.inspect expects 0 arguments, got {}",
                    args.len()
                )));
            }
            let url = request.path.as_deref().unwrap_or("(not set)");
            println!("[inspect] {} {url}", request.method);
            if !request.headers.is_empty() {
                println!("  headers:");
                for (k, v) in &request.headers {
                    println!("    {k}: {v}");
                }
            }
            if !request.query_strings.is_empty() {
                println!("  query:");
                for (k, v) in &request.query_strings {
                    println!("    {k}: {v}");
                }
            }
            if let Some(ref body) = request.body {
                println!("  body:");
                for line in body.lines() {
                    println!("    {line}");
                }
            }
            if !request.vars.is_empty() {
                println!("  vars:");
                for (k, v) in &request.vars {
                    println!("    {k}: {v}");
                }
            }
            if request.retries > 0 {
                let backoff_str = match &request.backoff {
                    BackoffStrategy::None => "none".to_string(),
                    BackoffStrategy::Fixed(ms) => format!("fixed_backoff({ms}ms)"),
                    BackoffStrategy::Linear(ms) => format!("linear_backoff({ms}ms)"),
                    BackoffStrategy::Exponential(ms) => format!("exponential_backoff({ms}ms)"),
                };
                println!("  retries: {} ({backoff_str})", request.retries);
            }
            if let Some(ms) = request.timeout_ms {
                match &request.timeout_message {
                    Some(msg) => println!("  timeout: {ms}ms (\"{msg}\")"),
                    None => println!("  timeout: {ms}ms"),
                }
            }
            Ok(RequestMethodResult::Request)
        }
        "with_mocks" => {
            let mock_list = match args.as_slice() {
                [Value::Array(items)] => items.clone(),
                [Value::Mock(m)] => vec![Value::Mock(m.clone())],
                _ => return Err(DefError::Runtime("request.with_mocks expects an array of mocks or a single mock".to_string())),
            };
            for item in mock_list {
                match item {
                    Value::Mock(m) => request.mocks.push(m),
                    _ => return Err(DefError::Runtime("with_mocks array must contain only mock values".to_string())),
                }
            }
            Ok(RequestMethodResult::Request)
        }
        "snapshot" => {
            if !args.is_empty() {
                return Err(DefError::Runtime(format!(
                    "request.snapshot expects 0 arguments, got {}",
                    args.len()
                )));
            }
            request.snapshot = true;
            Ok(RequestMethodResult::Request)
        }
        "mock_with_snapshot" => {
            if !args.is_empty() {
                return Err(DefError::Runtime(format!(
                    "request.mock_with_snapshot expects 0 arguments, got {}",
                    args.len()
                )));
            }
            request.mock_with_snapshot = true;
            Ok(RequestMethodResult::Request)
        }
        "do" => {
            if !args.is_empty() {
                return Err(DefError::Runtime(format!(
                    "request.do expects 0 arguments, got {}",
                    args.len()
                )));
            }

            if request.mock_with_snapshot {
                let url = request.path.as_deref().unwrap_or("unknown").to_string();
                let base_name = snapshot_slug(&request.method, &url);
                let snapshots_dir = base_dir.join("snapshots");
                if let Some(response) = load_response_snapshot(&snapshots_dir, &base_name, &request.method, &url) {
                    return Ok(RequestMethodResult::Value(Value::Response(response)));
                }
                let response_value = execute_http_request(request)?;
                if let Value::Response(ref rv) = response_value {
                    save_snapshot(rv, &request.method, &url, base_dir);
                }
                return Ok(RequestMethodResult::Value(response_value));
            }

            let response = execute_http_request(request)?;
            if request.snapshot {
                if let Value::Response(ref rv) = response {
                    save_snapshot(rv, &request.method, request.path.as_deref().unwrap_or("unknown"), base_dir);
                }
            }
            Ok(RequestMethodResult::Value(response))
        }
        "status" => {
            if !args.is_empty() {
                return Err(DefError::Runtime(format!(
                    "request.status expects 0 arguments, got {}",
                    args.len()
                )));
            }

            request
                .status
                .map(Value::Integer)
                .map(RequestMethodResult::Value)
                .ok_or_else(|| DefError::Runtime("request has not been executed".to_string()))
        }
        _ => Err(DefError::Runtime(format!(
            "unknown request method '{name}'"
        ))),
    }
}

fn set_request_header(
    headers: &mut Vec<(String, String)>,
    name: &str,
    value: &str,
) -> DefResult<()> {
    let name = name.trim();
    if name.is_empty() {
        return Err(DefError::Runtime(
            "request header name cannot be empty".to_string(),
        ));
    }

    let normalized = name.to_ascii_lowercase();
    headers.retain(|(current, _)| current.to_ascii_lowercase() != normalized);
    headers.push((name.to_string(), value.to_string()));
    Ok(())
}

fn request_header_from_args(args: Vec<Value>) -> DefResult<(String, String)> {
    match args.as_slice() {
        [Value::Tuple { key, value }] => {
            let Value::String(value) = value.as_ref() else {
                return Err(DefError::Runtime(
                    "request.header tuple value must be a string".to_string(),
                ));
            };

            Ok((key.clone(), value.clone()))
        }
        [Value::String(name), Value::String(value)] => Ok((name.clone(), value.clone())),
        [_] => Err(DefError::Runtime(
            "request.header expects a tuple(\"Name\", \"value\") argument".to_string(),
        )),
        _ => Err(DefError::Runtime(format!(
            "request.header expects 1 tuple argument, got {}",
            args.len()
        ))),
    }
}

fn set_request_query_string(
    query_strings: &mut Vec<(String, String)>,
    name: &str,
    value: &str,
) -> DefResult<()> {
    let name = name.trim();
    if name.is_empty() {
        return Err(DefError::Runtime(
            "request query string name cannot be empty".to_string(),
        ));
    }

    query_strings.retain(|(current, _)| current != name);
    query_strings.push((name.to_string(), value.to_string()));
    Ok(())
}

fn request_query_string_from_args(args: Vec<Value>) -> DefResult<(String, String)> {
    match args.as_slice() {
        [Value::Tuple { key, value }] => {
            let Value::String(value) = value.as_ref() else {
                return Err(DefError::Runtime(
                    "request.query_string tuple value must be a string".to_string(),
                ));
            };

            Ok((key.clone(), value.clone()))
        }
        [_] => Err(DefError::Runtime(
            "request.query_string expects a tuple(\"name\", \"value\") argument".to_string(),
        )),
        _ => Err(DefError::Runtime(format!(
            "request.query_string expects 1 tuple argument, got {}",
            args.len()
        ))),
    }
}

fn request_var_from_args(args: Vec<Value>) -> DefResult<(String, String)> {
    match args.as_slice() {
        [Value::Tuple { key, value }] => {
            let string_value = primitive_to_string(value.as_ref())?;
            Ok((key.clone(), string_value))
        }
        _ => Err(DefError::Runtime(format!(
            "request.with_var expects 1 variable identifier, got {}",
            args.len()
        ))),
    }
}

fn primitive_to_string(value: &Value) -> DefResult<String> {
    match value {
        Value::String(s) => Ok(s.clone()),
        Value::Integer(n) => Ok(n.to_string()),
        Value::Float(f) => Ok(f.to_string()),
        Value::Boolean(b) => Ok(b.to_string()),
        _ => Err(DefError::Runtime(
            "request.with_var variable must be a string, integer, float, or boolean".to_string(),
        )),
    }
}

fn set_request_var(vars: &mut Vec<(String, String)>, name: &str, value: &str) -> DefResult<()> {
    let name = name.trim();
    if name.is_empty() {
        return Err(DefError::Runtime(
            "request.with_var variable name cannot be empty".to_string(),
        ));
    }

    vars.retain(|(current, _)| current != name);
    vars.push((name.to_string(), value.to_string()));
    Ok(())
}

fn apply_request_vars_to_headers(headers: &mut [(String, String)], vars: &[(String, String)]) {
    for (_, value) in headers {
        *value = render_template(value, vars);
    }
}

fn apply_request_vars_to_query_strings(
    query_strings: &mut [(String, String)],
    vars: &[(String, String)],
) {
    for (_, value) in query_strings {
        *value = render_template(value, vars);
    }
}

fn apply_request_vars_to_body(body: &mut Option<String>, vars: &[(String, String)]) {
    if let Some(body) = body {
        *body = render_template(body, vars);
    }
}

pub(super) fn render_template(value: &str, vars: &[(String, String)]) -> String {
    let mut rendered = value.to_string();
    for (name, replacement) in vars {
        rendered = rendered.replace(&format!("{{{{{name}}}}}"), replacement);
    }
    rendered
}

fn find_unresolved(text: &str) -> Option<String> {
    let open = text.find("{{")?;
    let rest = &text[open + 2..];
    let placeholder = if let Some(close) = rest.find("}}") {
        rest[..close].trim().to_string()
    } else {
        rest.to_string()
    };
    Some(placeholder)
}

fn check_unresolved_vars(request: &RequestValue) -> DefResult<()> {
    for (header_name, value) in &request.headers {
        if let Some(placeholder) = find_unresolved(value) {
            return Err(DefError::Runtime(format!(
                "header '{header_name}' contains unresolved template variable '{{{{{placeholder}}}}}' — register it with with_var({placeholder})"
            )));
        }
    }
    for (param_name, value) in &request.query_strings {
        if let Some(placeholder) = find_unresolved(value) {
            return Err(DefError::Runtime(format!(
                "query string '{param_name}' contains unresolved template variable '{{{{{placeholder}}}}}' — register it with with_var({placeholder})"
            )));
        }
    }
    if let Some(body) = &request.body {
        if let Some(placeholder) = find_unresolved(body) {
            return Err(DefError::Runtime(format!(
                "request body contains unresolved template variable '{{{{{placeholder}}}}}' — register it with with_var({placeholder})"
            )));
        }
    }
    Ok(())
}

pub(super) fn read_headers_file(base_dir: &Path, header_path: &str) -> DefResult<Vec<(String, String)>> {
    let path = resolve_path(base_dir, header_path);
    let source = fs::read_to_string(&path).map_err(|error| {
        DefError::Runtime(format!(
            "request.headers_from failed to read '{}': {error}",
            path.display()
        ))
    })?;

    parse_headers_source(&source, &path.display().to_string())
}

pub(super) fn read_body_file(base_dir: &Path, body_path: &str) -> DefResult<String> {
    let path = resolve_path(base_dir, body_path);
    fs::read_to_string(&path).map_err(|error| {
        DefError::Runtime(format!(
            "request.body_from failed to read '{}': {error}",
            path.display()
        ))
    })
}

fn read_query_string_file(
    base_dir: &Path,
    query_string_path: &str,
) -> DefResult<Vec<(String, String)>> {
    let path = resolve_path(base_dir, query_string_path);
    let source = fs::read_to_string(&path).map_err(|error| {
        DefError::Runtime(format!(
            "request.query_string_from failed to read '{}': {error}",
            path.display()
        ))
    })?;

    parse_query_string_source(&source, &path.display().to_string())
}

pub(super) fn parse_headers_source(source: &str, context: &str) -> DefResult<Vec<(String, String)>> {
    let mut headers = Vec::new();

    for (index, line) in source.lines().enumerate() {
        let line_number = index + 1;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with('#') {
            continue;
        }

        let Some((name, value)) = line.split_once(':') else {
            return Err(DefError::Runtime(format!(
                "invalid header line {line_number} in '{context}': expected 'Name: value'"
            )));
        };

        let name = name.trim();
        if name.is_empty() {
            return Err(DefError::Runtime(format!(
                "invalid header line {line_number} in '{context}': header name cannot be empty"
            )));
        }

        headers.push((name.to_string(), value.trim().to_string()));
    }

    Ok(headers)
}

fn parse_query_string_source(source: &str, context: &str) -> DefResult<Vec<(String, String)>> {
    let mut query_strings = Vec::new();

    for (index, line) in source.lines().enumerate() {
        let line_number = index + 1;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with('#') {
            continue;
        }

        let Some((name, value)) = line.split_once(':') else {
            return Err(DefError::Runtime(format!(
                "invalid query string line {line_number} in '{context}': expected 'Name: value'"
            )));
        };

        let name = name.trim();
        if name.is_empty() {
            return Err(DefError::Runtime(format!(
                "invalid query string line {line_number} in '{context}': query string name cannot be empty"
            )));
        }

        query_strings.push((name.to_string(), value.trim().to_string()));
    }

    Ok(query_strings)
}

fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            b' ' => out.push('+'),
            b => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn read_form_file(base_dir: &Path, form_path: &str) -> DefResult<Vec<(String, String)>> {
    let path = resolve_path(base_dir, form_path);
    let source = fs::read_to_string(&path).map_err(|e| {
        DefError::Runtime(format!(
            "request.form_from failed to read '{}': {e}",
            path.display()
        ))
    })?;
    parse_form_source(&source, &path.display().to_string())
}

fn parse_form_source(source: &str, context: &str) -> DefResult<Vec<(String, String)>> {
    let mut fields = Vec::new();
    for (index, line) in source.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with('#') {
            continue;
        }
        let Some((key, value)) = trimmed.split_once(':') else {
            return Err(DefError::Runtime(format!(
                "invalid form line {} in '{context}': expected 'key: value'",
                index + 1
            )));
        };
        let key = key.trim();
        if key.is_empty() {
            return Err(DefError::Runtime(format!(
                "invalid form line {} in '{context}': field name cannot be empty",
                index + 1
            )));
        }
        fields.push((key.to_string(), value.trim().to_string()));
    }
    Ok(fields)
}

pub(super) fn resolve_path(base_dir: &Path, path: &str) -> PathBuf {
    let path = Path::new(path);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base_dir.join(path)
    }
}

fn execute_http_request(request: &mut RequestValue) -> DefResult<Value> {
    let max_attempts = request.retries as usize + 1;
    let mut last_error = String::new();

    for attempt in 0..max_attempts {
        match execute_http_request_once(request) {
            Ok(value) => {
                if let Value::Response(ref r) = value {
                    request.status = Some(r.status);
                }
                return Ok(value);
            }
            Err(DefError::Request(msg)) => {
                last_error = msg;
                if attempt + 1 < max_attempts {
                    let delay_ms = backoff_delay(&request.backoff, attempt);
                    if delay_ms > 0 {
                        thread::sleep(Duration::from_millis(delay_ms));
                    }
                }
            }
            Err(e) => return Err(e),
        }
    }

    Err(DefError::Request(last_error))
}

fn execute_http_request_once(request: &RequestValue) -> DefResult<Value> {
    check_unresolved_vars(request)?;

    let path = request
        .path
        .as_ref()
        .ok_or_else(|| DefError::Runtime("request.path must be set before request.do".to_string()))?
        .clone();

    // Check mocks first
    for mock in &request.mocks {
        if mock.method.eq_ignore_ascii_case(&request.method) && mock.url == path {
            if !mock.configured {
                return Err(DefError::Runtime(format!(
                    "mock for {} {} has no reply configured — call .reply() or .fail()",
                    mock.method, mock.url
                )));
            }
            if mock.delay_ms > 0 {
                thread::sleep(Duration::from_millis(mock.delay_ms));
            }
            return Ok(Value::Response(ResponseValue {
                status: mock.status,
                body: mock.body.clone(),
                headers: mock.headers.clone(),
                duration_ms: mock.delay_ms as i64,
                method: request.method.clone(),
                url: path.clone(),
            }));
        }
    }

    let mut agent_builder = ureq::AgentBuilder::new();
    if let Some(ms) = request.timeout_ms {
        agent_builder = agent_builder.timeout(Duration::from_millis(ms));
    }
    let agent = agent_builder.build();

    let mut http_request = agent.request(&request.method, &path);
    for (name, value) in &request.query_strings {
        http_request = http_request.query(name, value);
    }
    for (name, value) in &request.headers {
        http_request = http_request.set(name, value);
    }

    let start = std::time::Instant::now();
    let result = match &request.body {
        Some(body) => http_request.send_string(body),
        None => http_request.call(),
    };
    let duration_ms = start.elapsed().as_millis() as i64;

    let response = match result {
        Ok(r) => r,
        Err(ureq::Error::Status(code, r)) => {
            // HTTP error responses (4xx/5xx) are valid responses — return them so scripts
            // can inspect res.status() and branch with if/else.
            let headers = r
                .headers_names()
                .into_iter()
                .filter_map(|name| {
                    r.header(&name)
                        .map(|value| (name.to_ascii_lowercase(), value.to_string()))
                })
                .collect();
            let body = r.into_string().unwrap_or_default();
            return Ok(Value::Response(ResponseValue {
                status: i64::from(code),
                body,
                headers,
                duration_ms,
                method: request.method.clone(),
                url: path.clone(),
            }));
        }
        Err(ureq::Error::Transport(transport)) => {
            let msg = if let Some(ref custom) = request.timeout_message {
                custom.clone()
            } else {
                format!("request failed: {transport}")
            };
            return Err(DefError::Request(msg));
        }
    };

    let status = i64::from(response.status());
    let headers = response
        .headers_names()
        .into_iter()
        .filter_map(|name| {
            response
                .header(&name)
                .map(|value| (name.to_ascii_lowercase(), value.to_string()))
        })
        .collect();

    let body = response
        .into_string()
        .map_err(|error| DefError::Request(format!("failed to read response body: {error}")))?;

    Ok(Value::Response(ResponseValue {
        status,
        body,
        headers,
        duration_ms,
        method: request.method.clone(),
        url: path.clone(),
    }))
}

fn backoff_delay(strategy: &BackoffStrategy, attempt: usize) -> u64 {
    match strategy {
        BackoffStrategy::None => 0,
        BackoffStrategy::Fixed(ms) => *ms,
        BackoffStrategy::Linear(ms) => *ms * (attempt as u64 + 1),
        BackoffStrategy::Exponential(ms) => *ms * (1u64 << attempt),
    }
}

fn parse_backoff_ms(method: &str, args: &[Value]) -> DefResult<u64> {
    if args.len() != 1 {
        return Err(DefError::Runtime(format!(
            "request.{method} expects 1 argument, got {}",
            args.len()
        )));
    }
    let Value::Integer(ms) = args[0] else {
        return Err(DefError::Runtime(format!(
            "request.{method} expects a non-negative integer in milliseconds"
        )));
    };
    if ms < 0 {
        return Err(DefError::Runtime(format!(
            "request.{method} expects a non-negative integer in milliseconds"
        )));
    }
    Ok(ms as u64)
}

// ── JSON path helpers ─────────────────────────────────────────────────────────

enum JsonPathSegment {
    Field(String),
    Index(usize),
}

fn parse_json_path(path: &str) -> Result<Vec<JsonPathSegment>, String> {
    if !path.starts_with('$') {
        return Err(format!("invalid json path '{path}'"));
    }
    let chars: Vec<char> = path[1..].chars().collect();
    let mut i = 0;
    let mut segments = Vec::new();
    while i < chars.len() {
        match chars[i] {
            '.' => {
                i += 1;
                if i >= chars.len() || chars[i] == '.' || chars[i] == '[' {
                    return Err(format!("invalid json path '{path}'"));
                }
                let mut field = String::new();
                while i < chars.len() && chars[i] != '.' && chars[i] != '[' {
                    let c = chars[i];
                    if !c.is_alphanumeric() && c != '_' && c != '-' {
                        return Err(format!("invalid json path '{path}'"));
                    }
                    field.push(c);
                    i += 1;
                }
                segments.push(JsonPathSegment::Field(field));
            }
            '[' => {
                i += 1;
                let mut digits = String::new();
                loop {
                    if i >= chars.len() {
                        return Err(format!("invalid json path '{path}'"));
                    }
                    match chars[i] {
                        ']' => { i += 1; break; }
                        c if c.is_ascii_digit() => { digits.push(c); i += 1; }
                        _ => return Err(format!("invalid json path '{path}'")),
                    }
                }
                if digits.is_empty() {
                    return Err(format!("invalid json path '{path}'"));
                }
                let idx = digits
                    .parse::<usize>()
                    .map_err(|_| format!("invalid json path '{path}'"))?;
                segments.push(JsonPathSegment::Index(idx));
            }
            _ => return Err(format!("invalid json path '{path}'")),
        }
    }
    Ok(segments)
}

fn navigate_json<'a>(
    root: &'a serde_json::Value,
    segments: &[JsonPathSegment],
) -> Option<&'a serde_json::Value> {
    let mut current = root;
    for seg in segments {
        current = match seg {
            JsonPathSegment::Field(name) => current.get(name.as_str())?,
            JsonPathSegment::Index(idx) => current.get(*idx)?,
        };
    }
    Some(current)
}

fn json_to_def_value(v: &serde_json::Value) -> Value {
    match v {
        serde_json::Value::Null => Value::Nil,
        serde_json::Value::Bool(b) => Value::Boolean(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Integer(i)
            } else if let Some(f) = n.as_f64() {
                Value::Float(f)
            } else {
                Value::String(n.to_string())
            }
        }
        serde_json::Value::String(s) => Value::String(s.clone()),
        other => Value::String(other.to_string()),
    }
}

// ── response methods ──────────────────────────────────────────────────────────

pub(super) fn call_response_method(
    response: ResponseValue,
    name: &str,
    args: Vec<Value>,
    base_dir: &Path,
) -> DefResult<Value> {
    match name {
        "body" => {
            if !args.is_empty() {
                return Err(DefError::Runtime(format!(
                    "response.body expects 0 arguments, got {}",
                    args.len()
                )));
            }

            Ok(Value::String(response.body))
        }
        "status" => {
            if !args.is_empty() {
                return Err(DefError::Runtime(format!(
                    "response.status expects 0 arguments, got {}",
                    args.len()
                )));
            }

            Ok(Value::Integer(response.status))
        }
        "headers" => {
            if !args.is_empty() {
                return Err(DefError::Runtime(format!(
                    "response.headers expects 0 arguments, got {}",
                    args.len()
                )));
            }

            Ok(Value::Array(
                response
                    .headers
                    .into_iter()
                    .map(|(name, value)| Value::Tuple {
                        key: name,
                        value: Box::new(Value::String(value)),
                    })
                    .collect(),
            ))
        }
        "header" => {
            if args.len() != 1 {
                return Err(DefError::Runtime(format!(
                    "response.header expects 1 argument, got {}",
                    args.len()
                )));
            }

            let Value::String(name) = &args[0] else {
                return Err(DefError::Runtime(
                    "response.header expects a string header name".to_string(),
                ));
            };

            let name = name.to_ascii_lowercase();
            Ok(Value::String(
                response
                    .headers
                    .into_iter()
                    .find_map(|(header_name, value)| (header_name == name).then_some(value))
                    .unwrap_or_default(),
            ))
        }
        "duration" => {
            if !args.is_empty() {
                return Err(DefError::Runtime(format!(
                    "response.duration expects 0 arguments, got {}",
                    args.len()
                )));
            }

            Ok(Value::Integer(response.duration_ms))
        }
        "ok" => {
            if !args.is_empty() {
                return Err(DefError::Runtime(format!(
                    "response.ok expects 0 arguments, got {}",
                    args.len()
                )));
            }

            Ok(Value::Boolean(
                response.status >= 200 && response.status < 300,
            ))
        }
        "size" => {
            if !args.is_empty() {
                return Err(DefError::Runtime(format!(
                    "response.size expects 0 arguments, got {}",
                    args.len()
                )));
            }

            Ok(Value::Integer(response.body.len() as i64))
        }
        "content_type" => {
            if !args.is_empty() {
                return Err(DefError::Runtime(format!(
                    "response.content_type expects 0 arguments, got {}",
                    args.len()
                )));
            }

            let ct = response
                .headers
                .iter()
                .find_map(|(name, value)| {
                    (name.to_ascii_lowercase() == "content-type").then(|| value.clone())
                })
                .unwrap_or_default();

            Ok(Value::String(ct))
        }
        "body_contains" => {
            if args.len() != 1 {
                return Err(DefError::Runtime(format!(
                    "response.body_contains expects 1 argument, got {}",
                    args.len()
                )));
            }

            let Value::String(search) = &args[0] else {
                return Err(DefError::Runtime(
                    "response.body_contains expects a string argument".to_string(),
                ));
            };

            Ok(Value::Boolean(response.body.contains(search.as_str())))
        }
        "describe_status" => {
            if !args.is_empty() {
                return Err(DefError::Runtime(format!(
                    "response.describe_status expects 0 arguments, got {}",
                    args.len()
                )));
            }

            let label = match response.status {
                100 => "100 Continue",
                101 => "101 Switching Protocols",
                200 => "200 OK",
                201 => "201 Created",
                202 => "202 Accepted",
                204 => "204 No Content",
                206 => "206 Partial Content",
                301 => "301 Moved Permanently",
                302 => "302 Found",
                304 => "304 Not Modified",
                307 => "307 Temporary Redirect",
                308 => "308 Permanent Redirect",
                400 => "400 Bad Request",
                401 => "401 Unauthorized",
                403 => "403 Forbidden",
                404 => "404 Not Found",
                405 => "405 Method Not Allowed",
                408 => "408 Request Timeout",
                409 => "409 Conflict",
                410 => "410 Gone",
                422 => "422 Unprocessable Entity",
                429 => "429 Too Many Requests",
                500 => "500 Internal Server Error",
                501 => "501 Not Implemented",
                502 => "502 Bad Gateway",
                503 => "503 Service Unavailable",
                504 => "504 Gateway Timeout",
                _   => "unknown",
            };

            Ok(Value::String(label.to_string()))
        }
        "inspect" => {
            if !args.is_empty() {
                return Err(DefError::Runtime(format!(
                    "response.inspect expects 0 arguments, got {}",
                    args.len()
                )));
            }
            let ok_label = if response.status >= 200 && response.status < 300 {
                "ok"
            } else {
                "error"
            };
            println!(
                "[inspect] {} ({ok_label}, {}ms)",
                response.status, response.duration_ms
            );
            if !response.headers.is_empty() {
                println!("  headers:");
                for (k, v) in &response.headers {
                    println!("    {k}: {v}");
                }
            }
            if !response.body.is_empty() {
                println!("  body:");
                for line in response.body.lines() {
                    println!("    {line}");
                }
            }
            Ok(Value::Response(response))
        }
        "json" => {
            if args.len() != 1 {
                return Err(DefError::Runtime(format!(
                    "response.json expects 1 argument, got {}",
                    args.len()
                )));
            }
            let Value::String(path) = &args[0] else {
                return Err(DefError::Runtime(
                    "response.json expects a string path argument".to_string(),
                ));
            };
            let root: serde_json::Value = serde_json::from_str(&response.body)
                .map_err(|_| DefError::Runtime("response body is not valid JSON".to_string()))?;
            let segments = parse_json_path(path).map_err(DefError::Runtime)?;
            let found = navigate_json(&root, &segments)
                .ok_or_else(|| DefError::Runtime(format!("json path '{path}' not found")))?;
            Ok(json_to_def_value(found))
        }
        "json_exists" => {
            if args.len() != 1 {
                return Err(DefError::Runtime(format!(
                    "response.json_exists expects 1 argument, got {}",
                    args.len()
                )));
            }
            let Value::String(path) = &args[0] else {
                return Err(DefError::Runtime(
                    "response.json_exists expects a string path argument".to_string(),
                ));
            };
            let root: serde_json::Value = serde_json::from_str(&response.body)
                .map_err(|_| DefError::Runtime("response body is not valid JSON".to_string()))?;
            let segments = parse_json_path(path).map_err(DefError::Runtime)?;
            Ok(Value::Boolean(navigate_json(&root, &segments).is_some()))
        }
        "body_matches" => {
            if args.len() != 1 {
                return Err(DefError::Runtime(format!(
                    "response.body_matches expects 1 argument, got {}",
                    args.len()
                )));
            }
            let Value::String(pattern) = &args[0] else {
                return Err(DefError::Runtime(
                    "response.body_matches expects a string regex pattern".to_string(),
                ));
            };
            let re = Regex::new(pattern).map_err(|e| {
                DefError::Runtime(format!("response.body_matches: invalid regex '{pattern}': {e}"))
            })?;
            Ok(Value::Boolean(re.is_match(&response.body)))
        }
        "html" => {
            if args.len() != 1 {
                return Err(DefError::Runtime(format!(
                    "response.html expects 1 argument, got {}",
                    args.len()
                )));
            }
            let Value::String(selector_str) = &args[0] else {
                return Err(DefError::Runtime(
                    "response.html expects a string CSS selector".to_string(),
                ));
            };
            let selector = Selector::parse(selector_str).map_err(|_| {
                DefError::Runtime(format!("response.html: invalid CSS selector '{selector_str}'"))
            })?;
            let document = Html::parse_document(&response.body);
            let text = document
                .select(&selector)
                .next()
                .map(|el| el.text().collect::<Vec<_>>().join("").trim().to_string())
                .unwrap_or_default();
            Ok(Value::String(text))
        }
        "html_all" => {
            if args.len() != 1 {
                return Err(DefError::Runtime(format!(
                    "response.html_all expects 1 argument, got {}",
                    args.len()
                )));
            }
            let Value::String(selector_str) = &args[0] else {
                return Err(DefError::Runtime(
                    "response.html_all expects a string CSS selector".to_string(),
                ));
            };
            let selector = Selector::parse(selector_str).map_err(|_| {
                DefError::Runtime(format!(
                    "response.html_all: invalid CSS selector '{selector_str}'"
                ))
            })?;
            let document = Html::parse_document(&response.body);
            let items = document
                .select(&selector)
                .map(|el| Value::String(el.text().collect::<Vec<_>>().join("").trim().to_string()))
                .collect();
            Ok(Value::Array(items))
        }
        "html_attr" => {
            if args.len() != 2 {
                return Err(DefError::Runtime(format!(
                    "response.html_attr expects 2 arguments, got {}",
                    args.len()
                )));
            }
            let (Value::String(selector_str), Value::String(attr_name)) = (&args[0], &args[1])
            else {
                return Err(DefError::Runtime(
                    "response.html_attr expects (css_selector, attribute_name) as strings"
                        .to_string(),
                ));
            };
            let selector = Selector::parse(selector_str).map_err(|_| {
                DefError::Runtime(format!(
                    "response.html_attr: invalid CSS selector '{selector_str}'"
                ))
            })?;
            let document = Html::parse_document(&response.body);
            let value = document
                .select(&selector)
                .next()
                .and_then(|el| el.value().attr(attr_name))
                .unwrap_or_default()
                .to_string();
            Ok(Value::String(value))
        }
        "assert_snapshot" => {
            if !args.is_empty() {
                return Err(DefError::Runtime(format!(
                    "response.assert_snapshot expects 0 arguments, got {}",
                    args.len()
                )));
            }

            let base_name = snapshot_slug(&response.method, &response.url);
            let snapshots_dir = base_dir.join("snapshots");

            let Some(snapshot) = load_response_snapshot(&snapshots_dir, &base_name, &response.method, &response.url) else {
                return Err(DefError::Runtime(format!(
                    "assert_snapshot: no snapshot found for {} {} — run with .snapshot() first",
                    response.method, response.url
                )));
            };

            if snapshot.status != response.status {
                return Err(DefError::Runtime(format!(
                    "assert_snapshot failed: status changed from {} to {}",
                    snapshot.status, response.status
                )));
            }

            let is_json = response.headers.iter().any(|(k, v)| {
                k == "content-type" && v.contains("application/json")
            });

            if is_json {
                let snap_json: serde_json::Value = serde_json::from_str(&snapshot.body)
                    .map_err(|_| DefError::Runtime("assert_snapshot: snapshot body is not valid JSON".to_string()))?;
                let actual_json: serde_json::Value = serde_json::from_str(&response.body)
                    .map_err(|_| DefError::Runtime("assert_snapshot: response body is not valid JSON".to_string()))?;

                let diffs = compare_json_structures(&snap_json, &actual_json, "$");
                if !diffs.is_empty() {
                    return Err(DefError::Runtime(format!(
                        "assert_snapshot failed: JSON structure changed\n{}",
                        diffs.join("\n")
                    )));
                }
            } else if snapshot.body.is_empty() != response.body.is_empty() {
                return Err(DefError::Runtime(format!(
                    "assert_snapshot failed: body presence changed (snapshot was {}, response is {})",
                    if snapshot.body.is_empty() { "empty" } else { "non-empty" },
                    if response.body.is_empty() { "empty" } else { "non-empty" },
                )));
            }

            Ok(Value::Response(response))
        }
        _ => Err(DefError::Runtime(format!(
            "unknown response method '{name}'"
        ))),
    }
}

fn compare_json_structures(
    snapshot: &serde_json::Value,
    actual: &serde_json::Value,
    path: &str,
) -> Vec<String> {
    use serde_json::Value as J;
    match (snapshot, actual) {
        (J::Null, J::Null) | (J::Bool(_), J::Bool(_)) | (J::Number(_), J::Number(_)) | (J::String(_), J::String(_)) => vec![],
        (J::Array(snap_arr), J::Array(act_arr)) => {
            if snap_arr.is_empty() && !act_arr.is_empty() {
                vec![format!("{path}: expected empty array, got array with {} element(s)", act_arr.len())]
            } else if !snap_arr.is_empty() && act_arr.is_empty() {
                vec![format!("{path}: expected non-empty array, got empty array")]
            } else if !snap_arr.is_empty() {
                compare_json_structures(&snap_arr[0], &act_arr[0], &format!("{path}[0]"))
            } else {
                vec![]
            }
        }
        (J::Object(snap_obj), J::Object(act_obj)) => {
            let mut diffs = vec![];
            for (key, snap_val) in snap_obj {
                match act_obj.get(key) {
                    None => diffs.push(format!("{path}: missing field \"{key}\"")),
                    Some(act_val) => diffs.extend(compare_json_structures(snap_val, act_val, &format!("{path}.{key}"))),
                }
            }
            for key in act_obj.keys() {
                if !snap_obj.contains_key(key) {
                    diffs.push(format!("{path}: unexpected field \"{key}\" not present in snapshot"));
                }
            }
            diffs
        }
        _ => vec![format!(
            "{path}: expected {}, got {}",
            json_type_name(snapshot),
            json_type_name(actual)
        )],
    }
}

fn json_type_name(v: &serde_json::Value) -> &'static str {
    match v {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

fn snapshot_slug(method: &str, url: &str) -> String {
    let method = method.to_lowercase();
    let url = url
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    let slug: String = url
        .chars()
        .map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect();
    let slug = slug
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    format!("{method}-{slug}")
}

fn snapshot_exists(snapshots_dir: &Path, base_name: &str) -> bool {
    let Ok(entries) = fs::read_dir(snapshots_dir) else {
        return false;
    };
    let prefix = format!("{base_name}-");
    entries.flatten().any(|e| {
        e.file_name()
            .to_string_lossy()
            .starts_with(prefix.as_str())
    })
}

fn save_snapshot(response: &ResponseValue, method: &str, url: &str, base_dir: &Path) {
    write_response_snapshot(response, &snapshot_slug(method, url), base_dir);
}

// ── response snapshot helpers (mock_with_snapshot) ────────────────────────────

fn load_response_snapshot(snapshots_dir: &Path, base_name: &str, method: &str, url: &str) -> Option<ResponseValue> {
    let Ok(entries) = fs::read_dir(snapshots_dir) else {
        return None;
    };
    let prefix = format!("{base_name}-");
    let sdef_entry = entries.flatten().find(|e| {
        let name = e.file_name().to_string_lossy().to_string();
        name.starts_with(prefix.as_str()) && name.ends_with(".sdef")
    })?;

    let file_stem = sdef_entry
        .file_name()
        .to_string_lossy()
        .trim_end_matches(".sdef")
        .to_string();

    let status_str = fs::read_to_string(sdef_entry.path()).ok()?;
    let status: i64 = status_str.trim().parse().ok()?;

    let hdef_path = snapshots_dir.join(format!("{file_stem}.hdef"));
    let headers = if hdef_path.exists() {
        fs::read_to_string(&hdef_path)
            .unwrap_or_default()
            .lines()
            .filter_map(|line| {
                line.split_once(": ").map(|(k, v)| (k.to_ascii_lowercase(), v.to_string()))
            })
            .collect()
    } else {
        Vec::new()
    };

    let jdef_path = snapshots_dir.join(format!("{file_stem}.jdef"));
    let tdef_path = snapshots_dir.join(format!("{file_stem}.tdef"));
    let body = if jdef_path.exists() {
        fs::read_to_string(&jdef_path).unwrap_or_default()
    } else if tdef_path.exists() {
        fs::read_to_string(&tdef_path).unwrap_or_default()
    } else {
        String::new()
    };

    Some(ResponseValue {
        status,
        body,
        headers,
        duration_ms: 0,
        method: method.to_string(),
        url: url.to_string(),
    })
}

fn write_response_snapshot(response: &ResponseValue, base_name: &str, base_dir: &Path) {
    let snapshots_dir = base_dir.join("snapshots");

    if snapshot_exists(&snapshots_dir, base_name) {
        return;
    }

    if let Err(e) = fs::create_dir_all(&snapshots_dir) {
        eprintln!("snapshot: failed to create snapshots directory: {e}");
        return;
    }

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let name = format!("{base_name}-{timestamp}");

    let _ = fs::write(snapshots_dir.join(format!("{name}.sdef")), response.status.to_string());

    if !response.headers.is_empty() {
        let content = response
            .headers
            .iter()
            .map(|(k, v)| format!("{k}: {v}"))
            .collect::<Vec<_>>()
            .join("\n");
        let _ = fs::write(snapshots_dir.join(format!("{name}.hdef")), content);
    }

    if !response.body.is_empty() {
        let is_json = response.headers.iter().any(|(k, v)| {
            k.to_ascii_lowercase() == "content-type" && v.contains("application/json")
        });
        let ext = if is_json { "jdef" } else { "tdef" };
        let _ = fs::write(snapshots_dir.join(format!("{name}.{ext}")), &response.body);
    }

    println!("snapshot: snapshots/{name}");
}
