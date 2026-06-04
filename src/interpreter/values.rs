use std::path::{Path, PathBuf};

use crate::ast::{AssignmentOperator, BinaryOperator, MatchPattern, Type, UnaryOperator};
use crate::error::{DefError, DefResult};
use chrono::Local;

use crate::value::{ResponseValue, Value};

pub(super) fn call_integer_method(n: i64, name: &str, args: Vec<Value>) -> DefResult<Value> {
    match name {
        "random" => {
            if args.len() != 2 {
                return Err(DefError::Runtime(format!(
                    "integer.random expects 2 arguments (min, max), got {}",
                    args.len()
                )));
            }
            let min = match &args[0] {
                Value::Integer(v) => *v,
                _ => return Err(DefError::Runtime(
                    "integer.random expects integer arguments".to_string(),
                )),
            };
            let max = match &args[1] {
                Value::Integer(v) => *v,
                _ => return Err(DefError::Runtime(
                    "integer.random expects integer arguments".to_string(),
                )),
            };
            if min > max {
                return Err(DefError::Runtime(format!(
                    "integer.random: min ({min}) must be <= max ({max})"
                )));
            }
            use rand::Rng;
            Ok(Value::Integer(rand::thread_rng().gen_range(min..=max)))
        }
        "to_string" => {
            if !args.is_empty() {
                return Err(DefError::Runtime(format!(
                    "integer.to_string expects 0 arguments, got {}",
                    args.len()
                )));
            }
            Ok(Value::String(n.to_string()))
        }
        _ => Err(DefError::Runtime(format!(
            "undefined integer method '{name}'"
        ))),
    }
}

pub(super) fn call_float_method(f: f64, name: &str, args: Vec<Value>) -> DefResult<Value> {
    match name {
        "random" => {
            if args.len() != 2 {
                return Err(DefError::Runtime(format!(
                    "float.random expects 2 arguments (min, max), got {}",
                    args.len()
                )));
            }
            let min = match &args[0] {
                Value::Float(v) => *v,
                Value::Integer(v) => *v as f64,
                _ => return Err(DefError::Runtime(
                    "float.random expects numeric arguments".to_string(),
                )),
            };
            let max = match &args[1] {
                Value::Float(v) => *v,
                Value::Integer(v) => *v as f64,
                _ => return Err(DefError::Runtime(
                    "float.random expects numeric arguments".to_string(),
                )),
            };
            if min > max {
                return Err(DefError::Runtime(format!(
                    "float.random: min ({min}) must be <= max ({max})"
                )));
            }
            use rand::Rng;
            Ok(Value::Float(rand::thread_rng().gen_range(min..=max)))
        }
        "to_string" => {
            if !args.is_empty() {
                return Err(DefError::Runtime(format!(
                    "float.to_string expects 0 arguments, got {}",
                    args.len()
                )));
            }
            Ok(Value::String(f.to_string()))
        }
        _ => Err(DefError::Runtime(format!(
            "undefined float method '{name}'"
        ))),
    }
}

pub(super) fn call_string_method(s: &str, name: &str, args: Vec<Value>) -> DefResult<Value> {
    match name {
        "from_env_var" => {
            if args.len() != 1 {
                return Err(DefError::Runtime(format!(
                    "from_env_var expects 1 argument, got {}",
                    args.len()
                )));
            }
            let Value::String(var_name) = &args[0] else {
                return Err(DefError::Runtime(
                    "from_env_var expects a string argument".to_string(),
                ));
            };
            std::env::var(var_name).map(Value::String).map_err(|_| {
                DefError::Runtime(format!("environment variable '{var_name}' is not set"))
            })
        }
        _ => Err(DefError::Runtime(format!(
            "undefined string method '{name}' on value {s:?}"
        ))),
    }
}

pub(super) fn printable_value(value: &Value) -> String {
    match value {
        Value::Integer(value) => value.to_string(),
        Value::Float(value) => value.to_string(),
        Value::String(value) => value.clone(),
        Value::Boolean(value) => value.to_string(),
        Value::Array(values) => format!("{values:?}"),
        Value::Tuple { key, value } => format!("tuple({key:?}, {value:?})"),
        Value::DateTime(value) => value.format("%c").to_string(),
        Value::Request(request) => format!("{request:?}"),
        Value::RequestHandle(request) => request.clone(),
        Value::Response(response) => response.body.clone(),
        Value::Mock(mock) => format!("mock({} {})", mock.method, mock.url),
        Value::Uninitialized(type_annotation) => format!("{type_annotation:?}"),
        Value::Nil => "nil".to_string(),
    }
}

