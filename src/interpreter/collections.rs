use std::collections::HashMap;

use crate::error::{DefError, DefResult};
use crate::value::Value;

use super::ScopeStack;

pub(super) fn call_array_method(
    items: Vec<Value>,
    name: &str,
    args: Vec<Value>,
) -> DefResult<Value> {
    let mut items = items;
    match name {
        "len" => {
            if !args.is_empty() {
                return Err(DefError::Runtime(format!(
                    "array.len expects 0 arguments, got {}",
                    args.len()
                )));
            }

            Ok(Value::Integer(items.len() as i64))
        }
        "is_empty" => {
            if !args.is_empty() {
                return Err(DefError::Runtime(format!(
                    "array.is_empty expects 0 arguments, got {}",
                    args.len()
                )));
            }

            Ok(Value::Boolean(items.is_empty()))
        }
        "get" => {
            if args.len() != 1 {
                return Err(DefError::Runtime(format!(
                    "array.get expects 1 argument, got {}",
                    args.len()
                )));
            }

            get_array_index(Value::Array(items), args[0].clone())
        }
        "push" => {
            if args.len() != 1 {
                return Err(DefError::Runtime(format!(
                    "array.push expects 1 argument, got {}",
                    args.len()
                )));
            }

            items.push(args[0].clone());
            Ok(Value::Array(items))
        }
        _ => Err(DefError::Runtime(format!("unknown array method '{name}'"))),
    }
}

pub(super) fn call_array_push_on_scoped_variable(
    scopes: &mut ScopeStack,
    name: &str,
    args: &[Value],
) -> DefResult<Option<Value>> {
    for scope in scopes.iter_mut().rev() {
        if let Some(value) = scope.get_mut(name) {
            return push_to_array_value(value, args).map(Some);
        }
    }

    Ok(None)
}

pub(super) fn call_array_push_on_global_variable(
    globals: &mut HashMap<String, Value>,
    name: &str,
    args: &[Value],
) -> DefResult<Option<Value>> {
    if let Some(value) = globals.get_mut(name) {
        return push_to_array_value(value, args).map(Some);
    }

    Ok(None)
}

fn push_to_array_value(value: &mut Value, args: &[Value]) -> DefResult<Value> {
    let Value::Array(items) = value else {
        return Err(DefError::Runtime(
            "array.push is only available on array values".to_string(),
        ));
    };

    if args.len() != 1 {
        return Err(DefError::Runtime(format!(
            "array.push expects 1 argument, got {}",
            args.len()
        )));
    }

    items.push(args[0].clone());
    Ok(Value::Array(items.clone()))
}

pub(super) fn call_tuple_method(
    key: String,
    value: Value,
    name: &str,
    args: Vec<Value>,
) -> DefResult<Value> {
    match name {
        "key" => {
            if !args.is_empty() {
                return Err(DefError::Runtime(format!(
                    "tuple.key expects 0 arguments, got {}",
                    args.len()
                )));
            }

            Ok(Value::String(key))
        }
        "value" => {
            if !args.is_empty() {
                return Err(DefError::Runtime(format!(
                    "tuple.value expects 0 arguments, got {}",
                    args.len()
                )));
            }

            Ok(value)
        }
        _ => Err(DefError::Runtime(format!("unknown tuple method '{name}'"))),
    }
}

pub(super) fn get_array_index(object: Value, index: Value) -> DefResult<Value> {
    let Value::Array(items) = object else {
        return Err(DefError::Runtime(
            "index access is only available on array values".to_string(),
        ));
    };

    let Value::Integer(index) = index else {
        return Err(DefError::Runtime(
            "array index must be an integer".to_string(),
        ));
    };

    if index < 0 {
        return Err(DefError::Runtime(format!(
            "array index must be non-negative, got {index}"
        )));
    }

    items.get(index as usize).cloned().ok_or_else(|| {
        DefError::Runtime(format!(
            "array index {index} is out of range for length {}",
            items.len()
        ))
    })
}
