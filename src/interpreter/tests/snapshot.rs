use super::*;

fn run_with_mock_snapshot(temp: &std::path::Path, script: &str) {
    let mut lexer = Lexer::new(script);
    let tokens = lexer.tokenize().unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    Interpreter::with_base_dir(temp).interpret(&program).unwrap();
}

#[allow(dead_code)]
fn run_with_mock_snapshot_error(temp: &std::path::Path, script: &str) -> DefError {
    let mut lexer = Lexer::new(script);
    let tokens = lexer.tokenize().unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    Interpreter::with_base_dir(temp).interpret(&program).unwrap_err()
}

#[test]
fn snapshot_saves_response_status_as_sdef() {
    let dir = temp_dir();
    let script = "\
def m as mock(POST, \"https://api.example.com/posts\").reply(201, \"id: 1\")\n\
def res as response(\n\
  request(POST)\n\
    .path(\"https://api.example.com/posts\")\n\
    .with_mocks(m)\n\
    .snapshot()\n\
    .do()\n\
)\n\
assert(res.status() == 201)";

    run_with_mock_snapshot(&dir, script);

    let snapshots = dir.join("snapshots");
    assert!(snapshots.exists(), "snapshots directory not created");

    let sdef_entry = fs::read_dir(&snapshots)
        .unwrap()
        .flatten()
        .find(|e| e.file_name().to_string_lossy().ends_with(".sdef"))
        .expect("no .sdef file saved");

    let status = fs::read_to_string(sdef_entry.path()).unwrap();
    assert_eq!(status.trim(), "201", "wrong status code in .sdef");
}

#[test]
fn snapshot_does_not_overwrite_existing() {
    let dir = temp_dir();
    let script = "\
def m as mock(GET, \"https://api.example.com/users\").reply(200, \"ok\")\n\
def res as response(\n\
  request(GET)\n\
    .path(\"https://api.example.com/users\")\n\
    .with_mocks(m)\n\
    .snapshot()\n\
    .do()\n\
)";

    run_with_mock_snapshot(&dir, script);

    let snapshots = dir.join("snapshots");
    let count_before = fs::read_dir(&snapshots).unwrap().count();

    run_with_mock_snapshot(&dir, script);
    let count_after = fs::read_dir(&snapshots).unwrap().count();

    assert_eq!(count_before, count_after, "snapshot was overwritten on second run");
}

#[test]
fn snapshot_saves_response_body_as_jdef_when_json_content_type() {
    let dir = temp_dir();
    let script = "\
def m as mock(POST, \"https://api.example.com/posts\")\n\
  .header(tuple(\"Content-Type\", \"application/json\"))\n\
  .reply(201, \"created\")\n\
def res as response(\n\
  request(POST)\n\
    .path(\"https://api.example.com/posts\")\n\
    .with_mocks(m)\n\
    .snapshot()\n\
    .do()\n\
)\n\
assert(res.status() == 201)";

    run_with_mock_snapshot(&dir, script);

    let snapshots = dir.join("snapshots");
    let jdef_exists = fs::read_dir(&snapshots)
        .unwrap()
        .flatten()
        .any(|e| e.file_name().to_string_lossy().ends_with(".jdef"));
    assert!(jdef_exists, "no .jdef file saved for JSON response body");
}

#[test]
fn snapshot_saves_response_headers_as_hdef() {
    let dir = temp_dir();
    let script = "\
def m as mock(GET, \"https://api.example.com/items\")\n\
  .header(tuple(\"X-Request-Id\", \"abc123\"))\n\
  .reply(200, \"ok\")\n\
def res as response(\n\
  request(GET)\n\
    .path(\"https://api.example.com/items\")\n\
    .with_mocks(m)\n\
    .snapshot()\n\
    .do()\n\
)\n\
assert(res.ok())";

    run_with_mock_snapshot(&dir, script);

    let snapshots = dir.join("snapshots");
    let hdef_exists = fs::read_dir(&snapshots)
        .unwrap()
        .flatten()
        .any(|e| e.file_name().to_string_lossy().ends_with(".hdef"));
    assert!(hdef_exists, "no .hdef file saved for response headers");
}

#[test]
fn snapshot_slug_uses_method_and_url() {
    let dir = temp_dir();
    let script = "\
def m as mock(GET, \"https://httpbingo.org/anything\").reply(200, \"ok\")\n\
def res as response(\n\
  request(GET)\n\
    .path(\"https://httpbingo.org/anything\")\n\
    .with_mocks(m)\n\
    .snapshot()\n\
    .do()\n\
)";

    run_with_mock_snapshot(&dir, script);

    let snapshots = dir.join("snapshots");
    let found = fs::read_dir(&snapshots)
        .unwrap()
        .flatten()
        .any(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with("get-httpbingo-org-anything-")
        });
    assert!(found, "snapshot file does not have expected slug prefix");
}

