use super::*;

#[test]
fn evaluates_addition_expression() {
    let value = run("1 + 2");
    assert_eq!(value, Value::Integer(3));
}

#[test]
fn evaluates_modulo_expression() {
    let value = run("5 % 2");
    assert_eq!(value, Value::Integer(1));
}

#[test]
fn evaluates_equality_expression() {
    let value = run("assert(1 + 2 == 3 == (\"def\" == \"postman\" == false))");
    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn evaluates_not_equal_expression() {
    let value = run("assert(true != false)");
    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn evaluates_numeric_comparison_expressions() {
    let value = run("assert(12 > 10)\n\
             assert(10 < 12)\n\
             assert(12 >= 12)\n\
             assert(10 <= 10)\n\
             assert(10.5 > 10)\n\
             assert(10 < 10.5)");
    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn evaluates_boolean_operator_expressions() {
    let value = run("assert(true and true)\n\
             assert((false and false) == false)\n\
             assert((true and false) == false)\n\
             assert((false and true) == false)\n\
             assert(true or false)\n\
             assert(false or true)\n\
             assert((false or false) == false)");
    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn evaluates_not_expression() {
    let value = run("assert(not false)\nassert((not true) == false)");
    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn rejects_not_on_non_boolean_value() {
    let error = interpret_error("assert(not 1)", ".");
    assert!(
        matches!(error, DefError::Runtime(message) if message.contains("not operator supports only boolean values"))
    );
}

#[test]
fn rejects_non_boolean_operator_operands() {
    let error = interpret_error("assert(1 and true)", ".");
    assert!(
        matches!(error, DefError::Runtime(message) if message.contains("support only boolean values"))
    );
}

#[test]
fn rejects_non_numeric_ordered_comparison() {
    let error = interpret_error("assert(\"a\" > \"b\")", ".");
    assert!(
        matches!(error, DefError::Runtime(message) if message.contains("support only integer and float"))
    );
}
