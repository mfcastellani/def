use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::error::{DefError, DefResult};
use crate::value::{RequestValue, ResponseValue, Value};

pub(super) fn new_request_value(method: &str) -> Value {
    Value::Request(RequestValue {
        method: method.to_ascii_uppercase(),
        path: None,
        status: None,
        headers: Vec::new(),
        query_strings: Vec::new(),
        body: None,
        vars: Vec::new(),
    })
}

pub(super) enum RequestMethodResult {
    Request,
    Value(Value),
}

pub(super) fn apply_request_method(
    request: &mut RequestValue,
    name: &str,
    args: Vec<Value>,
    base_dir: &Path,
) -> DefResult<RequestMethodResult> {
    match name {
        "path" => {
            if args.len() != 1 {
                return Err(DefError::Runtime(format!(
                    "request.path expects 1 argument, got {}",
                    args.len()
                )));
            }

            let Value::String(path) = &args[0] else {
                return Err(DefError::Runtime(
                    "request.path expects a string URL".to_string(),
                ));
            };

            request.path = Some(path.clone());
            Ok(RequestMethodResult::Request)
        }
        "header" => {
            let (name, value) = request_header_from_args(args)?;
            set_request_header(&mut request.headers, &name, &value)?;
            Ok(RequestMethodResult::Request)
        }
        "headers_from" => {
            if args.len() != 1 {
                return Err(DefError::Runtime(format!(
                    "request.headers_from expects 1 argument, got {}",
                    args.len()
                )));
            }

            let Value::String(path) = &args[0] else {
                return Err(DefError::Runtime(
                    "request.headers_from expects a string path".to_string(),
                ));
            };

            for (name, value) in read_headers_file(base_dir, path)? {
                let value = render_template(&value, &request.vars);
                set_request_header(&mut request.headers, &name, &value)?;
            }

            Ok(RequestMethodResult::Request)
        }
        "query_string" => {
            let (name, value) = request_query_string_from_args(args)?;
            set_request_query_string(&mut request.query_strings, &name, &value)?;
            Ok(RequestMethodResult::Request)
        }
        "query_string_from" => {
            if args.len() != 1 {
                return Err(DefError::Runtime(format!(
                    "request.query_string_from expects 1 argument, got {}",
                    args.len()
                )));
            }

            let Value::String(path) = &args[0] else {
                return Err(DefError::Runtime(
                    "request.query_string_from expects a string path".to_string(),
                ));
            };

            for (name, value) in read_query_string_file(base_dir, path)? {
                let value = render_template(&value, &request.vars);
                set_request_query_string(&mut request.query_strings, &name, &value)?;
            }

            Ok(RequestMethodResult::Request)
        }
        "body_from" => {
            if args.len() != 1 {
                return Err(DefError::Runtime(format!(
                    "request.body_from expects 1 argument, got {}",
                    args.len()
                )));
            }

            let Value::String(path) = &args[0] else {
                return Err(DefError::Runtime(
                    "request.body_from expects a string path".to_string(),
                ));
            };

            let raw = read_body_file(base_dir, path)?;
            request.body = Some(render_template(&raw, &request.vars));
            Ok(RequestMethodResult::Request)
        }
        "type" => {
            if args.len() != 1 {
                return Err(DefError::Runtime(format!(
                    "request.type expects 1 argument (JSON or TEXT), got {}",
                    args.len()
                )));
            }

            let Value::String(body_type) = &args[0] else {
                return Err(DefError::Runtime(
                    "request.type expects JSON or TEXT".to_string(),
                ));
            };

            let content_type = match body_type.to_ascii_uppercase().as_str() {
                "JSON" => "application/json",
                "TEXT" => "text/plain",
                other => {
                    return Err(DefError::Runtime(format!(
                        "request.type expects JSON or TEXT, got '{other}'"
                    )))
                }
            };

            set_request_header(&mut request.headers, "Content-Type", content_type)?;
            Ok(RequestMethodResult::Request)
        }
        "with_var" => {
            let (name, value) = request_var_from_args(args)?;
            set_request_var(&mut request.vars, &name, &value)?;
            apply_request_vars_to_headers(&mut request.headers, &request.vars);
            apply_request_vars_to_query_strings(&mut request.query_strings, &request.vars);
            apply_request_vars_to_body(&mut request.body, &request.vars);
            Ok(RequestMethodResult::Request)
        }
        "do" => {
            if !args.is_empty() {
                return Err(DefError::Runtime(format!(
                    "request.do expects 0 arguments, got {}",
                    args.len()
                )));
            }

            execute_http_request(request).map(RequestMethodResult::Value)
        }
        "status" => {
            if !args.is_empty() {
                return Err(DefError::Runtime(format!(
                    "request.status expects 0 arguments, got {}",
                    args.len()
                )));
            }

            request
                .status
                .map(Value::Integer)
                .map(RequestMethodResult::Value)
                .ok_or_else(|| DefError::Runtime("request has not been executed".to_string()))
        }
        _ => Err(DefError::Runtime(format!(
            "unknown request method '{name}'"
        ))),
    }
}