pub(super) fn coerce_value_to_type(expected: &Type, value: Value) -> DefResult<Value> {
    match (expected, value) {
        (Type::String, Value::Response(response)) => Ok(Value::String(response.body)),
        (Type::Response, value @ Value::Response(_)) => Ok(value),
        (Type::Array, Value::Array(values)) => Ok(Value::Array(values)),
        (Type::Tuple, value @ Value::Tuple { .. }) => Ok(value),
        (Type::DateTime, value @ Value::DateTime(_)) => Ok(value),
        (Type::Mock, value @ Value::Mock(_)) => Ok(value),
        (expected, value) if value.value_type().as_ref() == Some(expected) => Ok(value),
        (expected, value) => Err(DefError::Runtime(format!(
            "expected value of type {expected:?}, got {value:?}"
        ))),
    }
}

pub(super) fn coerce_assignment(name: &str, current: &Value, value: Value) -> DefResult<Value> {
    let Some(expected) = current.value_type() else {
        return Err(DefError::Runtime(format!(
            "invalid assignment: variable '{name}' has no assignable type"
        )));
    };

    coerce_value_to_type(&expected, value).map_err(|error| match error {
        DefError::Runtime(_) => DefError::Runtime(format!(
            "invalid assignment: variable '{name}' expects {expected:?}"
        )),
        error => error,
    })
}

pub(super) fn apply_assignment_operator(
    name: &str,
    current: Value,
    operator: AssignmentOperator,
    value: Value,
) -> DefResult<Value> {
    match operator {
        AssignmentOperator::Assign => Ok(value),
        AssignmentOperator::AddAssign => {
            evaluate_compound_assignment(name, current, BinaryOperator::Add, value, "+=")
        }
        AssignmentOperator::SubtractAssign => {
            evaluate_compound_assignment(name, current, BinaryOperator::Subtract, value, "-=")
        }
    }
}

pub(super) fn is_valid_tuple_value(value: &Value) -> bool {
    matches!(
        value,
        Value::String(_) | Value::Integer(_) | Value::Float(_) | Value::Boolean(_)
    )
}

fn evaluate_compound_assignment(
    name: &str,
    current: Value,
    operator: BinaryOperator,
    value: Value,
    label: &str,
) -> DefResult<Value> {
    evaluate_numeric_binary(current, operator, value).map_err(|_| {
        DefError::Runtime(format!(
            "invalid compound assignment: variable '{name}' does not support '{label}' with this value"
        ))
    })
}

pub(super) fn evaluate_numeric_binary(
    left: Value,
    operator: BinaryOperator,
    right: Value,
) -> DefResult<Value> {
    match (left, right) {
        (Value::Integer(left), Value::Integer(right)) => {
            let value = match operator {
                BinaryOperator::Add => Value::Integer(left + right),
                BinaryOperator::Subtract => Value::Integer(left - right),
                BinaryOperator::Multiply => Value::Integer(left * right),
                BinaryOperator::Divide => Value::Float(left as f64 / right as f64),
                BinaryOperator::Modulo => Value::Integer(left % right),
                BinaryOperator::Equal => unreachable!("equality is not a numeric operation"),
                BinaryOperator::NotEqual => unreachable!("inequality is not a numeric operation"),
                BinaryOperator::Greater
                | BinaryOperator::GreaterEqual
                | BinaryOperator::Less
                | BinaryOperator::LessEqual => {
                    unreachable!("comparison is not an arithmetic operation")
                }
                BinaryOperator::And | BinaryOperator::Or => {
                    unreachable!("boolean operation is not an arithmetic operation")
                }
            };
            Ok(value)
        }
        (Value::Float(left), Value::Float(right)) => {
            Ok(Value::Float(evaluate_float_binary(left, operator, right)))
        }
        (Value::Integer(left), Value::Float(right)) => Ok(Value::Float(evaluate_float_binary(
            left as f64,
            operator,
            right,
        ))),
        (Value::Float(left), Value::Integer(right)) => Ok(Value::Float(evaluate_float_binary(
            left,
            operator,
            right as f64,
        ))),
        _ => Err(DefError::Runtime(
            "arithmetic operators currently support only integer and float values".to_string(),
        )),
    }
}

