use super::*;

#[test]
fn mock_reply_returns_configured_response() {
    let value = run(
        "def m as mock(GET, \"https://api.example.com/users\").reply(200, \"name: Marcelo\")\n\
         def res as response(\n\
           request(GET)\n\
             .path(\"https://api.example.com/users\")\n\
             .with_mocks(m)\n\
             .do()\n\
         )\n\
         assert(res.ok())\n\
         assert(res.status() == 200)\n\
         assert(res.body_contains(\"Marcelo\"))",
    );
    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn mock_fail_returns_error_response() {
    let value = run(
        "def m as mock(POST, \"https://api.example.com/users\").fail(409, \"error: conflict\")\n\
         def res as response(\n\
           request(POST)\n\
             .path(\"https://api.example.com/users\")\n\
             .with_mocks(m)\n\
             .do()\n\
         )\n\
         assert(res.status() == 409)\n\
         assert(res.body_contains(\"conflict\"))",
    );
    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn mock_unmatched_url_does_not_intercept() {
    let mut lexer = Lexer::new(
        "def m as mock(GET, \"https://api.example.com/other\").reply(200, \"ok\")\n\
         def res as response(\n\
           request(GET)\n\
             .path(\"http://127.0.0.1:1/users\")\n\
             .with_mocks(m)\n\
             .do()\n\
         )",
    );
    let tokens = lexer.tokenize().unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let error = Interpreter::new().interpret(&program).unwrap_err();
    assert!(matches!(error, DefError::Request(_)));
}

#[test]
fn mock_without_reply_errors() {
    let error = interpret_error(
        "def m as mock(GET, \"https://api.example.com/users\")\n\
         def res as response(\n\
           request(GET)\n\
             .path(\"https://api.example.com/users\")\n\
             .with_mocks(m)\n\
             .do()\n\
         )",
        ".",
    );
    assert!(
        matches!(&error, DefError::Runtime(msg) if msg.contains("no reply configured")),
        "unexpected error: {error:?}"
    );
}

#[test]
fn mock_delay_sets_delay_ms() {
    let interpreter = interpreter_after(
        "def m as mock(GET, \"https://api.example.com\").delay(100).reply(200, \"ok\")",
        ".",
    );
    match interpreter.variables.get("m") {
        Some(Value::Mock(MockValue { delay_ms, configured, status, .. })) => {
            assert_eq!(*delay_ms, 100);
            assert!(*configured);
            assert_eq!(*status, 200);
        }
        other => panic!("expected mock value, got {other:?}"),
    }
}

#[test]
fn mock_inline_in_with_mocks() {
    let value = run(
        "def res as response(\n\
           request(GET)\n\
             .path(\"https://api.example.com/health\")\n\
             .with_mocks(mock(GET, \"https://api.example.com/health\").reply(200, \"ok\"))\n\
             .do()\n\
         )\n\
         assert(res.ok())\n\
         assert(res.status() == 200)",
    );
    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn mock_method_is_case_insensitive() {
    let value = run(
        "def m as mock(get, \"https://api.example.com/users\").reply(200, \"ok\")\n\
         def res as response(\n\
           request(GET)\n\
             .path(\"https://api.example.com/users\")\n\
             .with_mocks(m)\n\
             .do()\n\
         )\n\
         assert(res.ok())",
    );
    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn mock_array_with_multiple_mocks() {
    let value = run(
        "def m1 as mock(GET, \"https://api.example.com/a\").reply(200, \"a\")\n\
         def m2 as mock(POST, \"https://api.example.com/b\").reply(201, \"b\")\n\
         def mocks as array(m1, m2)\n\
         def r1 as response(\n\
           request(GET).path(\"https://api.example.com/a\").with_mocks(mocks).do()\n\
         )\n\
         def r2 as response(\n\
           request(POST).path(\"https://api.example.com/b\").with_mocks(mocks).do()\n\
         )\n\
         assert(r1.status() == 200)\n\
         assert(r1.body_contains(\"a\"))\n\
         assert(r2.status() == 201)\n\
         assert(r2.body_contains(\"b\"))",
    );
    assert_eq!(value, Value::Boolean(true));
}
