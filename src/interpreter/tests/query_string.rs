use super::*;

#[test]
fn request_query_string_adds_query_string() {
    let interpreter = interpreter_after(
        "def r as request(GET)\n\
             r.query_string(tuple(\"search\", \"def language\"))",
        ".",
    );
    let query_strings = request_query_strings(&interpreter, "r");
    assert_eq!(
        query_string_value(&query_strings, "search"),
        Some("def language".to_string())
    );
}

#[test]
fn request_query_strings_can_be_chained() {
    let interpreter = interpreter_after(
        "def r as request(GET)\n\
             r.query_string(tuple(\"search\", \"def\"))\n\
              .query_string(tuple(\"page\", \"1\"))",
        ".",
    );
    let query_strings = request_query_strings(&interpreter, "r");
    assert_eq!(
        query_string_value(&query_strings, "search"),
        Some("def".to_string())
    );
    assert_eq!(
        query_string_value(&query_strings, "page"),
        Some("1".to_string())
    );
}

#[test]
fn request_query_string_from_loads_file_relative_to_base_dir() {
    let dir = temp_dir();
    fs::write(dir.join("query.qdef"), "search: def\npage: 1\n").unwrap();
    let interpreter = interpreter_after(
        "def r as request(GET)\n\
             r.query_string_from(\"query.qdef\")",
        &dir,
    );
    let query_strings = request_query_strings(&interpreter, "r");
    assert_eq!(
        query_string_value(&query_strings, "search"),
        Some("def".to_string())
    );
    assert_eq!(
        query_string_value(&query_strings, "page"),
        Some("1".to_string())
    );
}

#[test]
fn request_query_string_from_ignores_comments_and_empty_lines() {
    let dir = temp_dir();
    fs::write(
        dir.join("query.qdef"),
        "\n// query params for tests\n# another comment\nsearch: def\n",
    )
    .unwrap();
    let interpreter = interpreter_after(
        "def r as request(GET)\n\
             r.query_string_from(\"query.qdef\")",
        &dir,
    );
    let query_strings = request_query_strings(&interpreter, "r");
    assert_eq!(query_strings.len(), 1);
    assert_eq!(
        query_string_value(&query_strings, "search"),
        Some("def".to_string())
    );
}

#[test]
fn request_query_string_from_accepts_colons_in_value() {
    let dir = temp_dir();
    fs::write(dir.join("query.qdef"), "token: abc:123\n").unwrap();
    let interpreter = interpreter_after(
        "def r as request(GET)\n\
             r.query_string_from(\"query.qdef\")",
        &dir,
    );
    let query_strings = request_query_strings(&interpreter, "r");
    assert_eq!(
        query_string_value(&query_strings, "token"),
        Some("abc:123".to_string())
    );
}

#[test]
fn request_repeated_query_string_uses_last_value() {
    let dir = temp_dir();
    fs::write(dir.join("query.qdef"), "search: old\nsearch: new\n").unwrap();
    let interpreter = interpreter_after(
        "def r as request(GET)\n\
             r.query_string_from(\"query.qdef\")",
        &dir,
    );
    let query_strings = request_query_strings(&interpreter, "r");
    assert_eq!(query_strings.len(), 1);
    assert_eq!(
        query_string_value(&query_strings, "search"),
        Some("new".to_string())
    );
}

#[test]
fn request_query_string_after_query_string_from_overrides_file_value() {
    let dir = temp_dir();
    fs::write(dir.join("query.qdef"), "search: from-file\n").unwrap();
    let interpreter = interpreter_after(
        "def r as request(GET)\n\
             r.query_string_from(\"query.qdef\")\n\
              .query_string(tuple(\"search\", \"manual\"))",
        &dir,
    );
    let query_strings = request_query_strings(&interpreter, "r");
    assert_eq!(
        query_string_value(&query_strings, "search"),
        Some("manual".to_string())
    );
}

#[test]
fn request_query_string_from_interpolates_with_var_called_after_file_load() {
    let dir = temp_dir();
    fs::write(dir.join("query.qdef"), "search: {{search_term}}\n").unwrap();
    let interpreter = interpreter_after(
        "def search_term as string(\"def language\")\n\
             def r as request(GET)\n\
             r.query_string_from(\"query.qdef\")\n\
              .with_var(search_term)",
        &dir,
    );
    let query_strings = request_query_strings(&interpreter, "r");
    assert_eq!(
        query_string_value(&query_strings, "search"),
        Some("def language".to_string())
    );
}

#[test]
fn request_query_string_from_interpolates_with_var_called_before_file_load() {
    let dir = temp_dir();
    fs::write(dir.join("query.qdef"), "search: {{search_term}}\n").unwrap();
    let interpreter = interpreter_after(
        "def search_term as string(\"def language\")\n\
             def r as request(GET)\n\
             r.with_var(search_term)\n\
              .query_string_from(\"query.qdef\")",
        &dir,
    );
    let query_strings = request_query_strings(&interpreter, "r");
    assert_eq!(
        query_string_value(&query_strings, "search"),
        Some("def language".to_string())
    );
}

#[test]
fn request_query_string_rejects_wrong_argument_count() {
    let error = interpret_error(
        "def r as request(GET)\nr.query_string(tuple(\"search\", \"def\"), tuple(\"page\", \"1\"))",
        ".",
    );
    assert!(
        matches!(error, DefError::Runtime(message) if message.contains("request.query_string expects 1 tuple argument"))
    );
}

#[test]
fn request_query_string_rejects_non_string_value() {
    let error = interpret_error(
        "def r as request(GET)\nr.query_string(tuple(\"page\", 1))",
        ".",
    );
    assert!(
        matches!(error, DefError::Runtime(message) if message.contains("tuple value must be a string"))
    );
}

#[test]
fn request_query_string_from_rejects_missing_file() {
    let dir = temp_dir();
    let error = interpret_error(
        "def r as request(GET)\nr.query_string_from(\"missing.qdef\")",
        &dir,
    );
    assert!(
        matches!(error, DefError::Runtime(message) if message.contains("query_string_from") && message.contains("missing.qdef"))
    );
}

#[test]
fn request_query_string_from_rejects_invalid_line_without_colon() {
    let dir = temp_dir();
    fs::write(dir.join("query.qdef"), "search: def\ninvalid\n").unwrap();
    let error = interpret_error(
        "def r as request(GET)\nr.query_string_from(\"query.qdef\")",
        &dir,
    );
    assert!(
        matches!(error, DefError::Runtime(message) if message.contains("line 2") && message.contains("expected 'Name: value'"))
    );
}

#[test]
fn request_query_string_from_rejects_empty_name() {
    let dir = temp_dir();
    fs::write(dir.join("query.qdef"), ": value\n").unwrap();
    let error = interpret_error(
        "def r as request(GET)\nr.query_string_from(\"query.qdef\")",
        &dir,
    );
    assert!(
        matches!(error, DefError::Runtime(message) if message.contains("line 1") && message.contains("query string name cannot be empty"))
    );
}
