use std::{collections::HashMap, thread, time::Duration};

use crate::ast::{Expression, FunctionDefinition, Statement};
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
        for statement in &function.body {
            last_value = self.execute_statement(statement, &mut function_scopes)?;
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

            let Some(Statement::Expression(expression)) = program.statements.into_iter().next()
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
