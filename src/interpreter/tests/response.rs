use super::*;
use crate::interpreter::http::call_response_method;

#[test]
fn response_exposes_body_and_status() {
    let value = run("def res as response()\n\
             assert(res.body() == \"\" == (res.status() == 0))");
    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn response_headers_returns_array() {
    let response = ResponseValue {
        status: 200,
        body: String::new(),
        headers: vec![
            ("content-type".to_string(), "application/json".to_string()),
            ("x-def-test".to_string(), "hello".to_string()),
        ],
        duration_ms: 0,
    };
    let value = call_response_method(response, "headers", Vec::new()).unwrap();
    assert_eq!(
        value,
        Value::Array(vec![
            Value::Tuple {
                key: "content-type".to_string(),
                value: Box::new(Value::String("application/json".to_string())),
            },
            Value::Tuple {
                key: "x-def-test".to_string(),
                value: Box::new(Value::String("hello".to_string())),
            },
        ])
    );
}

#[test]
fn response_headers_default_is_empty_array() {
    let value = run("def res as response()\n\
             assert(res.headers() == array())");
    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn response_headers_rejects_arguments() {
    let response = ResponseValue {
        status: 200,
        body: String::new(),
        headers: Vec::new(),
        duration_ms: 0,
    };
    let error = call_response_method(response, "headers", vec![Value::String("x".to_string())])
        .unwrap_err();
    assert!(
        matches!(error, DefError::Runtime(message) if message.contains("response.headers expects 0 arguments"))
    );
}

#[test]
fn response_can_be_declared_from_request_do_value() {
    let value = run("def res as response()\n\
             assert(res.body() == \"\" == (res.status() == 0))");
    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn response_can_be_coerced_to_string_body_for_compatibility() {
    let value = run("def response_body as string()\n\
             def res as response()\n\
             response_body = res\n\
             assert(response_body == \"\")");
    assert_eq!(value, Value::Boolean(true));
}
