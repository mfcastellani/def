use super::*;
use crate::interpreter::http::call_response_method;

#[test]
fn json_simple_integer_field() {
    let result = call_response_method(
        json_response(r#"{"id":1,"active":true}"#),
        "json",
        vec![Value::String("$.id".to_string())],
    )
    .unwrap();
    assert_eq!(result, Value::Integer(1));
}

#[test]
fn json_string_field() {
    let result = call_response_method(
        json_response(r#"{"id":1,"name":"Marcelo"}"#),
        "json",
        vec![Value::String("$.name".to_string())],
    )
    .unwrap();
    assert_eq!(result, Value::String("Marcelo".to_string()));
}

#[test]
fn json_nested_field() {
    let result = call_response_method(
        json_response(r#"{"user":{"name":"Alice","age":30}}"#),
        "json",
        vec![Value::String("$.user.name".to_string())],
    )
    .unwrap();
    assert_eq!(result, Value::String("Alice".to_string()));
}

#[test]
fn json_array_index() {
    let result = call_response_method(
        json_response(r#"{"items":["first","second","third"]}"#),
        "json",
        vec![Value::String("$.items[0]".to_string())],
    )
    .unwrap();
    assert_eq!(result, Value::String("first".to_string()));
}

#[test]
fn json_array_index_then_field() {
    let result = call_response_method(
        json_response(r#"{"users":[{"name":"Alice"},{"name":"Bob"}]}"#),
        "json",
        vec![Value::String("$.users[1].name".to_string())],
    )
    .unwrap();
    assert_eq!(result, Value::String("Bob".to_string()));
}

#[test]
fn json_boolean_value() {
    let result = call_response_method(
        json_response(r#"{"active":true,"deleted":false}"#),
        "json",
        vec![Value::String("$.active".to_string())],
    )
    .unwrap();
    assert_eq!(result, Value::Boolean(true));
}

#[test]
fn json_null_value() {
    let result = call_response_method(
        json_response(r#"{"data":null}"#),
        "json",
        vec![Value::String("$.data".to_string())],
    )
    .unwrap();
    assert_eq!(result, Value::Nil);
}

#[test]
fn json_float_value() {
    let result = call_response_method(
        json_response(r#"{"score":9.5}"#),
        "json",
        vec![Value::String("$.score".to_string())],
    )
    .unwrap();
    assert_eq!(result, Value::Float(9.5));
}

#[test]
fn json_root_returns_compact_string() {
    let body = r#"{"id":1}"#;
    let result = call_response_method(
        json_response(body),
        "json",
        vec![Value::String("$".to_string())],
    )
    .unwrap();
    assert!(matches!(result, Value::String(_)));
}

#[test]
fn json_path_not_found_errors() {
    let error = call_response_method(
        json_response(r#"{"id":1}"#),
        "json",
        vec![Value::String("$.missing".to_string())],
    )
    .unwrap_err();
    assert!(
        matches!(&error, DefError::Runtime(msg) if msg.contains("json path '$.missing' not found")),
        "unexpected error: {error:?}"
    );
}

#[test]
fn json_invalid_body_errors() {
    let error = call_response_method(
        json_response("not json"),
        "json",
        vec![Value::String("$.id".to_string())],
    )
    .unwrap_err();
    assert!(
        matches!(&error, DefError::Runtime(msg) if msg.contains("response body is not valid JSON")),
        "unexpected error: {error:?}"
    );
}

#[test]
fn json_invalid_path_errors() {
    let error = call_response_method(
        json_response(r#"{"id":1}"#),
        "json",
        vec![Value::String("id".to_string())],
    )
    .unwrap_err();
    assert!(
        matches!(&error, DefError::Runtime(msg) if msg.contains("invalid json path 'id'")),
        "unexpected error: {error:?}"
    );
}

#[test]
fn json_invalid_path_empty_field_errors() {
    let error = call_response_method(
        json_response(r#"{"id":1}"#),
        "json",
        vec![Value::String("$.".to_string())],
    )
    .unwrap_err();
    assert!(
        matches!(&error, DefError::Runtime(msg) if msg.contains("invalid json path")),
        "unexpected error: {error:?}"
    );
}

#[test]
fn json_invalid_path_unclosed_bracket_errors() {
    let error = call_response_method(
        json_response(r#"{"items":[1,2]}"#),
        "json",
        vec![Value::String("$.items[".to_string())],
    )
    .unwrap_err();
    assert!(
        matches!(&error, DefError::Runtime(msg) if msg.contains("invalid json path")),
        "unexpected error: {error:?}"
    );
}

#[test]
fn json_invalid_path_empty_index_errors() {
    let error = call_response_method(
        json_response(r#"{"items":[1,2]}"#),
        "json",
        vec![Value::String("$.items[]".to_string())],
    )
    .unwrap_err();
    assert!(
        matches!(&error, DefError::Runtime(msg) if msg.contains("invalid json path")),
        "unexpected error: {error:?}"
    );
}

#[test]
fn json_invalid_path_non_numeric_index_errors() {
    let error = call_response_method(
        json_response(r#"{"items":[1,2]}"#),
        "json",
        vec![Value::String("$.items[abc]".to_string())],
    )
    .unwrap_err();
    assert!(
        matches!(&error, DefError::Runtime(msg) if msg.contains("invalid json path")),
        "unexpected error: {error:?}"
    );
}

#[test]
fn json_wrong_arg_count_errors() {
    let error = call_response_method(json_response(r#"{"id":1}"#), "json", vec![]).unwrap_err();
    assert!(
        matches!(&error, DefError::Runtime(msg) if msg.contains("response.json expects 1 argument")),
        "unexpected error: {error:?}"
    );
}

#[test]
fn json_non_string_arg_errors() {
    let error = call_response_method(
        json_response(r#"{"id":1}"#),
        "json",
        vec![Value::Integer(1)],
    )
    .unwrap_err();
    assert!(
        matches!(&error, DefError::Runtime(msg) if msg.contains("response.json expects a string path argument")),
        "unexpected error: {error:?}"
    );
}

#[test]
fn json_exists_returns_true_for_existing_path() {
    let result = call_response_method(
        json_response(r#"{"id":1,"active":true}"#),
        "json_exists",
        vec![Value::String("$.id".to_string())],
    )
    .unwrap();
    assert_eq!(result, Value::Boolean(true));
}

#[test]
fn json_exists_returns_false_for_missing_path() {
    let result = call_response_method(
        json_response(r#"{"id":1}"#),
        "json_exists",
        vec![Value::String("$.missing".to_string())],
    )
    .unwrap();
    assert_eq!(result, Value::Boolean(false));
}

#[test]
fn json_exists_returns_false_for_missing_nested_path() {
    let result = call_response_method(
        json_response(r#"{"user":{"name":"Alice"}}"#),
        "json_exists",
        vec![Value::String("$.user.email".to_string())],
    )
    .unwrap();
    assert_eq!(result, Value::Boolean(false));
}

#[test]
fn json_exists_wrong_arg_count_errors() {
    let error =
        call_response_method(json_response(r#"{"id":1}"#), "json_exists", vec![]).unwrap_err();
    assert!(
        matches!(&error, DefError::Runtime(msg) if msg.contains("response.json_exists expects 1 argument")),
        "unexpected error: {error:?}"
    );
}

#[test]
fn json_exists_non_string_arg_errors() {
    let error = call_response_method(
        json_response(r#"{"id":1}"#),
        "json_exists",
        vec![Value::Boolean(true)],
    )
    .unwrap_err();
    assert!(
        matches!(&error, DefError::Runtime(msg) if msg.contains("response.json_exists expects a string path argument")),
        "unexpected error: {error:?}"
    );
}

#[test]
fn json_integration_with_mock_and_body_from() {
    let dir = temp_dir();
    fs::write(
        dir.join("user.json"),
        r#"{"id":1,"active":true,"score":9.5}"#,
    )
    .unwrap();
    let value = run_with_base_dir(
        "def m as mock(GET, \"https://api.example.com/users/1\")\n\
         .body_from(\"user.json\")\n\
         .reply(200)\n\
         def res as response(\n\
           request(GET)\n\
             .path(\"https://api.example.com/users/1\")\n\
             .with_mocks(m)\n\
             .do()\n\
         )\n\
         assert(res.json(\"$.id\") == 1)\n\
         assert(res.json(\"$.active\") == true)\n\
         assert(res.json_exists(\"$.id\"))\n\
         assert(not res.json_exists(\"$.missing\"))",
        dir.to_str().unwrap(),
    );
    assert_eq!(value, Value::Boolean(true));
}
