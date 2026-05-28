use std::{collections::HashMap, thread, time::Duration};

use crate::ast::{Expression, FunctionDefinition, Statement, Stmt};
use crate::error::{DefError, DefResult};
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::value::Value;

use super::{coerce_value_to_type, new_request_value, printable_value, Interpreter, ScopeStack};

impl Interpreter {
    pub(super) fn call_function(
        &mut self,
        name: &str,
        args: &[Expression],
        scopes: &mut ScopeStack,
    ) -> DefResult<Value> {
        if name == "assert" {
            return self.call_assert(args, scopes);
        }
        if name == "delay" {
            return self.call_delay(args, scopes);
        }
        if name == "print" {
            return self.call_print(args, scopes);
        }
        if name == "concat" {
            return self.call_concat(args, scopes);
        }
        if name == "from_cmd_param" {
            return self.call_from_cmd_param(args, scopes);
        }
        if name == "break" {
            if !args.is_empty() {
                return Err(DefError::Runtime("break() takes no arguments".to_string()));
            }
            return Err(DefError::LoopBreak);
        }
        if name == "next" {
            if !args.is_empty() {
                return Err(DefError::Runtime("next() takes no arguments".to_string()));
            }
            return Err(DefError::LoopNext);
        }

        let function = self
            .functions
            .get(name)
            .cloned()
            .ok_or_else(|| DefError::Runtime(format!("undefined function '{name}'")))?;

        let mut values = Vec::with_capacity(args.len());
        for arg in args {
            values.push(self.evaluate_expression(arg, scopes)?);
        }

        self.call_user_function(function, values)
    }

    pub(super) fn call_user_function_with_values(
        &mut self,
        name: &str,
        values: Vec<Value>,
    ) -> DefResult<Value> {
        let function = self
            .functions
            .get(name)
            .cloned()
            .ok_or_else(|| DefError::Runtime(format!("undefined function '{name}'")))?;

        self.call_user_function(function, values)
    }

    fn call_user_function(
        &mut self,
        function: FunctionDefinition,
        values: Vec<Value>,
    ) -> DefResult<Value> {
        if function.params.len() != values.len() {
            return Err(DefError::Runtime(format!(
                "function '{}' expects {} arguments, got {}",
                function.name,
                function.params.len(),
                values.len()
            )));
        }

        let mut function_scope = HashMap::new();
        for (param, value) in function.params.iter().zip(values) {
            let value = coerce_value_to_type(&param.type_annotation, value)?;
            function_scope.insert(param.name.clone(), value);
        }

        let mut function_scopes = vec![function_scope];
        let mut last_value = Value::Nil;
        for stmt in &function.body {
            last_value = self
                .execute_statement(&stmt.inner, &mut function_scopes)
                .map_err(|e| match e {
                    DefError::LoopBreak => DefError::Runtime(
                        "break() called outside of a loop".to_string(),
                    ),
                    DefError::LoopNext => DefError::Runtime(
                        "next() called outside of a loop".to_string(),
                    ),
                    other => other,
                })?;
        }

        Ok(last_value)
    }

    fn call_assert(&mut self, args: &[Expression], scopes: &mut ScopeStack) -> DefResult<Value> {
        if args.len() != 1 {
            return Err(DefError::Runtime(format!(
                "assert expects 1 boolean expression, got {}",
                args.len()
            )));
        }

        let value = self.evaluate_expression(&args[0], scopes)?;
        let Value::Boolean(value) = value else {
            return Err(DefError::Runtime(
                "assert expects a boolean expression".to_string(),
            ));
        };

        if !value {
            return Err(DefError::Runtime("assertion failed".to_string()));
        }

        Ok(Value::Boolean(value))
    }

    fn call_delay(&mut self, args: &[Expression], scopes: &mut ScopeStack) -> DefResult<Value> {
        if args.len() != 1 {
            return Err(DefError::Runtime(format!(
                "delay expects 1 argument, got {}",
                args.len()
            )));
        }

        if self.dry_run {
            return Ok(Value::Nil);
        }

        let milliseconds = self.evaluate_expression(&args[0], scopes)?;
        let Value::Integer(milliseconds) = milliseconds else {
            return Err(DefError::Runtime(
                "delay expects an integer argument in milliseconds".to_string(),
            ));
        };

        if milliseconds < 0 {
            return Err(DefError::Runtime(
                "delay expects a non-negative integer".to_string(),
            ));
        }

        thread::sleep(Duration::from_millis(milliseconds as u64));
        Ok(Value::Nil)
    }

    fn call_print(&mut self, args: &[Expression], scopes: &mut ScopeStack) -> DefResult<Value> {
        if args.len() != 1 {
            return Err(DefError::Runtime(format!(
                "print expects 1 argument, got {}",
                args.len()
            )));
        }

        if self.dry_run {
            return Ok(Value::Nil);
        }

        if let Expression::String(template) = &args[0] {
            if template.contains("{{") {
                let template = template.clone();
                let output = self.interpolate_print_template(&template, scopes)?;
                println!("{output}");
                return Ok(Value::Nil);
            }
        }

        let value = self.evaluate_expression(&args[0], scopes)?;
        println!("{}", printable_value(&value));
        Ok(Value::Nil)
    }