pub(super) fn pattern_matches(pattern: &MatchPattern, value: &Value) -> bool {
    match (pattern, value) {
        (MatchPattern::Integer(pattern), Value::Integer(value)) => pattern == value,
        (MatchPattern::Float(pattern), Value::Float(value)) => pattern == value,
        (MatchPattern::String(pattern), Value::String(value)) => pattern == value,
        (MatchPattern::Boolean(pattern), Value::Boolean(value)) => pattern == value,
        (MatchPattern::Wildcard, _) => true,
        _ => false,
    }
}

pub(crate) fn resolve_import_path(base_dir: &Path, import_path: &str) -> PathBuf {
    let import_path = Path::new(import_path);
    let mut path = if import_path.is_absolute() {
        import_path.to_path_buf()
    } else {
        base_dir.join(import_path)
    };

    if path.extension().is_none() {
        path.set_extension("def");
    }

    path
}

fn evaluate_float_binary(left: f64, operator: BinaryOperator, right: f64) -> f64 {
    match operator {
        BinaryOperator::Add => left + right,
        BinaryOperator::Subtract => left - right,
        BinaryOperator::Multiply => left * right,
        BinaryOperator::Divide => left / right,
        BinaryOperator::Modulo => left % right,
        BinaryOperator::Equal => unreachable!("equality is not a numeric operation"),
        BinaryOperator::NotEqual => unreachable!("inequality is not a numeric operation"),
        BinaryOperator::Greater
        | BinaryOperator::GreaterEqual
        | BinaryOperator::Less
        | BinaryOperator::LessEqual => unreachable!("comparison is not an arithmetic operation"),
        BinaryOperator::And | BinaryOperator::Or => {
            unreachable!("boolean operation is not an arithmetic operation")
        }
    }
}

pub(super) fn evaluate_boolean_binary(
    left: Value,
    operator: BinaryOperator,
    right: Value,
) -> DefResult<Value> {
    let (Value::Boolean(left), Value::Boolean(right)) = (left, right) else {
        return Err(DefError::Runtime(
            "boolean operators and and or support only boolean values".to_string(),
        ));
    };

    let result = match operator {
        BinaryOperator::And => left && right,
        BinaryOperator::Or => left || right,
        _ => unreachable!("expected boolean operator"),
    };

    Ok(Value::Boolean(result))
}

pub(super) fn evaluate_unary(operator: UnaryOperator, value: Value) -> DefResult<Value> {
    match operator {
        UnaryOperator::Not => {
            let Value::Boolean(value) = value else {
                return Err(DefError::Runtime(
                    "not operator supports only boolean values".to_string(),
                ));
            };

            Ok(Value::Boolean(!value))
        }
    }
}

pub(super) fn evaluate_numeric_comparison(
    left: Value,
    operator: BinaryOperator,
    right: Value,
) -> DefResult<Value> {
    let (left, right) = match (left, right) {
        (Value::Integer(left), Value::Integer(right)) => (left as f64, right as f64),
        (Value::Float(left), Value::Float(right)) => (left, right),
        (Value::Integer(left), Value::Float(right)) => (left as f64, right),
        (Value::Float(left), Value::Integer(right)) => (left, right as f64),
        _ => {
            return Err(DefError::Runtime(
                "comparison operators >, <, >=, and <= currently support only integer and float values"
                    .to_string(),
            ));
        }
    };

    let result = match operator {
        BinaryOperator::Greater => left > right,
        BinaryOperator::GreaterEqual => left >= right,
        BinaryOperator::Less => left < right,
        BinaryOperator::LessEqual => left <= right,
        _ => unreachable!("expected numeric comparison operator"),
    };

    Ok(Value::Boolean(result))
}

pub(super) fn default_value_for_type(type_annotation: &Type) -> Value {
    match type_annotation {
        Type::Integer => Value::Integer(0),
        Type::Float => Value::Float(0.0),
        Type::String => Value::String(String::new()),
        Type::Boolean => Value::Boolean(false),
        Type::Array => Value::Array(Vec::new()),
        Type::Tuple => Value::Tuple {
            key: String::new(),
            value: Box::new(Value::Nil),
        },
        Type::DateTime => Value::DateTime(Local::now()),
        Type::Request => Value::Uninitialized(Type::Request),
        Type::Mock => Value::Uninitialized(Type::Mock),
        Type::Response => Value::Response(ResponseValue {
            status: 0,
            body: String::new(),
            headers: Vec::new(),
            duration_ms: 0,
            method: String::new(),
            url: String::new(),
        }),
    }
}