#[test]
fn mock_with_snapshot_saves_rdef_on_first_run() {
    let dir = temp_dir();
    let script = "\
def m as mock(GET, \"https://api.example.com/data\").reply(200, \"hello\")\n\
def res as response(\n\
  request(GET)\n\
    .path(\"https://api.example.com/data\")\n\
    .with_mocks(m)\n\
    .mock_with_snapshot()\n\
    .do()\n\
)\n\
assert(res.status() == 200)\n\
assert(res.body() == \"hello\")";

    run_with_mock_snapshot(&dir, script);

    let snapshots = dir.join("snapshots");
    assert!(snapshots.exists(), "snapshots directory not created");

    let sdef_exists = fs::read_dir(&snapshots)
        .unwrap()
        .flatten()
        .any(|e| e.file_name().to_string_lossy().ends_with(".sdef"));
    assert!(sdef_exists, "no .sdef file saved on first run");
}

#[test]
fn mock_with_snapshot_replays_from_rdef_on_second_run() {
    let dir = temp_dir();
    let script_first = "\
def m as mock(GET, \"https://api.example.com/data\").reply(200, \"from-mock\")\n\
def res as response(\n\
  request(GET)\n\
    .path(\"https://api.example.com/data\")\n\
    .with_mocks(m)\n\
    .mock_with_snapshot()\n\
    .do()\n\
)\n\
assert(res.status() == 200)";

    // First run: saves the snapshot
    run_with_mock_snapshot(&dir, script_first);

    // Second run: no mock configured — must replay from snapshot
    let script_second = "\
def res as response(\n\
  request(GET)\n\
    .path(\"https://api.example.com/data\")\n\
    .mock_with_snapshot()\n\
    .do()\n\
)\n\
assert(res.status() == 200)\n\
assert(res.body() == \"from-mock\")";

    run_with_mock_snapshot(&dir, script_second);
}

#[test]
fn mock_with_snapshot_does_not_overwrite_existing_rdef() {
    let dir = temp_dir();
    let script = "\
def m as mock(GET, \"https://api.example.com/count\").reply(200, \"1\")\n\
def res as response(\n\
  request(GET)\n\
    .path(\"https://api.example.com/count\")\n\
    .with_mocks(m)\n\
    .mock_with_snapshot()\n\
    .do()\n\
)";

    run_with_mock_snapshot(&dir, script);

    let snapshots = dir.join("snapshots");
    let count_before = fs::read_dir(&snapshots).unwrap().count();

    run_with_mock_snapshot(&dir, script);
    let count_after = fs::read_dir(&snapshots).unwrap().count();

    assert_eq!(count_before, count_after, "response snapshot was overwritten on second run");
}

#[test]
fn mock_with_snapshot_rdef_slug_uses_method_and_url() {
    let dir = temp_dir();
    let script = "\
def m as mock(POST, \"https://api.example.com/items\").reply(201, \"created\")\n\
def res as response(\n\
  request(POST)\n\
    .path(\"https://api.example.com/items\")\n\
    .with_mocks(m)\n\
    .mock_with_snapshot()\n\
    .do()\n\
)";

    run_with_mock_snapshot(&dir, script);

    let snapshots = dir.join("snapshots");
    let found = fs::read_dir(&snapshots)
        .unwrap()
        .flatten()
        .any(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with("post-api-example-com-items-")
        });
    assert!(found, "response snapshot does not have expected slug prefix");
}

