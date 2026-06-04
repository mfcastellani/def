use super::*;
use crate::interpreter::http::call_response_method;
use std::path::Path;

fn html_response(body: &str) -> ResponseValue {
    ResponseValue {
        status: 200,
        body: body.to_string(),
        headers: vec![("content-type".to_string(), "text/html; charset=utf-8".to_string())],
        duration_ms: 0,
        method: String::new(),
        url: String::new(),
    }
}

// ── body_matches ──────────────────────────────────────────────────────────────

#[test]
fn body_matches_returns_true_when_pattern_found() {
    let result = call_response_method(
        html_response("<h1>Hello World</h1>"),
        "body_matches",
        vec![Value::String(r"Hello\s+World".to_string())],
        Path::new("."),
    )
    .unwrap();
    assert_eq!(result, Value::Boolean(true));
}

#[test]
fn body_matches_returns_false_when_pattern_not_found() {
    let result = call_response_method(
        html_response("<h1>Hello World</h1>"),
        "body_matches",
        vec![Value::String(r"\d{4}".to_string())],
        Path::new("."),
    )
    .unwrap();
    assert_eq!(result, Value::Boolean(false));
}

#[test]
fn body_matches_works_on_json_body_too() {
    let result = call_response_method(
        json_response(r#"{"status":"active","count":42}"#),
        "body_matches",
        vec![Value::String(r#""count":\d+"#.to_string())],
        Path::new("."),
    )
    .unwrap();
    assert_eq!(result, Value::Boolean(true));
}

#[test]
fn body_matches_errors_on_invalid_regex() {
    let error = call_response_method(
        html_response("<p>text</p>"),
        "body_matches",
        vec![Value::String("[invalid".to_string())],
        Path::new("."),
    )
    .unwrap_err();
    assert!(
        matches!(&error, DefError::Runtime(msg) if msg.contains("invalid regex")),
        "unexpected error: {error:?}"
    );
}

#[test]
fn body_matches_errors_on_wrong_arg_count() {
    let error =
        call_response_method(html_response("<p>x</p>"), "body_matches", vec![], Path::new("."))
            .unwrap_err();
    assert!(
        matches!(&error, DefError::Runtime(msg) if msg.contains("response.body_matches expects 1 argument")),
        "unexpected error: {error:?}"
    );
}

// ── html ──────────────────────────────────────────────────────────────────────

#[test]
fn html_returns_text_of_first_match() {
    let result = call_response_method(
        html_response("<html><body><h1>Title</h1><h1>Other</h1></body></html>"),
        "html",
        vec![Value::String("h1".to_string())],
        Path::new("."),
    )
    .unwrap();
    assert_eq!(result, Value::String("Title".to_string()));
}

#[test]
fn html_returns_empty_string_when_no_match() {
    let result = call_response_method(
        html_response("<html><body><p>Hello</p></body></html>"),
        "html",
        vec![Value::String("h1".to_string())],
        Path::new("."),
    )
    .unwrap();
    assert_eq!(result, Value::String(String::new()));
}

#[test]
fn html_works_with_class_selector() {
    let result = call_response_method(
        html_response(r#"<html><body><p class="intro">Welcome</p></body></html>"#),
        "html",
        vec![Value::String("p.intro".to_string())],
        Path::new("."),
    )
    .unwrap();
    assert_eq!(result, Value::String("Welcome".to_string()));
}

#[test]
fn html_works_with_title_tag() {
    let result = call_response_method(
        html_response("<html><head><title>My Page</title></head><body></body></html>"),
        "html",
        vec![Value::String("title".to_string())],
        Path::new("."),
    )
    .unwrap();
    assert_eq!(result, Value::String("My Page".to_string()));
}

#[test]
fn html_errors_on_invalid_selector() {
    let error = call_response_method(
        html_response("<p>text</p>"),
        "html",
        vec![Value::String("###bad".to_string())],
        Path::new("."),
    )
    .unwrap_err();
    assert!(
        matches!(&error, DefError::Runtime(msg) if msg.contains("invalid CSS selector")),
        "unexpected error: {error:?}"
    );
}

// ── html_all ──────────────────────────────────────────────────────────────────

#[test]
fn html_all_returns_all_matches() {
    let result = call_response_method(
        html_response("<ul><li>one</li><li>two</li><li>three</li></ul>"),
        "html_all",
        vec![Value::String("li".to_string())],
        Path::new("."),
    )
    .unwrap();
    assert_eq!(
        result,
        Value::Array(vec![
            Value::String("one".to_string()),
            Value::String("two".to_string()),
            Value::String("three".to_string()),
        ])
    );
}

#[test]
fn html_all_returns_empty_array_when_no_match() {
    let result = call_response_method(
        html_response("<p>Hello</p>"),
        "html_all",
        vec![Value::String("li".to_string())],
        Path::new("."),
    )
    .unwrap();
    assert_eq!(result, Value::Array(vec![]));
}

// ── html_attr ─────────────────────────────────────────────────────────────────

#[test]
fn html_attr_returns_attribute_of_first_match() {
    let result = call_response_method(
        html_response(r#"<a href="https://example.com">link</a>"#),
        "html_attr",
        vec![
            Value::String("a".to_string()),
            Value::String("href".to_string()),
        ],
        Path::new("."),
    )
    .unwrap();
    assert_eq!(result, Value::String("https://example.com".to_string()));
}

#[test]
fn html_attr_returns_empty_string_when_no_match() {
    let result = call_response_method(
        html_response("<p>Hello</p>"),
        "html_attr",
        vec![
            Value::String("a".to_string()),
            Value::String("href".to_string()),
        ],
        Path::new("."),
    )
    .unwrap();
    assert_eq!(result, Value::String(String::new()));
}

#[test]
fn html_attr_returns_empty_string_when_attr_missing() {
    let result = call_response_method(
        html_response("<a>link without href</a>"),
        "html_attr",
        vec![
            Value::String("a".to_string()),
            Value::String("href".to_string()),
        ],
        Path::new("."),
    )
    .unwrap();
    assert_eq!(result, Value::String(String::new()));
}

#[test]
fn html_attr_errors_on_wrong_arg_count() {
    let error = call_response_method(
        html_response("<a href='x'>link</a>"),
        "html_attr",
        vec![Value::String("a".to_string())],
        Path::new("."),
    )
    .unwrap_err();
    assert!(
        matches!(&error, DefError::Runtime(msg) if msg.contains("response.html_attr expects 2 arguments")),
        "unexpected error: {error:?}"
    );
}

// ── form_from integration ─────────────────────────────────────────────────────

#[test]
fn form_from_sets_urlencoded_body_and_content_type() {
    let dir = temp_dir();
    fs::write(dir.join("login.fdef"), "username: alice\npassword: s3cr3t\n").unwrap();

    let value = run_with_base_dir(
        "def m as mock(POST, \"https://api.example.com/login\")\n\
         .reply(200, \"ok\")\n\
         def res as response(\n\
           request(POST)\n\
             .path(\"https://api.example.com/login\")\n\
             .form_from(\"login.fdef\")\n\
             .with_mocks(m)\n\
             .do()\n\
         )\n\
         assert(res.ok())",
        dir.to_str().unwrap(),
    );
    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn form_from_url_encodes_special_characters() {
    let dir = temp_dir();
    fs::write(dir.join("search.fdef"), "q: hello world\npage: 1\n").unwrap();

    let interpreter = interpreter_after(
        "def r as request(POST)\n\
         r.path(\"https://api.example.com/search\")\n\
          .form_from(\"search.fdef\")",
        dir.as_path(),
    );

    match interpreter.variables.get("r") {
        Some(Value::Request(req)) => {
            assert_eq!(req.body.as_deref(), Some("q=hello+world&page=1"));
            let ct = req
                .headers
                .iter()
                .find(|(k, _)| k == "Content-Type")
                .map(|(_, v)| v.as_str());
            assert_eq!(ct, Some("application/x-www-form-urlencoded"));
        }
        other => panic!("expected request, got {other:?}"),
    }
}
