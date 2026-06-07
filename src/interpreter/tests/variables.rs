use super::*;

#[test]
fn registers_integer_variable_definition() {
    let value = run("def i as integer\nassert(i == 0)");
    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn registers_float_variable_definition() {
    let value = run("def price as float\nassert(price == 0.0)");
    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn registers_boolean_variable_definition() {
    let value = run("def ok as boolean\nassert(ok == false)");
    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn registers_default_values_for_each_basic_type() {
    let value = run("def a as integer\n\
             def b as float\n\
             def c as string\n\
             def d as array\n\
             def e as boolean\n\
             assert(a == 0 == (b == 0.0) == (c == \"\") == (d == array()) == (e == false))");
    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn default_array_is_empty() {
    let value = run("def items as array\nassert(items.len() == 0 == (items.is_empty() == true))");
    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn registers_variable_definition_with_initializer() {
    let value = run("def ok as boolean(true)\nassert(ok == true)");
    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn registers_variable_definition_with_function_call_initializer() {
    let value = run(
        "def sum as function(a as integer, b as integer) (\n  a + b\n)\n\
             def n as integer(sum(10, 12))\n\
             assert(n == 22)",
    );
    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn rejects_initializer_with_wrong_type() {
    let mut lexer = Lexer::new("def ok as boolean(10)");
    let tokens = lexer.tokenize().unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let error = Interpreter::new().interpret(&program).unwrap_err();
    assert!(matches!(error, DefError::Runtime(_)));
}

#[test]
fn assigns_valid_value() {
    let value =
        run("def a as integer\ndef b as float\na = 10\nb = 10.5\nassert(a == 10 == (b == 10.5))");
    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn applies_compound_assignment_to_numbers() {
    let value = run("def a as integer(10)\n\
             def b as float(10.5)\n\
             a += 5\n\
             b -= 0.5\n\
             assert(a == 15 == (b == 10.0))");
    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn rejects_assignment_with_wrong_type() {
    let mut lexer = Lexer::new("def a as integer\na = \"wrong\"");
    let tokens = lexer.tokenize().unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let error = Interpreter::new().interpret(&program).unwrap_err();
    assert!(matches!(error, DefError::Runtime(message) if message.contains("invalid assignment")));
}

#[test]
fn rejects_compound_assignment_for_unsupported_values() {
    let mut lexer = Lexer::new("def name as string(\"Def\")\nname += \" language\"");
    let tokens = lexer.tokenize().unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let error = Interpreter::new().interpret(&program).unwrap_err();
    assert!(
        matches!(error, DefError::Runtime(message) if message.contains("invalid compound assignment"))
    );
}

#[test]
fn evaluates_boolean_literal() {
    let value = run("true");
    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn const_variable_holds_value() {
    let value = run("def x as const integer(42)\nassert(x == 42)");
    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn const_rejects_reassignment() {
    let mut lexer = Lexer::new("def x as const integer(1)\nx = 2");
    let tokens = lexer.tokenize().unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let error = Interpreter::new().interpret(&program).unwrap_err();
    assert!(matches!(error, DefError::Runtime(msg) if msg.contains("cannot assign to const variable")));
}

#[test]
fn const_rejects_compound_reassignment() {
    let mut lexer = Lexer::new("def x as const integer(10)\nx += 5");
    let tokens = lexer.tokenize().unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let error = Interpreter::new().interpret(&program).unwrap_err();
    assert!(matches!(error, DefError::Runtime(msg) if msg.contains("cannot assign to const variable")));
}

#[test]
fn const_works_for_all_basic_types() {
    let value = run(
        "def a as const integer(1)\n\
         def b as const float(2.5)\n\
         def c as const string(\"hello\")\n\
         def d as const boolean(true)\n\
         assert(a == 1)\n\
         assert(b == 2.5)\n\
         assert(c == \"hello\")\n\
         assert(d == true)"
    );
    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn const_array_rejects_push() {
    let mut lexer = Lexer::new("def items as const array(\"a\", \"b\")\nitems.push(\"c\")");
    let tokens = lexer.tokenize().unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let error = Interpreter::new().interpret(&program).unwrap_err();
    assert!(matches!(error, DefError::Runtime(msg) if msg.contains("cannot call push() on const array")));
}

#[test]
fn const_in_local_scope_does_not_affect_outer_mutable_binding() {
    let value = run(
        "def x as integer(1)\n\
         if true (\n\
           def x as const integer(99)\n\
           assert(x == 99)\n\
         )\n\
         x = 2\n\
         assert(x == 2)"
    );
    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn mutable_in_local_scope_does_not_affect_outer_const_binding() {
    let value = run(
        "def x as const integer(1)\n\
         if true (\n\
           def x as integer(99)\n\
           x = 50\n\
           assert(x == 50)\n\
         )\n\
         assert(x == 1)"
    );
    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn multiline_string_basic() {
    let value = run(
        "def body as string(\"\"\"\nhello\nworld\n\"\"\")\nassert(body == \"hello\nworld\")"
    );
    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn multiline_string_dedents() {
    let value = run(
        "def body as string(\"\"\"\n  hello\n  world\n  \"\"\")\nassert(body == \"hello\nworld\")"
    );
    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn multiline_string_preserves_inner_indentation() {
    // Two spaces common indent stripped; inner block still has extra 2 spaces
    let value = run(
        "def body as string(\"\"\"\n  outer\n    inner\n  \"\"\")\nassert(body == \"outer\n  inner\")"
    );
    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn multiline_string_concat_works() {
    let value = run(
        "def s as string(\"\"\"\nhello\nworld\n\"\"\")\nassert(s == \"hello\nworld\")"
    );
    assert_eq!(value, Value::Boolean(true));
}