#[test]
fn assert_snapshot_passes_when_json_structure_matches() {
    let dir = temp_dir();
    fs::write(dir.join("user_v1.json"), r#"{"id":1,"active":true}"#).unwrap();
    fs::write(dir.join("user_v2.json"), r#"{"id":99,"active":false}"#).unwrap();

    let script_save = "\
def m as mock(GET, \"https://api.example.com/users/1\")\n\
  .header(tuple(\"Content-Type\", \"application/json\"))\n\
  .body_from(\"user_v1.json\")\n\
  .reply(200)\n\
def res as response(\n\
  request(GET)\n\
    .path(\"https://api.example.com/users/1\")\n\
    .with_mocks(m)\n\
    .snapshot()\n\
    .do()\n\
)";
    run_with_mock_snapshot(&dir, script_save);

    let script_assert = "\
def m as mock(GET, \"https://api.example.com/users/1\")\n\
  .header(tuple(\"Content-Type\", \"application/json\"))\n\
  .body_from(\"user_v2.json\")\n\
  .reply(200)\n\
def res as response(\n\
  request(GET)\n\
    .path(\"https://api.example.com/users/1\")\n\
    .with_mocks(m)\n\
    .do()\n\
)\n\
res.assert_snapshot()";
    run_with_mock_snapshot(&dir, script_assert);
}

#[test]
fn assert_snapshot_fails_when_json_field_type_changes() {
    let dir = temp_dir();
    fs::write(dir.join("item_v1.json"), r#"{"id":1}"#).unwrap();
    fs::write(dir.join("item_v2.json"), r#"{"id":true}"#).unwrap();

    let script_save = "\
def m as mock(GET, \"https://api.example.com/items/1\")\n\
  .header(tuple(\"Content-Type\", \"application/json\"))\n\
  .body_from(\"item_v1.json\")\n\
  .reply(200)\n\
def res as response(\n\
  request(GET)\n\
    .path(\"https://api.example.com/items/1\")\n\
    .with_mocks(m)\n\
    .snapshot()\n\
    .do()\n\
)";
    run_with_mock_snapshot(&dir, script_save);

    let script_assert = "\
def m as mock(GET, \"https://api.example.com/items/1\")\n\
  .header(tuple(\"Content-Type\", \"application/json\"))\n\
  .body_from(\"item_v2.json\")\n\
  .reply(200)\n\
def res as response(\n\
  request(GET)\n\
    .path(\"https://api.example.com/items/1\")\n\
    .with_mocks(m)\n\
    .do()\n\
)\n\
res.assert_snapshot()";
    let error = run_with_mock_snapshot_error(&dir, script_assert);
    assert!(
        matches!(&error, DefError::Runtime(msg) if msg.contains("$.id") && msg.contains("number") && msg.contains("boolean")),
        "unexpected error: {error:?}"
    );
}

#[test]
fn assert_snapshot_fails_when_json_field_missing() {
    let dir = temp_dir();
    fs::write(dir.join("product_v1.json"), r#"{"id":1,"active":true}"#).unwrap();
    fs::write(dir.join("product_v2.json"), r#"{"id":2}"#).unwrap();

    let script_save = "\
def m as mock(GET, \"https://api.example.com/products/1\")\n\
  .header(tuple(\"Content-Type\", \"application/json\"))\n\
  .body_from(\"product_v1.json\")\n\
  .reply(200)\n\
def res as response(\n\
  request(GET)\n\
    .path(\"https://api.example.com/products/1\")\n\
    .with_mocks(m)\n\
    .snapshot()\n\
    .do()\n\
)";
    run_with_mock_snapshot(&dir, script_save);

    let script_assert = "\
def m as mock(GET, \"https://api.example.com/products/1\")\n\
  .header(tuple(\"Content-Type\", \"application/json\"))\n\
  .body_from(\"product_v2.json\")\n\
  .reply(200)\n\
def res as response(\n\
  request(GET)\n\
    .path(\"https://api.example.com/products/1\")\n\
    .with_mocks(m)\n\
    .do()\n\
)\n\
res.assert_snapshot()";
    let error = run_with_mock_snapshot_error(&dir, script_assert);
    assert!(
        matches!(&error, DefError::Runtime(msg) if msg.contains("missing field") && msg.contains("active")),
        "unexpected error: {error:?}"
    );
}

#[test]
fn assert_snapshot_fails_when_status_changes() {
    let dir = temp_dir();
    let script_save = "\
def m as mock(GET, \"https://api.example.com/check\")\n\
  .reply(200, \"ok\")\n\
def res as response(\n\
  request(GET)\n\
    .path(\"https://api.example.com/check\")\n\
    .with_mocks(m)\n\
    .snapshot()\n\
    .do()\n\
)";
    run_with_mock_snapshot(&dir, script_save);

    let script_assert = "\
def m as mock(GET, \"https://api.example.com/check\")\n\
  .reply(503, \"down\")\n\
def res as response(\n\
  request(GET)\n\
    .path(\"https://api.example.com/check\")\n\
    .with_mocks(m)\n\
    .do()\n\
)\n\
res.assert_snapshot()";
    let error = run_with_mock_snapshot_error(&dir, script_assert);
    assert!(
        matches!(&error, DefError::Runtime(msg) if msg.contains("status changed") && msg.contains("200") && msg.contains("503")),
        "unexpected error: {error:?}"
    );
}

#[test]
fn assert_snapshot_errors_when_no_snapshot_exists() {
    let dir = temp_dir();
    let script = "\
def m as mock(GET, \"https://api.example.com/new\")\n\
  .reply(200, \"ok\")\n\
def res as response(\n\
  request(GET)\n\
    .path(\"https://api.example.com/new\")\n\
    .with_mocks(m)\n\
    .do()\n\
)\n\
res.assert_snapshot()";
    let error = run_with_mock_snapshot_error(&dir, script);
    assert!(
        matches!(&error, DefError::Runtime(msg) if msg.contains("no snapshot found")),
        "unexpected error: {error:?}"
    );
}
