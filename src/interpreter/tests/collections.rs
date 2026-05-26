use super::*;

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
fn array_push_appends_value() {
    let value = run("def items as array\nitems.push(\"one\")\nitems.push(\"two\")\nassert(items.len() == 2 == (items[1] == \"two\"))");
    assert_eq!(value, Value::Boolean(true));
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
