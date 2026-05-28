use super::*;

#[test]
fn while_loop_counts_to_target() {
    let value = run(
        "def n as integer(0)\n\
         while n < 5 do (\n\
           n += 1\n\
         )\n\
         assert(n == 5)",
    );
    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn while_loop_skips_body_when_condition_false() {
    let value = run(
        "def n as integer(0)\n\
         while false do (\n\
           n += 1\n\
         )\n\
         assert(n == 0)",
    );
    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn while_condition_must_be_boolean() {
    let error = interpret_error("while 1 do (\nprint(\"bad\")\n)", ".");
    assert!(
        matches!(error, DefError::Runtime(message) if message.contains("condition must evaluate to boolean"))
    );
}

#[test]
fn break_exits_while_loop_early() {
    let value = run(
        "def n as integer(0)\n\
         while true do (\n\
           if n == 3 (\n\
             break()\n\
           )\n\
           n += 1\n\
         )\n\
         assert(n == 3)",
    );
    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn next_skips_rest_of_while_iteration() {
    let value = run(
        "def n as integer(0)\n\
         def count as integer(0)\n\
         while n < 5 do (\n\
           n += 1\n\
           if n == 3 (\n\
             next()\n\
           )\n\
           count += 1\n\
         )\n\
         assert(count == 4)",
    );
    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn break_exits_for_loop_early() {
    let value = run(
        "def items as array(1, 2, 3, 4, 5)\n\
         def total as integer(0)\n\
         for item in items (\n\
           if item == 3 (\n\
             break()\n\
           )\n\
           total += item\n\
         )\n\
         assert(total == 3)",
    );
    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn next_skips_rest_of_for_iteration() {
    let value = run(
        "def items as array(1, 2, 3, 4, 5)\n\
         def total as integer(0)\n\
         for item in items (\n\
           if item == 3 (\n\
             next()\n\
           )\n\
           total += item\n\
         )\n\
         assert(total == 12)",
    );
    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn break_outside_loop_errors() {
    let error = interpret_error("break()", ".");
    assert!(
        matches!(&error, DefError::Runtime(msg) if msg.contains("break() called outside of a loop")),
        "unexpected error: {error:?}"
    );
}

#[test]
fn next_outside_loop_errors() {
    let error = interpret_error("next()", ".");
    assert!(
        matches!(&error, DefError::Runtime(msg) if msg.contains("next() called outside of a loop")),
        "unexpected error: {error:?}"
    );
}

#[test]
fn break_with_args_errors() {
    let error = interpret_error("while true do (\nbreak(1)\n)", ".");
    assert!(
        matches!(&error, DefError::Runtime(msg) if msg.contains("break() takes no arguments")),
        "unexpected error: {error:?}"
    );
}

#[test]
fn if_executes_then_or_else_branch() {
    let value = run("def status as integer(200)\n\
             def message as string()\n\
             if status == 200 (\n\
               message = \"ok\"\n\
             ) else (\n\
               message = \"error\"\n\
             )\n\
             assert(message == \"ok\")");
    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn if_condition_must_be_boolean() {
    let error = interpret_error("if 1 (\nprint(\"bad\")\n)", ".");
    assert!(
        matches!(error, DefError::Runtime(message) if message.contains("condition must evaluate to boolean"))
    );
}

#[test]
fn block_variable_does_not_leak_from_if() {
    let error = interpret_error(
        "if true (\n  def scoped as string(\"inside\")\n)\nscoped",
        ".",
    );
    assert!(
        matches!(error, DefError::Runtime(message) if message.contains("undefined identifier 'scoped'"))
    );
}

#[test]
fn function_variable_does_not_leak_to_global_scope() {
    let error = interpret_error(
        "def build as function() (\n  def scoped as string(\"inside\")\n  scoped\n)\nbuild()\nscoped",
        ".",
    );
    assert!(
        matches!(error, DefError::Runtime(message) if message.contains("undefined identifier 'scoped'"))
    );
}

#[test]
fn evaluates_for_loop_over_array() {
    let value = run("def items as array(1, 2, 3)\n\
             def total as integer(0)\n\
             for item in items (\n\
               total += item\n\
             )\n\
             assert(total == 6)");
    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn for_loop_over_empty_array_does_not_execute_body() {
    let value = run("def items as array\n\
             def total as integer(0)\n\
             for item in items (\n\
               total += 1\n\
             )\n\
             assert(total == 0)");
    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn rejects_for_loop_over_non_array() {
    let error = interpret_error(
        "def name as string(\"Marcelo\")\nfor item in name (\nprint(item)\n)",
        ".",
    );
    assert!(
        matches!(error, DefError::Runtime(message) if message.contains("for loop expects an array"))
    );
}

#[test]
fn defines_and_calls_sum_function() {
    let value = run("def sum as function(a as integer, b as integer) (\n  a + b\n)\nsum(1, 2)");
    assert_eq!(value, Value::Integer(3));
}

#[test]
fn validates_function_argument_types() {
    let mut lexer = Lexer::new(
        "def sum as function(a as integer, b as integer) (\n  a + b\n)\nsum(\"one\", 2)",
    );
    let tokens = lexer.tokenize().unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let error = Interpreter::new().interpret(&program).unwrap_err();
    assert!(matches!(error, DefError::Runtime(_)));
}

#[test]
fn match_returns_first_matching_arm() {
    let value = run(
        "def n as integer(2)\nmatch n (\n  1 => \"one\",\n  2 => \"two\",\n  _ => \"other\"\n)",
    );
    assert_eq!(value, Value::String("two".to_string()));
}

#[test]
fn match_can_initialize_variable() {
    let value = run("def n as integer(3)\n\
             def label as string(\n\
               match n (\n\
                 1 => \"one\",\n\
                 2 => \"two\",\n\
                 _ => \"other\"\n\
               )\n\
             )\n\
             assert(label == \"other\")");
    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn match_can_assign_variable() {
    let value = run("def status as integer(200)\n\
             def message as string()\n\
             message = match status (\n\
               200 => \"ok\",\n\
               404 => \"not found\",\n\
               _ => \"unexpected\"\n\
             )\n\
             assert(message == \"ok\")");
    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn match_rejects_unmatched_value() {
    let mut lexer = Lexer::new("match 3 (\n  1 => \"one\"\n)");
    let tokens = lexer.tokenize().unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let error = Interpreter::new().interpret(&program).unwrap_err();
    assert!(matches!(error, DefError::Runtime(message) if message.contains("did not match")));
}
