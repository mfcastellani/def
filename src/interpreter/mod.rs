use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use crate::ast::{
    AssignmentOperator, AssignmentTarget, BinaryOperator, Expression, ForLoop, FunctionDefinition,
    IfStatement, ImportDefinition, MatchArm, Program, Statement, Type, VariableDefinition,
};
use crate::error::{DefError, DefResult};
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::value::Value;

mod collections;
mod datetime;
mod functions;
mod http;
mod values;

use collections::{
    call_array_method, call_array_push_on_global_variable, call_array_push_on_scoped_variable,
    call_tuple_method, get_array_index,
};
use datetime::{call_datetime_method, is_datetime_setter};
use functions::request_value_from_initializer;
use http::{apply_request_method, call_response_method, new_request_value, RequestMethodResult};
use values::{
    apply_assignment_operator, coerce_assignment, coerce_value_to_type, default_value_for_type,
    evaluate_boolean_binary, evaluate_numeric_binary, evaluate_numeric_comparison, evaluate_unary,
    is_valid_tuple_value, pattern_matches, printable_value, resolve_import_path,
};

type ScopeStack = Vec<HashMap<String, Value>>;

#[derive(Debug)]
pub struct Interpreter {
    variables: HashMap<String, Value>,
    functions: HashMap<String, FunctionDefinition>,
    imports: HashMap<String, Interpreter>,
    base_dir: PathBuf,
}

impl Default for Interpreter {
    fn default() -> Self {
        Self {
            variables: HashMap::new(),
            functions: HashMap::new(),
            imports: HashMap::new(),
            base_dir: PathBuf::from("."),
        }
    }
}

