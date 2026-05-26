use super::*;

#[test]
fn request_can_be_configured_with_path() {
    let value = run("def r as request(GET)\n\
             r.path(\"https://example.com\")\n\
             assert(true == true)");
    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn request_path_can_be_chained() {
    let value = run("def r as request(GET)\n\
             r.path(\"https://example.com\").path(\"https://example.org\")\n\
             assert(true == true)");
    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn request_header_adds_header() {
    let interpreter = interpreter_after(
        "def r as request(GET)\n\
             r.header(tuple(\"Accept\", \"application/json\"))",
        ".",
    );
    let headers = request_headers(&interpreter, "r");
    assert_eq!(
        header_value(&headers, "Accept"),
        Some("application/json".to_string())
    );
}

#[test]
fn request_headers_can_be_chained() {
    let interpreter = interpreter_after(
        "def r as request(GET)\n\
             r.header(tuple(\"Accept\", \"application/json\"))\n\
              .header(tuple(\"Authorization\", \"Bearer token\"))",
        ".",
    );
    let headers = request_headers(&interpreter, "r");
    assert_eq!(
        header_value(&headers, "Accept"),
        Some("application/json".to_string())
    );
    assert_eq!(
        header_value(&headers, "Authorization"),
        Some("Bearer token".to_string())
    );
}

#[test]
fn request_headers_from_loads_file_relative_to_base_dir() {
    let dir = temp_dir();
    fs::write(
        dir.join("headers.hdef"),
        "Authorization: Bearer token\nContent-Type: application/json\n",
    )
    .unwrap();
    let interpreter = interpreter_after(
        "def r as request(GET)\n\
             r.headers_from(\"headers.hdef\")",
        &dir,
    );
    let headers = request_headers(&interpreter, "r");
    assert_eq!(
        header_value(&headers, "Authorization"),
        Some("Bearer token".to_string())
    );
    assert_eq!(
        header_value(&headers, "Content-Type"),
        Some("application/json".to_string())
    );
}

#[test]
fn request_headers_from_ignores_comments_and_empty_lines() {
    let dir = temp_dir();
    fs::write(
        dir.join("headers.hdef"),
        "\n// headers for tests\n# another comment\nAccept: application/json\n",
    )
    .unwrap();
    let interpreter = interpreter_after(
        "def r as request(GET)\n\
             r.headers_from(\"headers.hdef\")",
        &dir,
    );
    let headers = request_headers(&interpreter, "r");
    assert_eq!(headers.len(), 1);
    assert_eq!(
        header_value(&headers, "Accept"),
        Some("application/json".to_string())
    );
}

#[test]
fn request_headers_from_accepts_colons_in_value() {
    let dir = temp_dir();
    fs::write(dir.join("headers.hdef"), "Authorization: Bearer abc:123\n").unwrap();
    let interpreter = interpreter_after(
        "def r as request(GET)\n\
             r.headers_from(\"headers.hdef\")",
        &dir,
    );
    let headers = request_headers(&interpreter, "r");
    assert_eq!(
        header_value(&headers, "Authorization"),
        Some("Bearer abc:123".to_string())
    );
}

#[test]
fn request_repeated_header_uses_last_value() {
    let dir = temp_dir();
    fs::write(
        dir.join("headers.hdef"),
        "Accept: text/plain\nAccept: application/json\n",
    )
    .unwrap();
    let interpreter = interpreter_after(
        "def r as request(GET)\n\
             r.headers_from(\"headers.hdef\")",
        &dir,
    );
    let headers = request_headers(&interpreter, "r");
    assert_eq!(headers.len(), 1);
    assert_eq!(
        header_value(&headers, "Accept"),
        Some("application/json".to_string())
    );
}

#[test]
fn request_header_after_headers_from_overrides_file_value() {
    let dir = temp_dir();
    fs::write(dir.join("headers.hdef"), "Authorization: Bearer file\n").unwrap();
    let interpreter = interpreter_after(
        "def r as request(GET)\n\
             r.headers_from(\"headers.hdef\")\n\
              .header(tuple(\"Authorization\", \"Bearer override\"))",
        &dir,
    );
    let headers = request_headers(&interpreter, "r");
    assert_eq!(
        header_value(&headers, "Authorization"),
        Some("Bearer override".to_string())
    );
}

#[test]
fn request_headers_from_interpolates_with_var_called_after_file_load() {
    let dir = temp_dir();
    fs::write(dir.join("headers.hdef"), "Accept: {{accept_header}}\n").unwrap();
    let interpreter = interpreter_after(
        "def accept_header as string(\"application/json\")\n\
             def r as request(GET)\n\
             r.headers_from(\"headers.hdef\")\n\
              .with_var(accept_header)",
        &dir,
    );
    let headers = request_headers(&interpreter, "r");
    assert_eq!(
        header_value(&headers, "Accept"),
        Some("application/json".to_string())
    );
}

#[test]
fn request_headers_from_interpolates_with_var_called_before_file_load() {
    let dir = temp_dir();
    fs::write(dir.join("headers.hdef"), "Accept: {{accept_header}}\n").unwrap();
    let interpreter = interpreter_after(
        "def accept_header as string(\"application/json\")\n\
             def r as request(GET)\n\
             r.with_var(accept_header)\n\
              .headers_from(\"headers.hdef\")",
        &dir,
    );
    let headers = request_headers(&interpreter, "r");
    assert_eq!(
        header_value(&headers, "Accept"),
        Some("application/json".to_string())
    );
}

#[test]
fn request_with_var_accepts_primitive_types() {
    run(
        "def r as request(GET)\n\
             def page as integer(2)\n\
             def limit as float(10.5)\n\
             def active as boolean(true)\n\
             r.with_var(page)\n\
             r.with_var(limit)\n\
             r.with_var(active)",
    );
}

#[test]
fn request_with_var_rejects_non_primitive_value() {
    let error = interpret_error(
        "def items as array\n\
             def r as request(GET)\n\
             r.with_var(items)",
        ".",
    );
    assert!(
        matches!(error, DefError::Runtime(message) if message.contains("must be a string, integer, float, or boolean"))
    );
}

#[test]
fn request_do_fails_on_unresolved_header_template() {
    let dir = temp_dir();
    fs::write(dir.join("headers.hdef"), "Accept: {{accept}}\n").unwrap();
    let error = interpret_error(
        "def r as request(GET)\n\
         r.path(\"http://127.0.0.1:1\")\n\
         r.headers_from(\"headers.hdef\")\n\
         r.do()",
        dir.to_str().unwrap(),
    );
    assert!(
        matches!(&error, DefError::Runtime(msg) if msg.contains("unresolved template variable") && msg.contains("accept")),
        "unexpected error: {error:?}"
    );
}

#[test]
fn request_do_fails_on_unresolved_body_template() {
    let dir = temp_dir();
    fs::write(dir.join("body.jdef"), "{\"name\": \"{{username}}\"}\n").unwrap();
    let error = interpret_error(
        "def r as request(POST)\n\
         r.path(\"http://127.0.0.1:1\")\n\
         r.body_from(\"body.jdef\")\n\
         r.type(JSON)\n\
         r.do()",
        dir.to_str().unwrap(),
    );
    assert!(
        matches!(&error, DefError::Runtime(msg) if msg.contains("unresolved template variable") && msg.contains("username")),
        "unexpected error: {error:?}"
    );
}

#[test]
fn request_header_rejects_wrong_argument_count() {
    let error = interpret_error(
        "def r as request(GET)\nr.header(tuple(\"Accept\", \"application/json\"), tuple(\"X\", \"Y\"))",
        ".",
    );
    assert!(
        matches!(error, DefError::Runtime(message) if message.contains("request.header expects 1 tuple argument"))
    );
}

#[test]
fn request_header_rejects_non_string_arguments() {
    let error = interpret_error(
        "def r as request(GET)\nr.header(tuple(\"Accept\", 10))",
        ".",
    );
    assert!(
        matches!(error, DefError::Runtime(message) if message.contains("tuple value must be a string"))
    );
}

#[test]
fn request_headers_from_rejects_missing_file() {
    let dir = temp_dir();
    let error = interpret_error(
        "def r as request(GET)\nr.headers_from(\"missing.hdef\")",
        &dir,
    );
    assert!(
        matches!(error, DefError::Runtime(message) if message.contains("headers_from") && message.contains("missing.hdef"))
    );
}

#[test]
fn request_headers_from_rejects_invalid_line_without_colon() {
    let dir = temp_dir();
    fs::write(
        dir.join("headers.hdef"),
        "Accept: application/json\ninvalid\n",
    )
    .unwrap();
    let error = interpret_error(
        "def r as request(GET)\nr.headers_from(\"headers.hdef\")",
        &dir,
    );
    assert!(
        matches!(error, DefError::Runtime(message) if message.contains("line 2") && message.contains("expected 'Name: value'"))
    );
}

#[test]
fn request_headers_from_rejects_empty_header_name() {
    let dir = temp_dir();
    fs::write(dir.join("headers.hdef"), ": value\n").unwrap();
    let error = interpret_error(
        "def r as request(GET)\nr.headers_from(\"headers.hdef\")",
        &dir,
    );
    assert!(
        matches!(error, DefError::Runtime(message) if message.contains("line 1") && message.contains("header name cannot be empty"))
    );
}

#[test]
fn request_status_requires_execution() {
    let mut lexer = Lexer::new("def r as request(GET)\nr.status()");
    let tokens = lexer.tokenize().unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let error = Interpreter::new().interpret(&program).unwrap_err();
    assert!(
        matches!(error, DefError::Runtime(message) if message.contains("has not been executed"))
    );
}
