use std::path::Path;

use crate::error::{DefError, DefResult};
use crate::value::{MockValue, Value};

use super::http::{read_body_file, read_headers_file, render_template};

pub(super) fn call_mock_method(
    mut mock: MockValue,
    name: &str,
    args: Vec<Value>,
    base_dir: &Path,
) -> DefResult<Value> {
    match name {
        "reply" | "fail" => {
            match args.as_slice() {
                [Value::Integer(status)] => {
                    mock.status = *status;
                    mock.configured = true;
                }
                [Value::Integer(status), Value::String(body)] => {
                    mock.status = *status;
                    mock.body = body.clone();
                    mock.configured = true;
                }
                _ => {
                    return Err(DefError::Runtime(format!(
                        "mock.{name} expects (status) or (status, body)"
                    )))
                }
            }
            Ok(Value::Mock(mock))
        }
        "header" => {
            let (header_name, value) = mock_header_from_args(args)?;
            set_mock_header(&mut mock.headers, &header_name, &value)?;
            Ok(Value::Mock(mock))
        }
        "headers_from" => {
            let path = single_string_arg("mock.headers_from", args)?;
            for (header_name, value) in read_headers_file(base_dir, &path)? {
                let value = render_template(&value, &mock.vars);
                set_mock_header(&mut mock.headers, &header_name, &value)?;
            }
            Ok(Value::Mock(mock))
        }
        "body_from" => {
            let path = single_string_arg("mock.body_from", args)?;
            let raw = read_body_file(base_dir, &path)?;
            mock.body = render_template(&raw, &mock.vars);
            mock.configured = true;
            Ok(Value::Mock(mock))
        }
        "with_var" => {
            let (var_name, value) = mock_var_from_tuple_arg(args)?;
            set_mock_var(&mut mock.vars, &var_name, &value);
            apply_mock_vars_to_headers(&mut mock.headers, &mock.vars);
            apply_mock_vars_to_body(&mut mock.body, &mock.vars);
            Ok(Value::Mock(mock))
        }
        "delay" => match args.as_slice() {
            [Value::Integer(ms)] => {
                if *ms < 0 {
                    return Err(DefError::Runtime(
                        "mock.delay expects a non-negative integer".to_string(),
                    ));
                }
                mock.delay_ms = *ms as u64;
                Ok(Value::Mock(mock))
            }
            _ => Err(DefError::Runtime(
                "mock.delay expects 1 integer argument in milliseconds".to_string(),
            )),
        },
        // Stores a snapshot path for future replay support in `def server`.
        "from_snapshot" => {
            let path = single_string_arg("mock.from_snapshot", args)?;
            mock.snapshot_path = Some(path);
            mock.configured = true;
            Ok(Value::Mock(mock))
        }
        _ => Err(DefError::Runtime(format!("unknown mock method '{name}'"))),
    }
}

fn set_mock_header(
    headers: &mut Vec<(String, String)>,
    name: &str,
    value: &str,
) -> DefResult<()> {
    let name = name.trim();
    if name.is_empty() {
        return Err(DefError::Runtime(
            "mock header name cannot be empty".to_string(),
        ));
    }
    let lowercased = name.to_ascii_lowercase();
    headers.retain(|(current, _)| *current != lowercased);
    headers.push((lowercased, value.to_string()));
    Ok(())
}

fn set_mock_var(vars: &mut Vec<(String, String)>, name: &str, value: &str) {
    vars.retain(|(current, _)| current != name);
    vars.push((name.to_string(), value.to_string()));
}

fn apply_mock_vars_to_headers(headers: &mut [(String, String)], vars: &[(String, String)]) {
    for (_, value) in headers.iter_mut() {
        *value = render_template(value, vars);
    }
}

fn apply_mock_vars_to_body(body: &mut String, vars: &[(String, String)]) {
    *body = render_template(body, vars);
}

fn mock_header_from_args(args: Vec<Value>) -> DefResult<(String, String)> {
    match args.as_slice() {
        [Value::Tuple { key, value }] => {
            let Value::String(value) = value.as_ref() else {
                return Err(DefError::Runtime(
                    "mock.header tuple value must be a string".to_string(),
                ));
            };
            Ok((key.clone(), value.clone()))
        }
        [Value::String(name), Value::String(value)] => Ok((name.clone(), value.clone())),
        [_] => Err(DefError::Runtime(
            "mock.header expects a tuple(\"Name\", \"value\") argument".to_string(),
        )),
        _ => Err(DefError::Runtime(format!(
            "mock.header expects 1 or 2 string arguments, got {}",
            args.len()
        ))),
    }
}

fn mock_var_from_tuple_arg(args: Vec<Value>) -> DefResult<(String, String)> {
    match args.as_slice() {
        [Value::Tuple { key, value }] => {
            let string_value = primitive_to_string(value.as_ref())?;
            Ok((key.clone(), string_value))
        }
        _ => Err(DefError::Runtime(
            "mock.with_var expects a variable identifier".to_string(),
        )),
    }
}

fn primitive_to_string(value: &Value) -> DefResult<String> {
    match value {
        Value::String(s) => Ok(s.clone()),
        Value::Integer(n) => Ok(n.to_string()),
        Value::Float(f) => Ok(f.to_string()),
        Value::Boolean(b) => Ok(b.to_string()),
        _ => Err(DefError::Runtime(
            "mock.with_var value must be a primitive (string, integer, float, or boolean)"
                .to_string(),
        )),
    }
}

fn single_string_arg(method: &str, args: Vec<Value>) -> DefResult<String> {
    match args.as_slice() {
        [Value::String(path)] => Ok(path.clone()),
        [_] => Err(DefError::Runtime(format!(
            "{method} expects a string path argument"
        ))),
        _ => Err(DefError::Runtime(format!(
            "{method} expects 1 argument, got {}",
            args.len()
        ))),
    }
}