impl Interpreter {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_base_dir(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
            ..Self::default()
        }
    }

    pub fn interpret(&mut self, program: &Program) -> DefResult<Value> {
        let mut last_value = Value::Nil;
        let mut scopes = Vec::new();

        for statement in &program.statements {
            last_value = self.execute_statement(statement, &mut scopes)?;
        }

        Ok(last_value)
    }

    fn execute_statement(
        &mut self,
        statement: &Statement,
        scopes: &mut ScopeStack,
    ) -> DefResult<Value> {
        match statement {
            Statement::VariableDefinition(variable) => {
                self.define_variable(variable, scopes)?;
                Ok(Value::Nil)
            }
            Statement::ImportDefinition(import) => {
                self.define_import(import)?;
                Ok(Value::Nil)
            }
            Statement::Assignment(assignment) => {
                self.assign_variable(
                    &assignment.target,
                    assignment.operator,
                    &assignment.expression,
                    scopes,
                )?;
                Ok(Value::Nil)
            }
            Statement::ForLoop(for_loop) => self.execute_for_loop(for_loop, scopes),
            Statement::IfStatement(if_statement) => self.execute_if_statement(if_statement, scopes),
            Statement::FunctionDefinition(function) => {
                self.functions
                    .insert(function.name.clone(), function.clone());
                Ok(Value::Nil)
            }
            Statement::Expression(expression) => self.evaluate_expression(expression, scopes),
        }
    }

    fn define_variable(
        &mut self,
        variable: &VariableDefinition,
        scopes: &mut ScopeStack,
    ) -> DefResult<()> {
        let value = match &variable.initializer {
            Some(initializer) if variable.type_annotation == Type::Request => {
                request_value_from_initializer(initializer)?
            }
            Some(initializer) => {
                let value = self.evaluate_expression(initializer, scopes)?;
                coerce_value_to_type(&variable.type_annotation, value)?
            }
            None => default_value_for_type(&variable.type_annotation),
        };

        if let Some(scope) = scopes.last_mut() {
            scope.insert(variable.name.clone(), value);
        } else {
            self.variables.insert(variable.name.clone(), value);
        }
        Ok(())
    }

    fn assign_variable(
        &mut self,
        target: &AssignmentTarget,
        operator: AssignmentOperator,
        expression: &Expression,
        scopes: &mut ScopeStack,
    ) -> DefResult<()> {
        let value = self.evaluate_expression(expression, scopes)?;

        let AssignmentTarget::Identifier(name) = target else {
            return self.assign_import_member(target, operator, value);
        };

        if let Some(current) = get_scoped_variable(scopes, name).cloned() {
            let value = self.apply_assignment_operator(name, current.clone(), operator, value)?;
            let value = coerce_assignment(name, &current, value)?;
            assign_scoped_variable(scopes, name, value)?;
            return Ok(());
        }

        let Some(current) = self.variables.get(name) else {
            return Err(DefError::Runtime(format!(
                "invalid assignment: undefined variable '{name}'"
            )));
        };

        let value = self.apply_assignment_operator(name, current.clone(), operator, value)?;
        let value = coerce_assignment(name, current, value)?;
        self.variables.insert(name.to_string(), value);
        Ok(())
    }

    fn assign_import_member(
        &mut self,
        target: &AssignmentTarget,
        operator: AssignmentOperator,
        value: Value,
    ) -> DefResult<()> {
        let AssignmentTarget::Member { object, member } = target else {
            unreachable!();
        };

        let module = self
            .imports
            .get_mut(object)
            .ok_or_else(|| DefError::Runtime(format!("undefined import '{object}'")))?;

        let Some(current) = module.variables.get(member) else {
            return Err(DefError::Runtime(format!(
                "invalid assignment: undefined imported variable '{object}.{member}'"
            )));
        };

        let value = apply_assignment_operator(
            &format!("{object}.{member}"),
            current.clone(),
            operator,
            value,
        )?;
        let value = coerce_assignment(&format!("{object}.{member}"), current, value)?;
        module.variables.insert(member.clone(), value);
        Ok(())
    }

    fn apply_assignment_operator(
        &self,
        name: &str,
        current: Value,
        operator: AssignmentOperator,
        value: Value,
    ) -> DefResult<Value> {
        apply_assignment_operator(name, current, operator, value)
    }

    fn execute_for_loop(
        &mut self,
        for_loop: &ForLoop,
        scopes: &mut ScopeStack,
    ) -> DefResult<Value> {
        let iterable = self.evaluate_expression(&for_loop.iterable, scopes)?;
        let Value::Array(items) = iterable else {
            return Err(DefError::Runtime(
                "for loop expects an array expression".to_string(),
            ));
        };

        let mut last_value = Value::Nil;

        for item in items {
            scopes.push(HashMap::from([(for_loop.variable.clone(), item)]));
            for statement in &for_loop.body {
                last_value = self.execute_statement(statement, scopes)?;
            }
            scopes.pop();
        }

        Ok(last_value)
    }

    fn execute_if_statement(
        &mut self,
        if_statement: &IfStatement,
        scopes: &mut ScopeStack,
    ) -> DefResult<Value> {
        let condition = self.evaluate_expression(&if_statement.condition, scopes)?;
        let Value::Boolean(condition) = condition else {
            return Err(DefError::Runtime(
                "if condition must evaluate to boolean".to_string(),
            ));
        };

        let body = if condition {
            &if_statement.then_body
        } else {
            &if_statement.else_body
        };

        self.execute_block(body, scopes)
    }

    fn execute_block(&mut self, body: &[Statement], scopes: &mut ScopeStack) -> DefResult<Value> {
        scopes.push(HashMap::new());
        let mut last_value = Value::Nil;

        for statement in body {
            last_value = self.execute_statement(statement, scopes)?;
        }

        scopes.pop();
        Ok(last_value)
    }

    fn define_import(&mut self, import: &ImportDefinition) -> DefResult<()> {
        let path = resolve_import_path(&self.base_dir, &import.path);
        let source = fs::read_to_string(&path).map_err(|error| {
            DefError::Runtime(format!("failed to import '{}': {error}", path.display()))
        })?;

        let mut lexer = Lexer::new(&source);
        let tokens = lexer.tokenize()?;
        let program = Parser::new(tokens).parse_program()?;
        let base_dir = path.parent().unwrap_or(Path::new(".")).to_path_buf();
        let mut module = Interpreter::with_base_dir(base_dir);
        module.interpret(&program)?;

        self.imports.insert(import.name.clone(), module);
        Ok(())
    }

    fn evaluate_expression(
        &mut self,
        expression: &Expression,
        scopes: &mut ScopeStack,
    ) -> DefResult<Value> {
        match expression {
            Expression::Integer(value) => Ok(Value::Integer(*value)),
            Expression::Float(value) => Ok(Value::Float(*value)),
            Expression::String(value) => Ok(Value::String(value.clone())),
            Expression::Boolean(value) => Ok(Value::Boolean(*value)),
            Expression::Array(items) => {
                let mut values = Vec::with_capacity(items.len());
                for item in items {
                    values.push(self.evaluate_expression(item, scopes)?);
                }
                Ok(Value::Array(values))
            }
            Expression::Tuple(items) => self.evaluate_tuple(items, scopes),
            Expression::Request { method } => Ok(new_request_value(method)),
            Expression::Identifier(name) => {
                if let Some(value) =
                    get_scoped_variable(scopes, name).or_else(|| self.variables.get(name))
                {
                    return match value {
                        Value::Request(_) => Ok(Value::RequestHandle(name.clone())),
                        value => Ok(value.clone()),
                    };
                }

                Err(DefError::Runtime(format!("undefined identifier '{name}'")))
            }
            Expression::Binary {
                left,
                operator,
                right,
            } => {
                let left = self.evaluate_expression(left, scopes)?;
                let right = self.evaluate_expression(right, scopes)?;
                self.evaluate_binary(left, *operator, right)
            }
            Expression::Unary {
                operator,
                expression,
            } => {
                let value = self.evaluate_expression(expression, scopes)?;
                evaluate_unary(*operator, value)
            }
            Expression::FunctionCall { name, args } => self.call_function(name, args, scopes),
            Expression::MemberAccess { object, member } => self.get_member(object, member, scopes),
            Expression::MemberFunctionCall { object, name, args } => {
                self.call_member_function(object, name, args, scopes)
            }
            Expression::Index { object, index } => self.evaluate_index(object, index, scopes),
            Expression::Match { value, arms } => self.evaluate_match(value, arms, scopes),
            Expression::Block(statements) => self.execute_block(statements, scopes),
        }
    }

    fn evaluate_tuple(
        &mut self,
        items: &[Expression],
        scopes: &mut ScopeStack,
    ) -> DefResult<Value> {
        if items.len() != 2 {
            return Err(DefError::Runtime(format!(
                "tuple expects 2 arguments, got {}",
                items.len()
            )));
        }

        let key = self.evaluate_expression(&items[0], scopes)?;
        let Value::String(key) = key else {
            return Err(DefError::Runtime("tuple key must be a string".to_string()));
        };

        let value = self.evaluate_expression(&items[1], scopes)?;
        if !is_valid_tuple_value(&value) {
            return Err(DefError::Runtime(
                "tuple value must be string, integer, float or boolean".to_string(),
            ));
        }

        Ok(Value::Tuple {
            key,
            value: Box::new(value),
        })
    }

    fn evaluate_index(
        &mut self,
        object: &Expression,
        index: &Expression,
        scopes: &mut ScopeStack,
    ) -> DefResult<Value> {
        let object = self.evaluate_expression(object, scopes)?;
        let index = self.evaluate_expression(index, scopes)?;
        get_array_index(object, index)
    }

    fn get_member(
        &mut self,
        object: &Expression,
        member: &str,
        scopes: &mut ScopeStack,
    ) -> DefResult<Value> {
        if let Expression::Identifier(object) = object {
            if self.imports.contains_key(object) {
                return self.get_import_member(object, member);
            }
        }

        let object = self.evaluate_expression(object, scopes)?;
        Err(DefError::Runtime(format!(
            "undefined member '{object:?}.{member}'"
        )))
    }

    fn get_import_member(&self, object: &str, member: &str) -> DefResult<Value> {
        self.imports
            .get(object)
            .ok_or_else(|| DefError::Runtime(format!("undefined import '{object}'")))?
            .variables
            .get(member)
            .cloned()
            .ok_or_else(|| {
                DefError::Runtime(format!("undefined imported member '{object}.{member}'"))
            })
    }

    fn call_member_function(
        &mut self,
        object: &Expression,
        name: &str,
        args: &[Expression],
        scopes: &mut ScopeStack,
    ) -> DefResult<Value> {
        let mut values = Vec::with_capacity(args.len());
        for arg in args {
            if name == "with_var" {
                let Expression::Identifier(var_name) = arg else {
                    return Err(DefError::Runtime(
                        "request.with_var expects a variable identifier".to_string(),
                    ));
                };
                values.push(Value::Tuple {
                    key: var_name.clone(),
                    value: Box::new(self.evaluate_expression(arg, scopes)?),
                });
            } else if name == "type" {
                if let Expression::Identifier(type_name) = arg {
                    values.push(Value::String(type_name.clone()));
                } else {
                    values.push(self.evaluate_expression(arg, scopes)?);
                }
            } else {
                values.push(self.evaluate_expression(arg, scopes)?);
            }
        }

        if let Expression::Identifier(object_name) = object {
            if self.imports.contains_key(object_name) {
                return self
                    .imports
                    .get_mut(object_name)
                    .ok_or_else(|| DefError::Runtime(format!("undefined import '{object_name}'")))?
                    .call_user_function_with_values(name, values);
            }
        }

        if let Expression::Identifier(object_name) = object {
            if name == "push" {
                if let Some(value) =
                    call_array_push_on_scoped_variable(scopes, object_name, &values)?
                {
                    return Ok(value);
                }
                if let Some(value) =
                    call_array_push_on_global_variable(&mut self.variables, object_name, &values)?
                {
                    return Ok(value);
                }
            }

            if is_datetime_setter(name, values.len()) {
                if let Some(value) = self.call_datetime_setter_on_scoped_variable(
                    scopes,
                    object_name,
                    name,
                    values.clone(),
                )? {
                    return Ok(value);
                }
                if let Some(value) =
                    self.call_datetime_setter_on_global_variable(object_name, name, values.clone())?
                {
                    return Ok(value);
                }
            }
        }

        let object = self.evaluate_expression(object, scopes)?;
        match object {
            Value::RequestHandle(_) | Value::Request(_) => {
                self.call_request_method(object, name, values)
            }
            Value::Response(response) => call_response_method(response, name, values),
            Value::Array(items) => call_array_method(items, name, values),
            Value::Tuple { key, value } => call_tuple_method(key, *value, name, values),
            Value::DateTime(value) => call_datetime_method(value, name, values),
            _ => Err(DefError::Runtime(format!(
                "member function '{name}' is only available on request, response, array, tuple or datetime values"
            ))),
        }
    }

    fn call_request_method(
        &mut self,
        object: Value,
        name: &str,
        args: Vec<Value>,
    ) -> DefResult<Value> {
        match object {
            Value::RequestHandle(request_name) => {
                let base_dir = self.base_dir.clone();
                let request = self.variables.get_mut(&request_name).ok_or_else(|| {
                    DefError::Runtime(format!("undefined request '{request_name}'"))
                })?;

                let Value::Request(request) = request else {
                    return Err(DefError::Runtime(format!(
                        "member function '{request_name}.{name}' is only available on request values"
                    )));
                };

                let result = apply_request_method(request, name, args, &base_dir)?;
                Ok(match result {
                    RequestMethodResult::Request => Value::RequestHandle(request_name),
                    RequestMethodResult::Value(value) => value,
                })
            }
            Value::Request(mut request) => {
                let base_dir = self.base_dir.clone();
                let result = apply_request_method(&mut request, name, args, &base_dir)?;
                Ok(match result {
                    RequestMethodResult::Request => Value::Request(request),
                    RequestMethodResult::Value(value) => value,
                })
            }
            _ => Err(DefError::Runtime(format!(
                "member function '{name}' is only available on request values"
            ))),
        }
    }

    fn call_datetime_setter_on_scoped_variable(
        &mut self,
        scopes: &mut ScopeStack,
        object_name: &str,
        name: &str,
        args: Vec<Value>,
    ) -> DefResult<Option<Value>> {
        for scope in scopes.iter_mut().rev() {
            let Some(value) = scope.get_mut(object_name) else {
                continue;
            };

            let Value::DateTime(datetime) = value.clone() else {
                return Ok(None);
            };

            let updated = call_datetime_method(datetime, name, args)?;
            *value = updated.clone();
            return Ok(Some(updated));
        }

        Ok(None)
    }

    fn call_datetime_setter_on_global_variable(
        &mut self,
        object_name: &str,
        name: &str,
        args: Vec<Value>,
    ) -> DefResult<Option<Value>> {
        let Some(value) = self.variables.get_mut(object_name) else {
            return Ok(None);
        };

        let Value::DateTime(datetime) = value.clone() else {
            return Ok(None);
        };

        let updated = call_datetime_method(datetime, name, args)?;
        *value = updated.clone();
        Ok(Some(updated))
    }

    fn evaluate_match(
        &mut self,
        value: &Expression,
        arms: &[MatchArm],
        scopes: &mut ScopeStack,
    ) -> DefResult<Value> {
        let value = self.evaluate_expression(value, scopes)?;

        for arm in arms {
            if pattern_matches(&arm.pattern, &value) {
                return self.evaluate_expression(&arm.expression, scopes);
            }
        }

        Err(DefError::Runtime(format!(
            "match expression did not match value {value:?}"
        )))
    }

    fn evaluate_binary(
        &self,
        left: Value,
        operator: BinaryOperator,
        right: Value,
    ) -> DefResult<Value> {
        if operator == BinaryOperator::Equal {
            return Ok(Value::Boolean(left == right));
        }
        if operator == BinaryOperator::NotEqual {
            return Ok(Value::Boolean(left != right));
        }
        if matches!(operator, BinaryOperator::And | BinaryOperator::Or) {
            return evaluate_boolean_binary(left, operator, right);
        }
        if matches!(
            operator,
            BinaryOperator::Greater
                | BinaryOperator::GreaterEqual
                | BinaryOperator::Less
                | BinaryOperator::LessEqual
        ) {
            return evaluate_numeric_comparison(left, operator, right);
        }

        evaluate_numeric_binary(left, operator, right)
    }
}

fn get_scoped_variable<'a>(scopes: &'a ScopeStack, name: &str) -> Option<&'a Value> {
    scopes.iter().rev().find_map(|scope| scope.get(name))
}

fn assign_scoped_variable(scopes: &mut ScopeStack, name: &str, value: Value) -> DefResult<()> {
    for scope in scopes.iter_mut().rev() {
        if scope.contains_key(name) {
            scope.insert(name.to_string(), value);
            return Ok(());
        }
    }

    Err(DefError::Runtime(format!(
        "invalid assignment: undefined local variable '{name}'"
    )))
}

#[cfg(test)]
mod tests;
