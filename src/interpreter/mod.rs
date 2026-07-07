use std::{
    collections::{HashMap, HashSet},
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
};

use crate::ast::{
    AssignmentOperator, AssignmentTarget, BinaryOperator, EnvVarsLoad, Expression, ForLoop,
    FunctionDefinition, IfStatement, ImportDefinition, MatchArm, Program, Statement, Stmt, Type,
    UnaryOperator, VariableDefinition, WhileLoop,
};
use crate::error::{DefError, DefResult};
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::value::{FileMode, FileValue, MockValue, Value};

mod collections;
mod datetime;
mod files;
mod functions;
mod http;
mod mock;
mod values;

use collections::{
    call_array_method, call_array_push_on_global_variable, call_array_push_on_scoped_variable,
    call_tuple_method, get_array_index,
};
use files::FileState;
use datetime::{call_datetime_method, is_datetime_setter};
use functions::request_value_from_initializer;
use http::{apply_request_method, call_response_method, new_request_value, RequestMethodResult};
use mock::call_mock_method;
use values::{
    apply_assignment_operator, call_float_method, call_integer_method, call_string_method,
    coerce_assignment, coerce_value_to_type, default_value_for_type, evaluate_boolean_binary,
    evaluate_numeric_binary, evaluate_numeric_comparison, evaluate_unary, is_valid_tuple_value,
    pattern_matches, printable_value,
};

pub(crate) use values::resolve_import_path;

pub(super) struct ScopeFrame {
    pub(super) vars: HashMap<String, Value>,
    pub(super) consts: HashSet<String>,
}

impl ScopeFrame {
    pub(super) fn new() -> Self {
        Self { vars: HashMap::new(), consts: HashSet::new() }
    }

    pub(super) fn with_vars(vars: HashMap<String, Value>) -> Self {
        Self { vars, consts: HashSet::new() }
    }
}

type ScopeStack = Vec<ScopeFrame>;

#[derive(Debug)]
pub struct Interpreter {
    variables: HashMap<String, Value>,
    const_vars: HashSet<String>,
    functions: HashMap<String, FunctionDefinition>,
    imports: HashMap<String, Interpreter>,
    base_dir: PathBuf,
    source_file: String,
    pub(crate) dry_run: bool,
    params: HashMap<String, String>,
    file_states: HashMap<String, FileState>,
}

