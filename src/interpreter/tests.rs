use super::*;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::value::ResponseValue;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

static TEMP_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn run(input: &str) -> Value {
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize().unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    Interpreter::new().interpret(&program).unwrap()
}

fn run_with_base_dir(input: &str, base_dir: &str) -> Value {
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize().unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    Interpreter::with_base_dir(base_dir)
        .interpret(&program)
        .unwrap()
}

fn interpreter_after(input: &str, base_dir: impl Into<PathBuf>) -> Interpreter {
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize().unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    let mut interpreter = Interpreter::with_base_dir(base_dir);
    interpreter.interpret(&program).unwrap();
    interpreter
}

fn interpret_error(input: &str, base_dir: impl Into<PathBuf>) -> DefError {
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize().unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    Interpreter::with_base_dir(base_dir)
        .interpret(&program)
        .unwrap_err()
}

fn temp_dir() -> PathBuf {
    let id = TEMP_COUNTER.fetch_add(1, Ordering::SeqCst);
    let path = std::env::temp_dir().join(format!("def-headers-test-{id}"));
    fs::create_dir_all(&path).unwrap();
    path
}

fn request_headers(interpreter: &Interpreter, name: &str) -> Vec<(String, String)> {
    match interpreter.variables.get(name) {
        Some(Value::Request(request)) => request.headers.clone(),
        value => panic!("expected request '{name}', got {value:?}"),
    }
}

fn request_query_strings(interpreter: &Interpreter, name: &str) -> Vec<(String, String)> {
    match interpreter.variables.get(name) {
        Some(Value::Request(request)) => request.query_strings.clone(),
        value => panic!("expected request '{name}', got {value:?}"),
    }
}

fn header_value(headers: &[(String, String)], name: &str) -> Option<String> {
    let name = name.to_ascii_lowercase();
    headers.iter().rev().find_map(|(header_name, value)| {
        (header_name.to_ascii_lowercase() == name).then(|| value.clone())
    })
}

fn query_string_value(query_strings: &[(String, String)], name: &str) -> Option<String> {
    query_strings
        .iter()
        .rev()
        .find_map(|(query_name, value)| (query_name == name).then(|| value.clone()))
}

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
fn registers_datetime_variable_definition_with_system_default() {
    let interpreter = interpreter_after("def now as datetime", ".");

    assert!(matches!(
        interpreter.variables.get("now"),
        Some(Value::DateTime(_))
    ));
}