fn set_request_header(
    headers: &mut Vec<(String, String)>,
    name: &str,
    value: &str,
) -> DefResult<()> {
    let name = name.trim();
    if name.is_empty() {
        return Err(DefError::Runtime(
            "request header name cannot be empty".to_string(),
        ));
    }

    let normalized = name.to_ascii_lowercase();
    headers.retain(|(current, _)| current.to_ascii_lowercase() != normalized);
    headers.push((name.to_string(), value.to_string()));
    Ok(())
}

fn request_header_from_args(args: Vec<Value>) -> DefResult<(String, String)> {
    match args.as_slice() {
        [Value::Tuple { key, value }] => {
            let Value::String(value) = value.as_ref() else {
                return Err(DefError::Runtime(
                    "request.header tuple value must be a string".to_string(),
                ));
            };

            Ok((key.clone(), value.clone()))
        }
        [Value::String(name), Value::String(value)] => Ok((name.clone(), value.clone())),
        [_] => Err(DefError::Runtime(
            "request.header expects a tuple(\"Name\", \"value\") argument".to_string(),
        )),
        _ => Err(DefError::Runtime(format!(
            "request.header expects 1 tuple argument, got {}",
            args.len()
        ))),
    }
}

fn set_request_query_string(
    query_strings: &mut Vec<(String, String)>,
    name: &str,
    value: &str,
) -> DefResult<()> {
    let name = name.trim();
    if name.is_empty() {
        return Err(DefError::Runtime(
            "request query string name cannot be empty".to_string(),
        ));
    }

    query_strings.retain(|(current, _)| current != name);
    query_strings.push((name.to_string(), value.to_string()));
    Ok(())
}

fn request_query_string_from_args(args: Vec<Value>) -> DefResult<(String, String)> {
    match args.as_slice() {
        [Value::Tuple { key, value }] => {
            let Value::String(value) = value.as_ref() else {
                return Err(DefError::Runtime(
                    "request.query_string tuple value must be a string".to_string(),
                ));
            };

            Ok((key.clone(), value.clone()))
        }
        [_] => Err(DefError::Runtime(
            "request.query_string expects a tuple(\"name\", \"value\") argument".to_string(),
        )),
        _ => Err(DefError::Runtime(format!(
            "request.query_string expects 1 tuple argument, got {}",
            args.len()
        ))),
    }
}

fn request_var_from_args(args: Vec<Value>) -> DefResult<(String, String)> {
    match args.as_slice() {
        [Value::Tuple { key, value }] => {
            let Value::String(value) = value.as_ref() else {
                return Err(DefError::Runtime(
                    "request.with_var variable value must be a string".to_string(),
                ));
            };

            Ok((key.clone(), value.clone()))
        }
        _ => Err(DefError::Runtime(format!(
            "request.with_var expects 1 variable identifier, got {}",
            args.len()
        ))),
    }
}

fn set_request_var(vars: &mut Vec<(String, String)>, name: &str, value: &str) -> DefResult<()> {
    let name = name.trim();
    if name.is_empty() {
        return Err(DefError::Runtime(
            "request.with_var variable name cannot be empty".to_string(),
        ));
    }

    vars.retain(|(current, _)| current != name);
    vars.push((name.to_string(), value.to_string()));
    Ok(())
}

fn apply_request_vars_to_headers(headers: &mut [(String, String)], vars: &[(String, String)]) {
    for (_, value) in headers {
        *value = render_template(value, vars);
    }
}

fn apply_request_vars_to_query_strings(
    query_strings: &mut [(String, String)],
    vars: &[(String, String)],
) {
    for (_, value) in query_strings {
        *value = render_template(value, vars);
    }
}

fn apply_request_vars_to_body(body: &mut Option<String>, vars: &[(String, String)]) {
    if let Some(body) = body {
        *body = render_template(body, vars);
    }
}

