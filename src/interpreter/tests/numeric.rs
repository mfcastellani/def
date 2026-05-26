use super::*;

#[test]
fn integer_random_returns_value_in_range() {
    let value = run("def n as integer().random(1, 10)\nassert(n >= 1)\nassert(n <= 10)");
    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn integer_random_with_equal_bounds_returns_that_value() {
    let value = run("def n as integer().random(5, 5)\nassert(n == 5)");
    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn integer_random_wrong_arg_count_errors() {
    let error = interpret_error("def n as integer().random(1)", ".");
    assert!(
        matches!(&error, DefError::Runtime(msg) if msg.contains("integer.random expects 2 arguments")),
        "unexpected error: {error:?}"
    );
}

#[test]
fn integer_random_non_integer_arg_errors() {
    let error = interpret_error("def n as integer().random(1.0, 10.0)", ".");
    assert!(
        matches!(&error, DefError::Runtime(msg) if msg.contains("integer.random expects integer arguments")),
        "unexpected error: {error:?}"
    );
}

#[test]
fn integer_random_inverted_range_errors() {
    let error = interpret_error("def n as integer().random(10, 1)", ".");
    assert!(
        matches!(&error, DefError::Runtime(msg) if msg.contains("integer.random: min")),
        "unexpected error: {error:?}"
    );
}

#[test]
fn integer_to_string_converts_number() {
    let value = run("def n as integer(42)\ndef s as string(n.to_string())\nassert(s == \"42\")");
    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn integer_to_string_zero() {
    let value = run("def n as integer(0)\ndef s as string(n.to_string())\nassert(s == \"0\")");
    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn integer_to_string_negative() {
    let value = run("def n as integer(-7)\ndef s as string(n.to_string())\nassert(s == \"-7\")");
    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn integer_to_string_wrong_arg_count_errors() {
    let error = interpret_error("def n as integer(1)\nn.to_string(\"extra\")", ".");
    assert!(
        matches!(&error, DefError::Runtime(msg) if msg.contains("integer.to_string expects 0 arguments")),
        "unexpected error: {error:?}"
    );
}

#[test]
fn integer_undefined_method_errors() {
    let error = interpret_error("def n as integer(1)\nn.unknown()", ".");
    assert!(
        matches!(&error, DefError::Runtime(msg) if msg.contains("undefined integer method 'unknown'")),
        "unexpected error: {error:?}"
    );
}

#[test]
fn float_random_returns_value_in_range() {
    let value = run("def f as float().random(0.0, 1.0)\nassert(f >= 0.0)\nassert(f <= 1.0)");
    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn float_random_accepts_integer_args() {
    let value = run("def f as float().random(1, 100)\nassert(f >= 1.0)\nassert(f <= 100.0)");
    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn float_random_wrong_arg_count_errors() {
    let error = interpret_error("def f as float().random(1.0)", ".");
    assert!(
        matches!(&error, DefError::Runtime(msg) if msg.contains("float.random expects 2 arguments")),
        "unexpected error: {error:?}"
    );
}

#[test]
fn float_random_inverted_range_errors() {
    let error = interpret_error("def f as float().random(10.0, 1.0)", ".");
    assert!(
        matches!(&error, DefError::Runtime(msg) if msg.contains("float.random: min")),
        "unexpected error: {error:?}"
    );
}

#[test]
fn float_to_string_converts_number() {
    let value = run("def f as float(9.5)\ndef s as string(f.to_string())\nassert(s == \"9.5\")");
    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn float_to_string_wrong_arg_count_errors() {
    let error = interpret_error("def f as float(1.5)\nf.to_string(\"extra\")", ".");
    assert!(
        matches!(&error, DefError::Runtime(msg) if msg.contains("float.to_string expects 0 arguments")),
        "unexpected error: {error:?}"
    );
}

#[test]
fn float_undefined_method_errors() {
    let error = interpret_error("def f as float(1.5)\nf.unknown()", ".");
    assert!(
        matches!(&error, DefError::Runtime(msg) if msg.contains("undefined float method 'unknown'")),
        "unexpected error: {error:?}"
    );
}