#[test]
fn formats_datetime_with_def_format_string() {
    let value = run("def now as datetime\n\
             assert(now.format(\"literal\") == \"literal\")");

    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn reads_and_sets_datetime_parts() {
    let value = run("def now as datetime\n\
             now.day(1)\n\
             now.month(1)\n\
             now.year(2020)\n\
             now.hour(2)\n\
             now.minute(3)\n\
             now.second(4)\n\
             assert(now.day() == 1)\n\
             assert(now.month() == 1)\n\
             assert(now.year() == 2020)\n\
             assert(now.hour() == 2)\n\
             assert(now.minute() == 3)\n\
             assert(now.second() == 4)");

    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn datetime_setters_return_updated_datetime() {
    let value = run("def now as datetime\n\
             def changed as datetime(now.day(1))\n\
             assert(changed.day() == 1)");

    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn datetime_format_rejects_wrong_argument_count() {
    let error = interpret_error("def now as datetime\nnow.format()", ".");

    assert!(
        matches!(error, DefError::Runtime(message) if message.contains("datetime.format expects 1 argument"))
    );
}

#[test]
fn datetime_format_rejects_non_string_argument() {
    let error = interpret_error("def now as datetime\nnow.format(10)", ".");

    assert!(
        matches!(error, DefError::Runtime(message) if message.contains("datetime.format expects a string"))
    );
}

#[test]
fn datetime_part_rejects_wrong_argument_count() {
    let error = interpret_error("def now as datetime\nnow.day(1, 2)", ".");

    assert!(
        matches!(error, DefError::Runtime(message) if message.contains("datetime.day expects 0 or 1 argument"))
    );
}

#[test]
fn datetime_part_rejects_non_integer_setter_value() {
    let error = interpret_error("def now as datetime\nnow.day(\"1\")", ".");

    assert!(
        matches!(error, DefError::Runtime(message) if message.contains("datetime.day expects an integer value"))
    );
}

#[test]
fn datetime_part_rejects_invalid_setter_value() {
    let error = interpret_error("def now as datetime\nnow.month(13)", ".");

    assert!(
        matches!(error, DefError::Runtime(message) if message.contains("invalid datetime.month value 13"))
    );
}

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

#[test]
fn evaluates_array_literal_and_methods() {
    let value = run("def names as array(\"Marcelo\", \"Ana\")\n\
             assert(names.len() == 2 == (names.is_empty() == false) == (names.get(0) == \"Marcelo\"))");

    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn evaluates_array_index_access() {
    let value = run("def names as array(\"Marcelo\", \"Ana\")\n\
             assert(names[0] == \"Marcelo\" == (names[1] == \"Ana\"))");

    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn rejects_array_index_out_of_range() {
    let error = interpret_error("def names as array(\"Marcelo\")\nnames[1]", ".");

    assert!(matches!(error, DefError::Runtime(message) if message.contains("out of range")));
}

#[test]
fn rejects_negative_array_index() {
    let error = interpret_error("def names as array(\"Marcelo\")\nnames[-1]", ".");

    assert!(matches!(error, DefError::Runtime(message) if message.contains("non-negative")));
}

#[test]
fn rejects_non_integer_array_index() {
    let error = interpret_error("def names as array(\"Marcelo\")\nnames[\"0\"]", ".");

    assert!(
        matches!(error, DefError::Runtime(message) if message.contains("index must be an integer"))
    );
}

#[test]
fn rejects_index_access_on_non_array() {
    let error = interpret_error("def name as string(\"Marcelo\")\nname[0]", ".");

    assert!(
        matches!(error, DefError::Runtime(message) if message.contains("only available on array"))
    );
}

#[test]
fn evaluates_tuple_key_and_value() {
    let value = run("def age as tuple(\"Age\", 48)\n\
             assert(age.key() == \"Age\" == (age.value() == 48))");

    assert_eq!(value, Value::Boolean(true));
}

#[test]
fn rejects_tuple_with_wrong_argument_count() {
    let error = interpret_error("def age as tuple(\"Age\")", ".");

    assert!(matches!(error, DefError::Runtime(message) if message.contains("tuple expects 2")));

    let error = interpret_error("def age as tuple(\"Age\", 48, true)", ".");

    assert!(matches!(error, DefError::Runtime(message) if message.contains("tuple expects 2")));
}

#[test]
fn rejects_tuple_with_non_string_key() {
    let error = interpret_error("def age as tuple(10, 48)", ".");

    assert!(
        matches!(error, DefError::Runtime(message) if message.contains("key must be a string"))
    );
}

#[test]
fn evaluates_array_containing_tuples() {
    let value = run("def headers as array(\n\
               tuple(\"Accept\", \"application/json\"),\n\
               tuple(\"Authorization\", \"Bearer token\")\n\
             )\n\
             assert(headers.len() == 2 == (headers.get(0).key() == \"Accept\"))");

    assert_eq!(value, Value::Boolean(true));
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
fn tuple_methods_work_inside_for_loop() {
    let value = run("def headers as array(\n\
               tuple(\"Accept\", \"application/json\"),\n\
               tuple(\"Authorization\", \"Bearer abc123\")\n\
             )\n\
             def last_key as string()\n\
             def last_value as string()\n\
             for header in headers (\n\
               last_key = header.key()\n\
               last_value = header.value()\n\
             )\n\
             assert(last_key == \"Authorization\" == (last_value == \"Bearer abc123\"))");

    assert_eq!(value, Value::Boolean(true));
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
fn array_push_appends_value() {
    let value = run("def items as array\nitems.push(\"one\")\nitems.push(\"two\")\nassert(items.len() == 2 == (items[1] == \"two\"))");

    assert_eq!(value, Value::Boolean(true));
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

#[test]
fn imports_module_functions_and_variables() {
    let value = run_with_base_dir(
        "def math as imported(\"imports/math\")\n\
             math.variable = \"Marcelo\"\n\
             assert(math.add(10, 12) == 22 == (math.variable == \"Marcelo\"))",
        "examples/language",
    );

    assert_eq!(value, Value::Boolean(true));
}

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
    // integers, floats, and booleans are all valid; they are coerced to string
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
    let (path, _dir) = write_temp_file(
        "headers.hdef",
        "Accept: {{accept}}\n",
    );
    let error = interpret_error(
        &format!(
            "def r as request(GET)\n\
             r.path(\"http://127.0.0.1:1\")\n\
             r.headers_from(\"{path}\")\n\
             r.do()"
        ),
        ".",
    );
    assert!(
        matches!(&error, DefError::Runtime(msg) if msg.contains("unresolved template variable") && msg.contains("accept")),
        "unexpected error: {error:?}"
    );
}

#[test]
fn request_do_fails_on_unresolved_body_template() {
    let (path, _dir) = write_temp_file(
        "body.jdef",
        "{\"name\": \"{{username}}\"}\n",
    );
    let error = interpret_error(
        &format!(
            "def r as request(POST)\n\
             r.path(\"http://127.0.0.1:1\")\n\
             r.body_from(\"{path}\")\n\
             r.type(JSON)\n\
             r.do()"
        ),
        ".",
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