impl Default for Interpreter {
    fn default() -> Self {
        Self {
            variables: HashMap::new(),
            const_vars: HashSet::new(),
            functions: HashMap::new(),
            imports: HashMap::new(),
            base_dir: PathBuf::from("."),
            source_file: String::new(),
            dry_run: false,
            params: HashMap::new(),
            file_states: HashMap::new(),
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

    pub fn with_source_file(mut self, file: impl Into<String>) -> Self {
        self.source_file = file.into();
        self
    }

    pub fn with_dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }

    pub fn with_params(mut self, params: HashMap<String, String>) -> Self {
        self.params = params;
        self
    }

    /// Returns all global mock variables defined in this interpreter after execution.
    pub fn mocks(&self) -> Vec<(String, MockValue)> {
        self.variables
            .iter()
            .filter_map(|(name, v)| {
                if let Value::Mock(m) = v {
                    Some((name.clone(), m.clone()))
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn interpret(&mut self, program: &Program) -> DefResult<Value> {
        let mut last_value = Value::Nil;
        let mut scopes = Vec::new();

        for stmt in &program.statements {
            last_value = self
                .execute_statement(&stmt.inner, &mut scopes)
                .map_err(|e| match e {
                    DefError::LoopBreak => {
                        DefError::Runtime("break() called outside of a loop".to_string())
                    }
                    DefError::LoopNext => {
                        DefError::Runtime("next() called outside of a loop".to_string())
                    }
                    e if self.source_file.is_empty() => e,
                    e => e.at_location(stmt.line, &self.source_file),
                })?;
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
            Statement::EnvVarsLoad(load) => {
                self.load_env_vars(load)?;
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
            Statement::WhileLoop(while_loop) => self.execute_while_loop(while_loop, scopes),
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

        if let Some(frame) = scopes.last_mut() {
            frame.vars.insert(variable.name.clone(), value);
            if variable.is_const {
                frame.consts.insert(variable.name.clone());
            }
        } else {
            self.variables.insert(variable.name.clone(), value);
            if variable.is_const {
                self.const_vars.insert(variable.name.clone());
            }
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
            if is_scoped_const(scopes, name) {
                return Err(DefError::Runtime(format!(
                    "cannot assign to const variable '{name}'"
                )));
            }
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

        if self.const_vars.contains(name) {
            return Err(DefError::Runtime(format!(
                "cannot assign to const variable '{name}'"
            )));
        }

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

        if module.const_vars.contains(member) {
            return Err(DefError::Runtime(format!(
                "cannot assign to const imported variable '{object}.{member}'"
            )));
        }

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

        'outer: for item in items {
            scopes.push(ScopeFrame::with_vars(HashMap::from([(for_loop.variable.clone(), item)])));
            for stmt in &for_loop.body {
                match self.execute_statement(&stmt.inner, scopes) {
                    Ok(v) => last_value = v,
                    Err(DefError::LoopBreak) => {
                        scopes.pop();
                        break 'outer;
                    }
                    Err(DefError::LoopNext) => break,
                    Err(e) => {
                        scopes.pop();
                        return Err(e);
                    }
                }
            }
            scopes.pop();
        }

        Ok(last_value)
    }

    fn execute_while_loop(
        &mut self,
        while_loop: &WhileLoop,
        scopes: &mut ScopeStack,
    ) -> DefResult<Value> {
        let mut last_value = Value::Nil;

        loop {
            let condition = self.evaluate_expression(&while_loop.condition, scopes)?;
            let Value::Boolean(condition) = condition else {
                return Err(DefError::Runtime(
                    "while condition must evaluate to boolean".to_string(),
                ));
            };
            if !condition {
                break;
            }

            scopes.push(ScopeFrame::new());
            let mut break_loop = false;
            for stmt in &while_loop.body {
                match self.execute_statement(&stmt.inner, scopes) {
                    Ok(v) => last_value = v,
                    Err(DefError::LoopBreak) => {
                        break_loop = true;
                        break;
                    }
                    Err(DefError::LoopNext) => break,
                    Err(e) => {
                        scopes.pop();
                        return Err(e);
                    }
                }
            }
            scopes.pop();
            if break_loop {
                break;
            }
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

    fn execute_block(&mut self, body: &[Stmt], scopes: &mut ScopeStack) -> DefResult<Value> {
        scopes.push(ScopeFrame::new());
        let mut last_value = Value::Nil;

        for stmt in body {
            last_value = self.execute_statement(&stmt.inner, scopes)?;
        }

        scopes.pop();
        Ok(last_value)
    }

    fn define_import(&mut self, import: &ImportDefinition) -> DefResult<()> {
        let path = resolve_import_path(&self.base_dir, &import.path);
        let path_str = path.to_string_lossy().into_owned();
        let source = fs::read_to_string(&path).map_err(|error| {
            DefError::Runtime(format!("failed to import '{}': {error}", path.display()))
        })?;

        let mut lexer = Lexer::new(&source);
        let tokens = lexer.tokenize().map_err(|e| e.in_file(&path_str))?;
        let program = Parser::new(tokens)
            .parse_program()
            .map_err(|e| e.in_file(&path_str))?;
        let base_dir = path.parent().unwrap_or(Path::new(".")).to_path_buf();
        let mut module = Interpreter::with_base_dir(base_dir)
            .with_source_file(path_str)
            .with_dry_run(self.dry_run)
            .with_params(self.params.clone());
        module.interpret(&program)?;

        self.imports.insert(import.name.clone(), module);
        Ok(())
    }

    fn load_env_vars(&self, load: &EnvVarsLoad) -> DefResult<()> {
        let path = if std::path::Path::new(&load.path).is_absolute() {
            std::path::PathBuf::from(&load.path)
        } else {
            self.base_dir.join(&load.path)
        };

        let content = fs::read_to_string(&path).map_err(|error| {
            DefError::Runtime(format!(
                "failed to read env file '{}': {error}",
                path.display()
            ))
        })?;

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with("//") || line.starts_with('#') {
                continue;
            }
            let Some(eq_pos) = line.find('=') else {
                continue;
            };
            let var_name = line[..eq_pos].trim();
            let var_value = &line[eq_pos + 1..];
            if var_name.is_empty() {
                continue;
            }
            if std::env::var(var_name).is_ok() {
                eprintln!(
                    "warning: env var '{var_name}' defined in '{}' is already set in the system environment — file value ignored",
                    path.display()
                );
            } else {
                std::env::set_var(var_name, var_value);
            }
        }

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
            Expression::File { mode } => {
                let file_mode = match mode.as_str() {
                    "READ" => FileMode::Read,
                    "WRITE" => FileMode::Write,
                    "APPEND" => FileMode::Append,
                    _ => return Err(DefError::Runtime(format!(
                        "invalid file mode '{mode}'; expected READ, WRITE, or APPEND"
                    ))),
                };
                Ok(Value::File(FileValue {
                    path: None,
                    mode: file_mode,
                    is_open: false,
                }))
            }
            Expression::Mock { method, url } => {
                let url_val = self.evaluate_expression(url, scopes)?;
                let Value::String(url) = url_val else {
                    return Err(DefError::Runtime("mock URL must be a string".to_string()));
                };
                Ok(Value::Mock(crate::value::MockValue {
                    method: method.clone(),
                    url,
                    status: 0,
                    body: String::new(),
                    headers: Vec::new(),
                    vars: Vec::new(),
                    delay_ms: 0,
                    configured: false,
                    snapshot_path: None,
                }))
            }
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
            Expression::Block(stmts) => self.execute_block(stmts, scopes),
            Expression::Range { .. } => Err(DefError::Runtime(  // only valid inside range()
                "range expression is only valid as an argument to range()".to_string(),
            )),
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

    fn call_response_expect(
        &mut self,
        object: Value,
        args: &[Expression],
        scopes: &mut ScopeStack,
    ) -> DefResult<Value> {
        let response = match &object {
            Value::Response(r) => r,
            _ => {
                return Err(DefError::Runtime(
                    "expect is only available on response values".to_string(),
                ))
            }
        };

        if args.len() != 1 {
            return Err(DefError::Runtime(format!(
                "expect takes 1 predicate, got {}",
                args.len()
            )));
        }

        let status = response.status;
        let ok = response.status >= 200 && response.status < 300;
        let duration = response.duration_ms;
        let size = response.body.len() as i64;
        let body = response.body.clone();
        let content_type = response
            .headers
            .iter()
            .find(|(n, _)| n.to_lowercase() == "content-type")
            .map(|(_, v)| v.clone())
            .unwrap_or_default();

        let mut expect_vars = HashMap::new();
        expect_vars.insert("status".to_string(), Value::Integer(status));
        expect_vars.insert("ok".to_string(), Value::Boolean(ok));
        expect_vars.insert("duration".to_string(), Value::Integer(duration));
        expect_vars.insert("size".to_string(), Value::Integer(size));
        expect_vars.insert("body".to_string(), Value::String(body));
        expect_vars.insert("content_type".to_string(), Value::String(content_type));

        scopes.push(ScopeFrame::with_vars(expect_vars));
        let result = self.evaluate_expression(&args[0], scopes);
        scopes.pop();

        let Value::Boolean(passed) = result? else {
            return Err(DefError::Runtime(
                "expect predicate must evaluate to a boolean".to_string(),
            ));
        };

        if !passed {
            let predicate = format_predicate(&args[0]);
            return Err(DefError::Runtime(format!(
                "expect({predicate}) failed: status={status}, ok={ok}, duration={duration}ms"
            )));
        }

        Ok(object)
    }

    fn call_member_function(
        &mut self,
        object: &Expression,
        name: &str,
        args: &[Expression],
        scopes: &mut ScopeStack,
    ) -> DefResult<Value> {
        if name == "expect" {
            let object_val = self.evaluate_expression(object, scopes)?;
            return self.call_response_expect(object_val, args, scopes);
        }

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
                if is_scoped_const(scopes, object_name) || self.const_vars.contains(object_name) {
                    return Err(DefError::Runtime(format!(
                        "cannot call push() on const array '{object_name}'"
                    )));
                }
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

            let is_file_var = matches!(
                get_scoped_variable(scopes, object_name).or_else(|| self.variables.get(object_name.as_str())),
                Some(Value::File(_))
            );
            if is_file_var {
                return self.call_file_method_on_variable(scopes, object_name, name, values);
            }
        }

        let object = self.evaluate_expression(object, scopes)?;

        match object {
            Value::RequestHandle(_) | Value::Request(_) => {
                self.call_request_method(object, name, values)
            }
            Value::Response(response) => call_response_method(response, name, values, &self.base_dir),
            Value::Array(items) => call_array_method(items, name, values),
            Value::Tuple { key, value } => call_tuple_method(key, *value, name, values),
            Value::DateTime(value) => call_datetime_method(value, name, values),
            Value::String(s) => call_string_method(&s, name, values),
            Value::Integer(n) => call_integer_method(n, name, values),
            Value::Float(f) => call_float_method(f, name, values),
            Value::Mock(mock) => call_mock_method(mock, name, values, &self.base_dir),
            Value::File(file_val) => {
                if name == "path" {
                    if values.len() != 1 {
                        return Err(DefError::Runtime(format!(
                            "file.path expects 1 argument, got {}",
                            values.len()
                        )));
                    }
                    let Value::String(path) = values.into_iter().next().unwrap() else {
                        return Err(DefError::Runtime(
                            "file.path expects a string argument".to_string(),
                        ));
                    };
                    let mut updated = file_val;
                    updated.path = Some(path);
                    Ok(Value::File(updated))
                } else {
                    Err(DefError::Runtime(format!(
                        "file method '{name}' must be called on a named file variable, not an inline expression"
                    )))
                }
            }
            _ => Err(DefError::Runtime(format!(
                "member function '{name}' is only available on request, response, array, tuple, datetime, string, integer, float, mock, or file values"
            ))),
        }
    }

    fn call_request_method(
        &mut self,
        object: Value,
        name: &str,
        args: Vec<Value>,
    ) -> DefResult<Value> {
        if self.dry_run && name == "do" {
            return Ok(Value::Response(crate::value::ResponseValue {
                status: 200,
                body: String::new(),
                headers: vec![],
                duration_ms: 0,
                method: String::new(),
                url: String::new(),
            }));
        }

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

    fn get_file_value(&self, scopes: &ScopeStack, var_name: &str) -> DefResult<FileValue> {
        for frame in scopes.iter().rev() {
            if let Some(Value::File(fv)) = frame.vars.get(var_name) {
                return Ok(fv.clone());
            }
        }
        if let Some(Value::File(fv)) = self.variables.get(var_name) {
            return Ok(fv.clone());
        }
        Err(DefError::Runtime(format!(
            "undefined file variable '{var_name}'"
        )))
    }

    fn set_file_value(
        &mut self,
        scopes: &mut ScopeStack,
        var_name: &str,
        file_val: FileValue,
    ) -> DefResult<()> {
        for frame in scopes.iter_mut().rev() {
            if frame.vars.contains_key(var_name) {
                frame.vars.insert(var_name.to_string(), Value::File(file_val));
                return Ok(());
            }
        }
        if self.variables.contains_key(var_name) {
            self.variables
                .insert(var_name.to_string(), Value::File(file_val));
            return Ok(());
        }
        Err(DefError::Runtime(format!(
            "undefined file variable '{var_name}'"
        )))
    }

    fn call_file_method_on_variable(
        &mut self,
        scopes: &mut ScopeStack,
        var_name: &str,
        method: &str,
        args: Vec<Value>,
    ) -> DefResult<Value> {
        match method {
            "path" => {
                if args.len() != 1 {
                    return Err(DefError::Runtime(format!(
                        "file.path expects 1 argument, got {}",
                        args.len()
                    )));
                }
                let Value::String(path) = args.into_iter().next().unwrap() else {
                    return Err(DefError::Runtime(
                        "file.path expects a string argument".to_string(),
                    ));
                };
                let mut fv = self.get_file_value(scopes, var_name)?;
                fv.path = Some(path);
                self.set_file_value(scopes, var_name, fv)?;
                Ok(Value::Nil)
            }
            "open" => {
                if !args.is_empty() {
                    return Err(DefError::Runtime(format!(
                        "file.open expects 0 arguments, got {}",
                        args.len()
                    )));
                }
                let fv = self.get_file_value(scopes, var_name)?;
                if fv.is_open {
                    return Err(DefError::Runtime(format!(
                        "file '{var_name}' is already open"
                    )));
                }
                let path_str = fv.path.as_ref().ok_or_else(|| {
                    DefError::Runtime(format!(
                        "file '{var_name}' has no path set; call .path(\"...\") first"
                    ))
                })?;
                let full_path = if Path::new(path_str.as_str()).is_absolute() {
                    PathBuf::from(path_str)
                } else {
                    self.base_dir.join(path_str)
                };
                let state = match fv.mode {
                    FileMode::Read => {
                        let file = File::open(&full_path).map_err(|e| {
                            DefError::Runtime(format!(
                                "cannot open '{}' for reading: {e}",
                                full_path.display()
                            ))
                        })?;
                        FileState::Read(files::ReaderState {
                            reader: BufReader::new(file),
                            eof: false,
                        })
                    }
                    FileMode::Write => {
                        let file = File::create(&full_path).map_err(|e| {
                            DefError::Runtime(format!(
                                "cannot open '{}' for writing: {e}",
                                full_path.display()
                            ))
                        })?;
                        FileState::Write(BufWriter::new(file))
                    }
                    FileMode::Append => {
                        let file = OpenOptions::new()
                            .append(true)
                            .create(true)
                            .open(&full_path)
                            .map_err(|e| {
                                DefError::Runtime(format!(
                                    "cannot open '{}' for appending: {e}",
                                    full_path.display()
                                ))
                            })?;
                        FileState::Append(BufWriter::new(file))
                    }
                };
                self.file_states.insert(var_name.to_string(), state);
                let mut updated = fv;
                updated.is_open = true;
                self.set_file_value(scopes, var_name, updated)?;
                Ok(Value::Nil)
            }
            "eof" => {
                if !args.is_empty() {
                    return Err(DefError::Runtime(format!(
                        "file.eof expects 0 arguments, got {}",
                        args.len()
                    )));
                }
                let state = self.file_states.get(var_name).ok_or_else(|| {
                    DefError::Runtime(format!("file '{var_name}' is not open; call .open() first"))
                })?;
                match state {
                    FileState::Read(rs) => Ok(Value::Boolean(rs.eof)),
                    _ => Err(DefError::Runtime(
                        "file.eof is only available for files opened in READ mode".to_string(),
                    )),
                }
            }
            "read_line" => {
                if !args.is_empty() {
                    return Err(DefError::Runtime(format!(
                        "file.read_line expects 0 arguments, got {}",
                        args.len()
                    )));
                }
                let state = self.file_states.get_mut(var_name).ok_or_else(|| {
                    DefError::Runtime(format!("file '{var_name}' is not open; call .open() first"))
                })?;
                match state {
                    FileState::Read(rs) => {
                        let mut line = String::new();
                        let bytes = rs.reader.read_line(&mut line).map_err(|e| {
                            DefError::Runtime(format!("read_line error: {e}"))
                        })?;
                        if bytes == 0 {
                            rs.eof = true;
                            Ok(Value::String(String::new()))
                        } else {
                            if line.ends_with('\n') {
                                line.pop();
                                if line.ends_with('\r') {
                                    line.pop();
                                }
                            }
                            Ok(Value::String(line))
                        }
                    }
                    _ => Err(DefError::Runtime(
                        "file.read_line is only available for files opened in READ mode"
                            .to_string(),
                    )),
                }
            }
            "read_all" => {
                if !args.is_empty() {
                    return Err(DefError::Runtime(format!(
                        "file.read_all expects 0 arguments, got {}",
                        args.len()
                    )));
                }
                let state = self.file_states.get_mut(var_name).ok_or_else(|| {
                    DefError::Runtime(format!("file '{var_name}' is not open; call .open() first"))
                })?;
                match state {
                    FileState::Read(rs) => {
                        let mut content = String::new();
                        rs.reader.read_to_string(&mut content).map_err(|e| {
                            DefError::Runtime(format!("read_all error: {e}"))
                        })?;
                        rs.eof = true;
                        Ok(Value::String(content))
                    }
                    _ => Err(DefError::Runtime(
                        "file.read_all is only available for files opened in READ mode".to_string(),
                    )),
                }
            }
            "write" => {
                if args.len() != 1 {
                    return Err(DefError::Runtime(format!(
                        "file.write expects 1 argument, got {}",
                        args.len()
                    )));
                }
                let Value::String(content) = args.into_iter().next().unwrap() else {
                    return Err(DefError::Runtime(
                        "file.write expects a string argument".to_string(),
                    ));
                };
                let state = self.file_states.get_mut(var_name).ok_or_else(|| {
                    DefError::Runtime(format!("file '{var_name}' is not open; call .open() first"))
                })?;
                match state {
                    FileState::Write(w) | FileState::Append(w) => {
                        w.write_all(content.as_bytes()).map_err(|e| {
                            DefError::Runtime(format!("file.write error: {e}"))
                        })?;
                        Ok(Value::Nil)
                    }
                    _ => Err(DefError::Runtime(
                        "file.write is only available for files opened in WRITE or APPEND mode"
                            .to_string(),
                    )),
                }
            }
            "flush" => {
                if !args.is_empty() {
                    return Err(DefError::Runtime(format!(
                        "file.flush expects 0 arguments, got {}",
                        args.len()
                    )));
                }
                let state = self.file_states.get_mut(var_name).ok_or_else(|| {
                    DefError::Runtime(format!("file '{var_name}' is not open; call .open() first"))
                })?;
                match state {
                    FileState::Write(w) | FileState::Append(w) => {
                        w.flush().map_err(|e| {
                            DefError::Runtime(format!("file.flush error: {e}"))
                        })?;
                        Ok(Value::Nil)
                    }
                    _ => Err(DefError::Runtime(
                        "file.flush is only available for files opened in WRITE or APPEND mode"
                            .to_string(),
                    )),
                }
            }
            "close" => {
                if !args.is_empty() {
                    return Err(DefError::Runtime(format!(
                        "file.close expects 0 arguments, got {}",
                        args.len()
                    )));
                }
                if let Some(state) = self.file_states.remove(var_name) {
                    match state {
                        FileState::Write(mut w) | FileState::Append(mut w) => {
                            w.flush().map_err(|e| {
                                DefError::Runtime(format!("file.close flush error: {e}"))
                            })?;
                        }
                        FileState::Read(_) => {}
                    }
                }
                let mut fv = self.get_file_value(scopes, var_name)?;
                fv.is_open = false;
                self.set_file_value(scopes, var_name, fv)?;
                Ok(Value::Nil)
            }
            _ => Err(DefError::Runtime(format!(
                "undefined file method '{method}'"
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
        for frame in scopes.iter_mut().rev() {
            let Some(value) = frame.vars.get_mut(object_name) else {
                continue;
            };

            if frame.consts.contains(object_name) {
                return Err(DefError::Runtime(format!(
                    "cannot call '{name}()' on const datetime '{object_name}'"
                )));
            }

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

        if self.const_vars.contains(object_name) {
            return Err(DefError::Runtime(format!(
                "cannot call '{name}()' on const datetime '{object_name}'"
            )));
        }

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
    scopes.iter().rev().find_map(|frame| frame.vars.get(name))
}

fn is_scoped_const(scopes: &ScopeStack, name: &str) -> bool {
    for frame in scopes.iter().rev() {
        if frame.vars.contains_key(name) {
            return frame.consts.contains(name);
        }
    }
    false
}

fn assign_scoped_variable(scopes: &mut ScopeStack, name: &str, value: Value) -> DefResult<()> {
    for frame in scopes.iter_mut().rev() {
        if frame.vars.contains_key(name) {
            if frame.consts.contains(name) {
                return Err(DefError::Runtime(format!(
                    "cannot assign to const variable '{name}'"
                )));
            }
            frame.vars.insert(name.to_string(), value);
            return Ok(());
        }
    }

    Err(DefError::Runtime(format!(
        "invalid assignment: undefined local variable '{name}'"
    )))
}

fn format_predicate(expr: &Expression) -> String {
    match expr {
        Expression::Identifier(name) => name.clone(),
        Expression::Integer(n) => n.to_string(),
        Expression::Float(f) => f.to_string(),
        Expression::String(s) => format!("\"{s}\""),
        Expression::Boolean(b) => b.to_string(),
        Expression::Binary {
            left,
            operator,
            right,
        } => {
            let op = match operator {
                BinaryOperator::Equal => "==",
                BinaryOperator::NotEqual => "!=",
                BinaryOperator::Greater => ">",
                BinaryOperator::GreaterEqual => ">=",
                BinaryOperator::Less => "<",
                BinaryOperator::LessEqual => "<=",
                BinaryOperator::And => "and",
                BinaryOperator::Or => "or",
                BinaryOperator::Add => "+",
                BinaryOperator::Subtract => "-",
                BinaryOperator::Multiply => "*",
                BinaryOperator::Divide => "/",
                BinaryOperator::Modulo => "%",
            };
            format!("{} {op} {}", format_predicate(left), format_predicate(right))
        }
        Expression::Unary {
            operator: UnaryOperator::Not,
            expression,
        } => format!("not {}", format_predicate(expression)),
        _ => "...".to_string(),
    }
}

#[cfg(test)]
mod tests;