    fn interpolate_print_template(
        &mut self,
        template: &str,
        scopes: &mut ScopeStack,
    ) -> DefResult<String> {
        let mut output = String::new();
        let mut remaining = template;

        while let Some(open) = remaining.find("{{") {
            output.push_str(&remaining[..open]);
            remaining = &remaining[open + 2..];

            let Some(close) = remaining.find("}}") else {
                return Err(DefError::Runtime(
                    "unclosed '{{' in print template".to_string(),
                ));
            };

            let expr_src = remaining[..close].trim();
            remaining = &remaining[close + 2..];

            let mut lexer = Lexer::new(expr_src);
            let tokens = lexer.tokenize().map_err(|e| {
                DefError::Runtime(format!("invalid expression in print template: {e}"))
            })?;
            let program = Parser::new(tokens).parse_program().map_err(|e| {
                DefError::Runtime(format!("invalid expression in print template: {e}"))
            })?;

            let Some(Stmt {
                inner: Statement::Expression(expression),
                ..
            }) = program.statements.into_iter().next()
            else {
                return Err(DefError::Runtime(
                    "print template placeholder must be an expression".to_string(),
                ));
            };

            let value = self.evaluate_expression(&expression, scopes)?;
            output.push_str(&printable_value(&value));
        }

        output.push_str(remaining);
        Ok(output)
    }

    fn call_concat(&mut self, args: &[Expression], scopes: &mut ScopeStack) -> DefResult<Value> {
        if args.len() < 2 {
            return Err(DefError::Runtime(format!(
                "concat expects at least 2 arguments, got {}",
                args.len()
            )));
        }

        let mut output = String::new();
        for arg in args {
            let value = self.evaluate_expression(arg, scopes)?;
            let Value::String(value) = value else {
                return Err(DefError::Runtime(
                    "concat expects only string arguments".to_string(),
                ));
            };
            output.push_str(&value);
        }

        Ok(Value::String(output))
    }

    fn call_from_cmd_param(
        &mut self,
        args: &[Expression],
        scopes: &mut ScopeStack,
    ) -> DefResult<Value> {
        if args.is_empty() || args.len() > 2 {
            return Err(DefError::Runtime(format!(
                "from_cmd_param expects 1 or 2 arguments, got {}",
                args.len()
            )));
        }

        let name_val = self.evaluate_expression(&args[0], scopes)?;
        let Value::String(param_name) = name_val else {
            return Err(DefError::Runtime(
                "from_cmd_param expects a string param name as the first argument".to_string(),
            ));
        };

        let default = if args.len() == 2 {
            Some(self.evaluate_expression(&args[1], scopes)?)
        } else {
            None
        };

        if let Some(raw) = self.params.get(&param_name).cloned() {
            return match &default {
                None => Ok(Value::String(raw)),
                Some(def) => parse_cmd_param(def, &param_name, &raw),
            };
        }

        match default {
            Some(def) => Ok(def),
            None => Err(DefError::Runtime(format!(
                "required param '{param_name}' not provided — pass --param {param_name}=<value>"
            ))),
        }
    }
}

fn parse_cmd_param(default: &Value, param_name: &str, raw: &str) -> DefResult<Value> {
    match default {
        Value::String(_) => Ok(Value::String(raw.to_string())),
        Value::Integer(_) => raw.parse::<i64>().map(Value::Integer).map_err(|_| {
            DefError::Runtime(format!(
                "cannot parse param '{param_name}' value \"{raw}\" as integer"
            ))
        }),
        Value::Float(_) => raw.parse::<f64>().map(Value::Float).map_err(|_| {
            DefError::Runtime(format!(
                "cannot parse param '{param_name}' value \"{raw}\" as float"
            ))
        }),
        Value::Boolean(_) => match raw {
            "true" => Ok(Value::Boolean(true)),
            "false" => Ok(Value::Boolean(false)),
            _ => Err(DefError::Runtime(format!(
                "cannot parse param '{param_name}' value \"{raw}\" as boolean: expected true or false"
            ))),
        },
        Value::DateTime(_) => {
            use chrono::{DateTime, NaiveDate, TimeZone as _};
            if let Ok(dt) = DateTime::parse_from_rfc3339(raw) {
                return Ok(Value::DateTime(dt.with_timezone(&chrono::Local)));
            }
            if let Ok(date) = NaiveDate::parse_from_str(raw, "%Y-%m-%d") {
                let naive_dt = date.and_hms_opt(0, 0, 0).unwrap();
                return chrono::Local
                    .from_local_datetime(&naive_dt)
                    .single()
                    .map(Value::DateTime)
                    .ok_or_else(|| {
                        DefError::Runtime(format!(
                            "cannot parse param '{param_name}' value \"{raw}\" as datetime: ambiguous local time"
                        ))
                    });
            }
            Err(DefError::Runtime(format!(
                "cannot parse param '{param_name}' value \"{raw}\" as datetime: \
                 expected RFC3339 (2026-01-15T10:30:00+00:00) or date (2026-01-15)"
            )))
        }
        _ => Err(DefError::Runtime(
            "from_cmd_param default must be a string, integer, float, boolean, or datetime value".to_string(),
        )),
    }
}

pub(super) fn request_value_from_initializer(initializer: &Expression) -> DefResult<Value> {
    let method = match initializer {
        Expression::Identifier(method) => method.clone(),
        Expression::String(method) => method.clone(),
        expression => {
            return Err(DefError::Runtime(format!(
                "request initializer expects an HTTP method, got {expression:?}"
            )));
        }
    };

    Ok(new_request_value(&method))
}