fn render_template(value: &str, vars: &[(String, String)]) -> String {
    let mut rendered = value.to_string();
    for (name, replacement) in vars {
        rendered = rendered.replace(&format!("{{{{{name}}}}}"), replacement);
    }
    rendered
}

fn read_headers_file(base_dir: &Path, header_path: &str) -> DefResult<Vec<(String, String)>> {
    let path = resolve_path(base_dir, header_path);
    let source = fs::read_to_string(&path).map_err(|error| {
        DefError::Runtime(format!(
            "request.headers_from failed to read '{}': {error}",
            path.display()
        ))
    })?;

    parse_headers_source(&source, &path.display().to_string())
}

fn read_body_file(base_dir: &Path, body_path: &str) -> DefResult<String> {
    let path = resolve_path(base_dir, body_path);
    fs::read_to_string(&path).map_err(|error| {
        DefError::Runtime(format!(
            "request.body_from failed to read '{}': {error}",
            path.display()
        ))
    })
}

fn read_query_string_file(
    base_dir: &Path,
    query_string_path: &str,
) -> DefResult<Vec<(String, String)>> {
    let path = resolve_path(base_dir, query_string_path);
    let source = fs::read_to_string(&path).map_err(|error| {
        DefError::Runtime(format!(
            "request.query_string_from failed to read '{}': {error}",
            path.display()
        ))
    })?;

    parse_query_string_source(&source, &path.display().to_string())
}

fn parse_headers_source(source: &str, context: &str) -> DefResult<Vec<(String, String)>> {
    let mut headers = Vec::new();

    for (index, line) in source.lines().enumerate() {
        let line_number = index + 1;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with('#') {
            continue;
        }

        let Some((name, value)) = line.split_once(':') else {
            return Err(DefError::Runtime(format!(
                "invalid header line {line_number} in '{context}': expected 'Name: value'"
            )));
        };

        let name = name.trim();
        if name.is_empty() {
            return Err(DefError::Runtime(format!(
                "invalid header line {line_number} in '{context}': header name cannot be empty"
            )));
        }

        headers.push((name.to_string(), value.trim().to_string()));
    }

    Ok(headers)
}

fn parse_query_string_source(source: &str, context: &str) -> DefResult<Vec<(String, String)>> {
    let mut query_strings = Vec::new();

    for (index, line) in source.lines().enumerate() {
        let line_number = index + 1;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with('#') {
            continue;
        }

        let Some((name, value)) = line.split_once(':') else {
            return Err(DefError::Runtime(format!(
                "invalid query string line {line_number} in '{context}': expected 'Name: value'"
            )));
        };

        let name = name.trim();
        if name.is_empty() {
            return Err(DefError::Runtime(format!(
                "invalid query string line {line_number} in '{context}': query string name cannot be empty"
            )));
        }

        query_strings.push((name.to_string(), value.trim().to_string()));
    }

    Ok(query_strings)
}

fn resolve_path(base_dir: &Path, path: &str) -> PathBuf {
    let path = Path::new(path);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base_dir.join(path)
    }
}

fn execute_http_request(request: &mut RequestValue) -> DefResult<Value> {
    let path = request
        .path
        .as_ref()
        .ok_or_else(|| DefError::Runtime("request.path must be set before request.do".to_string()))?
        .clone();

    let mut http_request = ureq::request(&request.method, &path);
    for (name, value) in &request.query_strings {
        http_request = http_request.query(name, value);
    }
    for (name, value) in &request.headers {
        http_request = http_request.set(name, value);
    }

    let start = std::time::Instant::now();
    let response = match &request.body {
        Some(body) => http_request.send_string(body),
        None => http_request.call(),
    }
    .map_err(|error| DefError::Runtime(format!("request failed: {error}")))?;
    let duration_ms = start.elapsed().as_millis() as i64;

    let status = i64::from(response.status());
    request.status = Some(status);
    let headers = response
        .headers_names()
        .into_iter()
        .filter_map(|name| {
            response
                .header(&name)
                .map(|value| (name.to_ascii_lowercase(), value.to_string()))
        })
        .collect();

    let body = response
        .into_string()
        .map_err(|error| DefError::Runtime(format!("failed to read response body: {error}")))?;

    Ok(Value::Response(ResponseValue {
        status,
        body,
        headers,
        duration_ms,
    }))
}

