use super::*;

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
