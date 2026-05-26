use super::*;

#[test]
fn assert_returns_true_when_expression_is_true() {
    let value = run("assert(1 + 2 == 3)");
    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn assert_fails_when_expression_is_false() {
    let error = interpret_error("assert(\"def\" == \"postman\")", ".");
    assert!(matches!(error, DefError::Runtime(message) if message == "assertion failed"));
}

#[test]
fn assert_failure_aborts_script_execution() {
    let error = interpret_error(
        "def value as integer(1)\n\
             assert(value == 2)\n\
             value = 3",
        ".",
    );
    assert!(matches!(error, DefError::Runtime(message) if message == "assertion failed"));
}

#[test]
fn assert_statement_accepts_boolean_expression() {
    let value = run("assert(1 + 2 == 3)");
    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn assert_statement_rejects_non_boolean_expression() {
    let error = interpret_error("assert(10)", ".");
    assert!(matches!(error, DefError::Runtime(message) if message.contains("boolean expression")));
}

#[test]
fn assert_rejects_two_argument_compatibility_format() {
    let error = interpret_error("assert(1 + 2, 3)", ".");
    assert!(
        matches!(error, DefError::Runtime(message) if message.contains("assert expects 1 boolean expression"))
    );
}

#[test]
fn concat_joins_strings() {
    let value =
        run("def name as string(concat(\"Mar\", \"ce\", \"lo\"))\nassert(name == \"Marcelo\")");
    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn concat_rejects_non_string_arguments() {
    let error = interpret_error("concat(\"Age\", 48)", ".");
    assert!(matches!(error, DefError::Runtime(message) if message.contains("only string")));
}

#[test]
fn delay_accepts_integer_milliseconds() {
    let value = run("delay(0)\nassert(true == true)");
    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn delay_rejects_non_integer_argument() {
    let mut lexer = Lexer::new("delay(1.5)");
    let tokens = lexer.tokenize().unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let error = Interpreter::new().interpret(&program).unwrap_err();
    assert!(
        matches!(error, DefError::Runtime(message) if message.contains("delay expects an integer"))
    );
}

#[test]
fn print_accepts_string_argument() {
    let value = run("print(\"hello\")\nassert(true == true)");
    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn print_accepts_non_string_argument() {
    let value = run("print(10)\nassert(true == true)");
    assert_eq!(value, Value::Boolean(true));
}