pub(super) fn call_response_method(
    response: ResponseValue,
    name: &str,
    args: Vec<Value>,
) -> DefResult<Value> {
    match name {
        "body" => {
            if !args.is_empty() {
                return Err(DefError::Runtime(format!(
                    "response.body expects 0 arguments, got {}",
                    args.len()
                )));
            }

            Ok(Value::String(response.body))
        }
        "status" => {
            if !args.is_empty() {
                return Err(DefError::Runtime(format!(
                    "response.status expects 0 arguments, got {}",
                    args.len()
                )));
            }

            Ok(Value::Integer(response.status))
        }
        "headers" => {
            if !args.is_empty() {
                return Err(DefError::Runtime(format!(
                    "response.headers expects 0 arguments, got {}",
                    args.len()
                )));
            }

            Ok(Value::Array(
                response
                    .headers
                    .into_iter()
                    .map(|(name, value)| Value::String(format!("{name}: {value}")))
                    .collect(),
            ))
        }
        "header" => {
            if args.len() != 1 {
                return Err(DefError::Runtime(format!(
                    "response.header expects 1 argument, got {}",
                    args.len()
                )));
            }

            let Value::String(name) = &args[0] else {
                return Err(DefError::Runtime(
                    "response.header expects a string header name".to_string(),
                ));
            };

            let name = name.to_ascii_lowercase();
            Ok(Value::String(
                response
                    .headers
                    .into_iter()
                    .find_map(|(header_name, value)| (header_name == name).then_some(value))
                    .unwrap_or_default(),
            ))
        }
        "duration" => {
            if !args.is_empty() {
                return Err(DefError::Runtime(format!(
                    "response.duration expects 0 arguments, got {}",
                    args.len()
                )));
            }

            Ok(Value::Integer(response.duration_ms))
        }
        "ok" => {
            if !args.is_empty() {
                return Err(DefError::Runtime(format!(
                    "response.ok expects 0 arguments, got {}",
                    args.len()
                )));
            }

            Ok(Value::Boolean(
                response.status >= 200 && response.status < 300,
            ))
        }
        "size" => {
            if !args.is_empty() {
                return Err(DefError::Runtime(format!(
                    "response.size expects 0 arguments, got {}",
                    args.len()
                )));
            }

            Ok(Value::Integer(response.body.len() as i64))
        }
        "content_type" => {
            if !args.is_empty() {
                return Err(DefError::Runtime(format!(
                    "response.content_type expects 0 arguments, got {}",
                    args.len()
                )));
            }

            let ct = response
                .headers
                .iter()
                .find_map(|(name, value)| {
                    (name.to_ascii_lowercase() == "content-type").then(|| value.clone())
                })
                .unwrap_or_default();

            Ok(Value::String(ct))
        }
        "body_contains" => {
            if args.len() != 1 {
                return Err(DefError::Runtime(format!(
                    "response.body_contains expects 1 argument, got {}",
                    args.len()
                )));
            }

            let Value::String(search) = &args[0] else {
                return Err(DefError::Runtime(
                    "response.body_contains expects a string argument".to_string(),
                ));
            };

            Ok(Value::Boolean(response.body.contains(search.as_str())))
        }
        "describe_status" => {
            if !args.is_empty() {
                return Err(DefError::Runtime(format!(
                    "response.describe_status expects 0 arguments, got {}",
                    args.len()
                )));
            }

            let label = match response.status {
                100 => "100 Continue",
                101 => "101 Switching Protocols",
                200 => "200 OK",
                201 => "201 Created",
                202 => "202 Accepted",
                204 => "204 No Content",
                206 => "206 Partial Content",
                301 => "301 Moved Permanently",
                302 => "302 Found",
                304 => "304 Not Modified",
                307 => "307 Temporary Redirect",
                308 => "308 Permanent Redirect",
                400 => "400 Bad Request",
                401 => "401 Unauthorized",
                403 => "403 Forbidden",
                404 => "404 Not Found",
                405 => "405 Method Not Allowed",
                408 => "408 Request Timeout",
                409 => "409 Conflict",
                410 => "410 Gone",
                422 => "422 Unprocessable Entity",
                429 => "429 Too Many Requests",
                500 => "500 Internal Server Error",
                501 => "501 Not Implemented",
                502 => "502 Bad Gateway",
                503 => "503 Service Unavailable",
                504 => "504 Gateway Timeout",
                _   => "unknown",
            };

            Ok(Value::String(label.to_string()))
        }
        _ => Err(DefError::Runtime(format!(
            "unknown response method '{name}'"
        ))),
    }
}
