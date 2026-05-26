use super::*;

#[test]
fn from_cmd_param_string_uses_param_when_provided() {
    let interp = interp_with_params(
        "def name as string(from_cmd_param(\"name\", \"default\"))",
        &[("name", "hello")],
    );
    assert_eq!(interp.variables.get("name"), Some(&Value::String("hello".to_string())));
}

#[test]
fn from_cmd_param_string_uses_default_when_param_missing() {
    let interp = interp_with_params(
        "def name as string(from_cmd_param(\"name\", \"default\"))",
        &[],
    );
    assert_eq!(interp.variables.get("name"), Some(&Value::String("default".to_string())));
}

#[test]
fn from_cmd_param_integer_parses_param() {
    let interp = interp_with_params(
        "def count as integer(from_cmd_param(\"count\", 0))",
        &[("count", "42")],
    );
    assert_eq!(interp.variables.get("count"), Some(&Value::Integer(42)));
}

#[test]
fn from_cmd_param_integer_uses_default_when_missing() {
    let interp = interp_with_params(
        "def count as integer(from_cmd_param(\"count\", 99))",
        &[],
    );
    assert_eq!(interp.variables.get("count"), Some(&Value::Integer(99)));
}

#[test]
fn from_cmd_param_float_parses_param() {
    let interp = interp_with_params(
        "def rate as float(from_cmd_param(\"rate\", 0.0))",
        &[("rate", "3.14")],
    );
    assert_eq!(interp.variables.get("rate"), Some(&Value::Float(3.14)));
}

#[test]
fn from_cmd_param_boolean_parses_true() {
    let interp = interp_with_params(
        "def active as boolean(from_cmd_param(\"active\", false))",
        &[("active", "true")],
    );
    assert_eq!(interp.variables.get("active"), Some(&Value::Boolean(true)));
}

#[test]
fn from_cmd_param_boolean_parses_false() {
    let interp = interp_with_params(
        "def active as boolean(from_cmd_param(\"active\", true))",
        &[("active", "false")],
    );
    assert_eq!(interp.variables.get("active"), Some(&Value::Boolean(false)));
}

#[test]
fn from_cmd_param_errors_on_invalid_integer() {
    let error = run_with_params_error(
        "def count as integer(from_cmd_param(\"count\", 0))",
        &[("count", "batata")],
    );
    assert!(matches!(error, DefError::Runtime(m) if m.contains("cannot parse param 'count'") && m.contains("as integer")));
}

#[test]
fn from_cmd_param_errors_on_invalid_boolean() {
    let error = run_with_params_error(
        "def active as boolean(from_cmd_param(\"active\", false))",
        &[("active", "yes")],
    );
    assert!(matches!(error, DefError::Runtime(m) if m.contains("cannot parse param 'active'") && m.contains("as boolean")));
}

#[test]
fn from_cmd_param_errors_when_required_param_missing() {
    let error = run_with_params_error(
        "def cpf as string(from_cmd_param(\"cpf\"))",
        &[],
    );
    assert!(matches!(error, DefError::Runtime(m) if m.contains("required param 'cpf' not provided")));
}

#[test]
fn from_cmd_param_works_in_template_interpolation() {
    let interp = interp_with_params(
        "def cpf as string(from_cmd_param(\"cpf\", \"000.000.000-00\"))\n\
         def msg as string(concat(\"cpf=\", cpf))",
        &[("cpf", "999.000.111-00")],
    );
    assert_eq!(
        interp.variables.get("msg"),
        Some(&Value::String("cpf=999.000.111-00".to_string()))
    );
}
