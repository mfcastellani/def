use crate::ast::{
    Assignment, AssignmentOperator, AssignmentTarget, BinaryOperator, EnvVarsLoad, Expression,
    ForLoop, FunctionDefinition, IfStatement, ImportDefinition, MatchArm, MatchPattern, Parameter,
    Program, Statement, Type, UnaryOperator, VariableDefinition,
};
use crate::error::{DefError, DefResult};
use crate::lexer::Token;

pub struct Parser {
    tokens: Vec<Token>,
    position: usize,
    parsing_match_value: bool,
    parsing_block_header: bool,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            position: 0,
            parsing_match_value: false,
            parsing_block_header: false,
        }
    }

    pub fn parse_program(&mut self) -> DefResult<Program> {
        let mut statements = Vec::new();

        self.skip_newlines();
        while !self.is_at_end() {
            statements.push(self.parse_statement()?);
            self.skip_newlines();
        }

        Ok(Program { statements })
    }

    fn parse_statement(&mut self) -> DefResult<Statement> {
        if self.matches(&Token::Def) {
            return self.parse_def_statement();
        }

        if self.matches(&Token::For) {
            return self.parse_for_loop();
        }

        if self.matches(&Token::If) {
            return self.parse_if_statement();
        }

        if self.is_assignment_start() {
            return self.parse_assignment();
        }

        Ok(Statement::Expression(self.parse_expression()?))
    }

    fn parse_assignment(&mut self) -> DefResult<Statement> {
        let target = self.parse_assignment_target()?;
        let operator = self.parse_assignment_operator()?;
        let expression = self.parse_expression()?;

        Ok(Statement::Assignment(Assignment {
            target,
            operator,
            expression,
        }))
    }

    fn parse_assignment_operator(&mut self) -> DefResult<AssignmentOperator> {
        match self.advance() {
            Token::Equal => Ok(AssignmentOperator::Assign),
            Token::PlusEqual => Ok(AssignmentOperator::AddAssign),
            Token::MinusEqual => Ok(AssignmentOperator::SubtractAssign),
            token => Err(DefError::Parse(format!(
                "expected assignment operator, found {token:?}"
            ))),
        }
    }

    fn parse_assignment_target(&mut self) -> DefResult<AssignmentTarget> {
        let name = self.consume_identifier("expected assignment target")?;

        if self.matches(&Token::Dot) {
            let member = self.consume_identifier("expected member name after '.'")?;
            return Ok(AssignmentTarget::Member {
                object: name,
                member,
            });
        }

        Ok(AssignmentTarget::Identifier(name))
    }

    fn parse_def_statement(&mut self) -> DefResult<Statement> {
        let name = self.consume_identifier("expected identifier after 'def'")?;
        self.consume(&Token::As, "expected 'as' after identifier")?;

        if self.matches(&Token::Function) {
            return self.parse_function_definition(name);
        }

        if self.matches(&Token::Imported) {
            return self.parse_import_definition(name);
        }

        if self.matches(&Token::EnvVars) {
            return self.parse_envvars_load(name);
        }

        let type_annotation = self.parse_type()?;
        let initializer = if self.matches(&Token::LeftParen) {
            if self.check(&Token::RightParen)
                && !matches!(type_annotation, Type::Array | Type::Tuple)
            {
                self.advance();
                if self.check(&Token::Dot) {
                    let base = Expression::String(String::new());
                    let expression = self.parse_postfix_expression(base)?;
                    return Ok(Statement::VariableDefinition(VariableDefinition {
                        name,
                        type_annotation,
                        initializer: Some(expression),
                    }));
                }
                return Ok(Statement::VariableDefinition(VariableDefinition {
                    name,
                    type_annotation,
                    initializer: None,
                }));
            }

            Some(self.parse_variable_initializer(&type_annotation)?)
        } else {
            None
        };

        Ok(Statement::VariableDefinition(VariableDefinition {
            name,
            type_annotation,
            initializer,
        }))
    }

    fn parse_variable_initializer(&mut self, type_annotation: &Type) -> DefResult<Expression> {
        match type_annotation {
            Type::Array => Ok(Expression::Array(
                self.parse_expression_list_until_right_paren()?,
            )),
            Type::Tuple => Ok(Expression::Tuple(
                self.parse_expression_list_until_right_paren()?,
            )),
            _ => {
                self.skip_newlines();
                if self.check(&Token::Def) {
                    return Ok(Expression::Block(
                        self.parse_statement_block_until_right_paren("variable initializer")?,
                    ));
                }
                let expression = self.parse_expression()?;
                self.skip_newlines();
                self.consume(
                    &Token::RightParen,
                    "expected ')' after variable initializer",
                )?;
                Ok(expression)
            }
        }
    }

    fn parse_for_loop(&mut self) -> DefResult<Statement> {
        let variable = self.consume_identifier("expected identifier after 'for'")?;
        self.consume(&Token::In, "expected 'in' after for variable")?;
        let iterable = self.parse_block_header_expression()?;
        self.skip_newlines();
        self.consume(&Token::LeftParen, "expected '(' before for body")?;
        let body = self.parse_statement_block_until_right_paren("for body")?;

        Ok(Statement::ForLoop(ForLoop {
            variable,
            iterable,
            body,
        }))
    }

    fn parse_if_statement(&mut self) -> DefResult<Statement> {
        let condition = self.parse_block_header_expression()?;
        self.skip_newlines();
        self.consume(&Token::LeftParen, "expected '(' before if body")?;
        let then_body = self.parse_statement_block_until_right_paren("if body")?;
        self.skip_newlines();

        let mut else_body = Vec::new();
        if self.matches(&Token::Else) {
            self.skip_newlines();
            self.consume(&Token::LeftParen, "expected '(' before else body")?;
            else_body = self.parse_statement_block_until_right_paren("else body")?;
        }

        Ok(Statement::IfStatement(IfStatement {
            condition,
            then_body,
            else_body,
        }))
    }

    fn parse_import_definition(&mut self, name: String) -> DefResult<Statement> {
        self.consume(&Token::LeftParen, "expected '(' after 'imported'")?;
        let path = match self.advance() {
            Token::String(path) => path,
            token => {
                return Err(DefError::Parse(format!(
                    "expected import path string, found {token:?}"
                )));
            }
        };
        self.consume(&Token::RightParen, "expected ')' after import path")?;

        Ok(Statement::ImportDefinition(ImportDefinition { name, path }))
    }

    fn parse_envvars_load(&mut self, name: String) -> DefResult<Statement> {
        self.consume(&Token::LeftParen, "expected '(' after 'envvars'")?;
        let path = match self.advance() {
            Token::String(path) => path,
            token => {
                return Err(DefError::Parse(format!(
                    "expected env file path string, found {token:?}"
                )));
            }
        };
        self.consume(&Token::RightParen, "expected ')' after env file path")?;

        Ok(Statement::EnvVarsLoad(EnvVarsLoad { name, path }))
    }

    fn parse_function_definition(&mut self, name: String) -> DefResult<Statement> {
        self.consume(&Token::LeftParen, "expected '(' after 'function'")?;
        let mut params = Vec::new();

        if !self.check(&Token::RightParen) {
            loop {
                let param_name = self.consume_identifier("expected parameter name")?;
                self.consume(&Token::As, "expected 'as' after parameter name")?;
                let type_annotation = self.parse_type()?;
                params.push(Parameter {
                    name: param_name,
                    type_annotation,
                });

                if !self.matches(&Token::Comma) {
                    break;
                }
            }
        }

        self.consume(&Token::RightParen, "expected ')' after function parameters")?;
        self.skip_newlines();
        self.consume(&Token::LeftParen, "expected '(' before function body")?;
        let body = self.parse_statement_block_until_right_paren("function body")?;

        Ok(Statement::FunctionDefinition(FunctionDefinition {
            name,
            params,
            body,
        }))
    }

    fn parse_statement_block_until_right_paren(
        &mut self,
        context: &str,
    ) -> DefResult<Vec<Statement>> {
        let mut body = Vec::new();
        self.skip_newlines();
        while !self.check(&Token::RightParen) && !self.is_at_end() {
            body.push(self.parse_statement()?);
            self.skip_newlines();
        }

        self.consume(&Token::RightParen, &format!("expected ')' after {context}"))?;
        Ok(body)
    }

    fn parse_block_header_expression(&mut self) -> DefResult<Expression> {
        let was_parsing_block_header = self.parsing_block_header;
        self.parsing_block_header = true;
        let expression = self.parse_expression();
        self.parsing_block_header = was_parsing_block_header;
        expression
    }

    fn parse_type(&mut self) -> DefResult<Type> {
        match self.advance() {
            Token::TypeInteger => Ok(Type::Integer),
            Token::TypeFloat => Ok(Type::Float),
            Token::TypeString => Ok(Type::String),
            Token::TypeBoolean => Ok(Type::Boolean),
            Token::TypeArray => Ok(Type::Array),
            Token::TypeTuple => Ok(Type::Tuple),
            Token::TypeDateTime => Ok(Type::DateTime),
            Token::TypeRequest => Ok(Type::Request),
            Token::Identifier(name) if name == "response" => Ok(Type::Response),
            token => Err(DefError::Parse(format!("expected type, found {token:?}"))),
        }
    }

    fn parse_expression(&mut self) -> DefResult<Expression> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> DefResult<Expression> {
        let mut expression = self.parse_and()?;

        while self.matches(&Token::Or) {
            let right = self.parse_and()?;
            expression = Expression::Binary {
                left: Box::new(expression),
                operator: BinaryOperator::Or,
                right: Box::new(right),
            };
        }

        Ok(expression)
    }

    fn parse_and(&mut self) -> DefResult<Expression> {
        let mut expression = self.parse_equality()?;

        while self.matches(&Token::And) {
            let right = self.parse_equality()?;
            expression = Expression::Binary {
                left: Box::new(expression),
                operator: BinaryOperator::And,
                right: Box::new(right),
            };
        }

        Ok(expression)
    }

    fn parse_equality(&mut self) -> DefResult<Expression> {
        let mut expression = self.parse_comparison()?;

        while self.matches(&Token::EqualEqual) || self.matches(&Token::BangEqual) {
            let operator = match self.previous() {
                Token::EqualEqual => BinaryOperator::Equal,
                Token::BangEqual => BinaryOperator::NotEqual,
                _ => unreachable!(),
            };
            let right = self.parse_comparison()?;
            expression = Expression::Binary {
                left: Box::new(expression),
                operator,
                right: Box::new(right),
            };
        }

        Ok(expression)
    }

    fn parse_comparison(&mut self) -> DefResult<Expression> {
        let mut expression = self.parse_term()?;

        while self.matches(&Token::Greater)
            || self.matches(&Token::GreaterEqual)
            || self.matches(&Token::Less)
            || self.matches(&Token::LessEqual)
        {
            let operator = match self.previous() {
                Token::Greater => BinaryOperator::Greater,
                Token::GreaterEqual => BinaryOperator::GreaterEqual,
                Token::Less => BinaryOperator::Less,
                Token::LessEqual => BinaryOperator::LessEqual,
                _ => unreachable!(),
            };
            let right = self.parse_term()?;
            expression = Expression::Binary {
                left: Box::new(expression),
                operator,
                right: Box::new(right),
            };
        }

        Ok(expression)
    }

    fn parse_term(&mut self) -> DefResult<Expression> {
        let mut expression = self.parse_factor()?;

        while self.matches(&Token::Plus) || self.matches(&Token::Minus) {
            let operator = match self.previous() {
                Token::Plus => BinaryOperator::Add,
                Token::Minus => BinaryOperator::Subtract,
                _ => unreachable!(),
            };
            let right = self.parse_factor()?;
            expression = Expression::Binary {
                left: Box::new(expression),
                operator,
                right: Box::new(right),
            };
        }

        Ok(expression)
    }

    fn parse_factor(&mut self) -> DefResult<Expression> {
        let mut expression = self.parse_unary()?;

        while self.matches(&Token::Star)
            || self.matches(&Token::Slash)
            || self.matches(&Token::Percent)
        {
            let operator = match self.previous() {
                Token::Star => BinaryOperator::Multiply,
                Token::Slash => BinaryOperator::Divide,
                Token::Percent => BinaryOperator::Modulo,
                _ => unreachable!(),
            };
            let right = self.parse_unary()?;
            expression = Expression::Binary {
                left: Box::new(expression),
                operator,
                right: Box::new(right),
            };
        }

        Ok(expression)
    }

    fn parse_unary(&mut self) -> DefResult<Expression> {
        if self.matches(&Token::Not) {
            let expression = self.parse_unary()?;
            return Ok(Expression::Unary {
                operator: UnaryOperator::Not,
                expression: Box::new(expression),
            });
        }

        if self.matches(&Token::Minus) {
            let right = self.parse_unary()?;
            return Ok(Expression::Binary {
                left: Box::new(Expression::Integer(0)),
                operator: BinaryOperator::Subtract,
                right: Box::new(right),
            });
        }

        self.parse_primary()
    }

    fn parse_primary(&mut self) -> DefResult<Expression> {
        let expression = match self.advance() {
            Token::Integer(value) => Expression::Integer(value),
            Token::Float(value) => Expression::Float(value),
            Token::String(value) => Expression::String(value),
            Token::Boolean(value) => Expression::Boolean(value),
            Token::Match => return self.parse_match_expression(),
            Token::TypeArray => {
                self.consume(&Token::LeftParen, "expected '(' after 'array'")?;
                Expression::Array(self.parse_expression_list_until_right_paren()?)
            }
            Token::TypeTuple => {
                self.consume(&Token::LeftParen, "expected '(' after 'tuple'")?;
                Expression::Tuple(self.parse_expression_list_until_right_paren()?)
            }
            Token::TypeRequest => {
                self.consume(&Token::LeftParen, "expected '(' after 'request'")?;
                let method = match self.advance() {
                    Token::Identifier(method) => method,
                    Token::String(method) => method,
                    token => {
                        return Err(DefError::Parse(format!(
                            "request expression expects an HTTP method, found {token:?}"
                        )));
                    }
                };
                self.consume(&Token::RightParen, "expected ')' after request method")?;
                Expression::Request { method }
            }
            Token::Identifier(name) => {
                if self.check(&Token::LeftParen)
                    && !(self.parsing_match_value && self.left_paren_starts_match_arms())
                    && !(self.parsing_block_header && self.left_paren_starts_statement_block())
                {
                    self.advance();
                    let args = self.parse_call_arguments()?;
                    Expression::FunctionCall { name, args }
                } else {
                    Expression::Identifier(name)
                }
            }
            Token::LeftParen => {
                let expression = self.parse_expression()?;
                self.consume(&Token::RightParen, "expected ')' after expression")?;
                expression
            }
            token => Err(DefError::Parse(format!(
                "expected expression, found {token:?}"
            )))?,
        };

        self.parse_postfix_expression(expression)
    }

    fn parse_postfix_expression(&mut self, mut expression: Expression) -> DefResult<Expression> {
        loop {
            let before_newlines = self.position;
            self.skip_newlines();
            if self.matches(&Token::Dot) {
                let member = self.consume_identifier("expected member name after '.'")?;
                expression = if self.matches(&Token::LeftParen) {
                    let args = self.parse_call_arguments()?;
                    Expression::MemberFunctionCall {
                        object: Box::new(expression),
                        name: member,
                        args,
                    }
                } else {
                    Expression::MemberAccess {
                        object: Box::new(expression),
                        member,
                    }
                };
                continue;
            }

            if self.matches(&Token::LeftBracket) {
                let index = self.parse_expression()?;
                self.consume(&Token::RightBracket, "expected ']' after index expression")?;
                expression = Expression::Index {
                    object: Box::new(expression),
                    index: Box::new(index),
                };
                continue;
            }

            self.position = before_newlines;
            break;
        }

        Ok(expression)
    }

    fn parse_call_arguments(&mut self) -> DefResult<Vec<Expression>> {
        self.parse_expression_list_until_right_paren()
    }

    fn parse_expression_list_until_right_paren(&mut self) -> DefResult<Vec<Expression>> {
        let mut args = Vec::new();
        self.skip_newlines();

        if self.matches(&Token::RightParen) {
            return Ok(args);
        }

        loop {
            args.push(self.parse_expression()?);
            self.skip_newlines();

            if !self.matches(&Token::Comma) {
                break;
            }

            self.skip_newlines();
            if self.check(&Token::RightParen) {
                break;
            }
        }

        self.consume(&Token::RightParen, "expected ')' after expression list")?;
        Ok(args)
    }

    fn is_assignment_start(&self) -> bool {
        if !matches!(self.peek(), Token::Identifier(_)) {
            return false;
        }

        if self.check_next(&Token::Equal) {
            return true;
        }
        if self.check_next(&Token::PlusEqual) || self.check_next(&Token::MinusEqual) {
            return true;
        }

        self.check_next(&Token::Dot)
            && matches!(self.peek_at(2), Token::Identifier(_))
            && matches!(
                self.peek_at(3),
                Token::Equal | Token::PlusEqual | Token::MinusEqual
            )
    }

    fn parse_match_expression(&mut self) -> DefResult<Expression> {
        let was_parsing_match_value = self.parsing_match_value;
        self.parsing_match_value = true;
        let value = self.parse_expression();
        self.parsing_match_value = was_parsing_match_value;
        let value = value?;
        self.skip_newlines();
        self.consume(&Token::LeftParen, "expected '(' after match value")?;
        self.skip_newlines();

        let mut arms = Vec::new();
        while !self.check(&Token::RightParen) && !self.is_at_end() {
            let pattern = self.parse_match_pattern()?;
            self.consume(&Token::FatArrow, "expected '=>' after match pattern")?;
            let expression = self.parse_expression()?;
            arms.push(MatchArm {
                pattern,
                expression,
            });

            self.skip_newlines();
            if !self.matches(&Token::Comma) {
                break;
            }
            self.skip_newlines();
        }

        self.consume(&Token::RightParen, "expected ')' after match arms")?;

        Ok(Expression::Match {
            value: Box::new(value),
            arms,
        })
    }

    fn left_paren_starts_match_arms(&self) -> bool {
        if !self.check(&Token::LeftParen) {
            return false;
        }

        let mut position = self.position + 1;
        while matches!(self.peek_at(position - self.position), Token::Newline) {
            position += 1;
        }

        if !matches!(
            self.tokens.get(position),
            Some(Token::Integer(_))
                | Some(Token::Float(_))
                | Some(Token::String(_))
                | Some(Token::Boolean(_))
                | Some(Token::Identifier(_))
        ) {
            return false;
        }

        position += 1;
        while matches!(self.tokens.get(position), Some(Token::Newline)) {
            position += 1;
        }

        matches!(self.tokens.get(position), Some(Token::FatArrow))
    }

    fn left_paren_starts_statement_block(&self) -> bool {
        if !self.check(&Token::LeftParen) {
            return false;
        }

        let mut position = self.position + 1;
        while matches!(self.tokens.get(position), Some(Token::Newline)) {
            position += 1;
        }

        matches!(
            self.tokens.get(position),
            Some(Token::Def)
                | Some(Token::For)
                | Some(Token::If)
                | Some(Token::Identifier(_))
                | Some(Token::TypeArray)
                | Some(Token::TypeTuple)
                | Some(Token::TypeRequest)
                | Some(Token::Match)
                | Some(Token::Integer(_))
                | Some(Token::Float(_))
                | Some(Token::String(_))
                | Some(Token::Boolean(_))
        )
    }

    fn parse_match_pattern(&mut self) -> DefResult<MatchPattern> {
        match self.advance() {
            Token::Integer(value) => Ok(MatchPattern::Integer(value)),
            Token::Float(value) => Ok(MatchPattern::Float(value)),
            Token::String(value) => Ok(MatchPattern::String(value)),
            Token::Boolean(value) => Ok(MatchPattern::Boolean(value)),
            Token::Identifier(name) if name == "_" => Ok(MatchPattern::Wildcard),
            token => Err(DefError::Parse(format!(
                "expected match pattern, found {token:?}"
            ))),
        }
    }

    fn skip_newlines(&mut self) {
        while self.matches(&Token::Newline) {}
    }

    fn consume(&mut self, expected: &Token, message: &str) -> DefResult<()> {
        if self.matches(expected) {
            Ok(())
        } else {
            Err(DefError::Parse(format!(
                "{message}, found {:?}",
                self.peek()
            )))
        }
    }

    fn consume_identifier(&mut self, message: &str) -> DefResult<String> {
        match self.advance() {
            Token::Identifier(name) => Ok(name),
            token => Err(DefError::Parse(format!("{message}, found {token:?}"))),
        }
    }

    fn matches(&mut self, expected: &Token) -> bool {
        if self.check(expected) {
            self.position += 1;
            true
        } else {
            false
        }
    }

    fn check(&self, expected: &Token) -> bool {
        if self.is_at_end() {
            return matches!(expected, Token::Eof);
        }

        std::mem::discriminant(self.peek()) == std::mem::discriminant(expected)
    }

    fn check_next(&self, expected: &Token) -> bool {
        let token = self.peek_at(1);
        std::mem::discriminant(token) == std::mem::discriminant(expected)
    }

    fn advance(&mut self) -> Token {
        let token = self.peek().clone();
        if !self.is_at_end() {
            self.position += 1;
        }
        token
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.position - 1]
    }

    fn peek(&self) -> &Token {
        self.peek_at(0)
    }

    fn peek_at(&self, offset: usize) -> &Token {
        self.tokens
            .get(self.position + offset)
            .unwrap_or(&Token::Eof)
    }

    fn is_at_end(&self) -> bool {
        matches!(self.peek(), Token::Eof)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    fn parse(input: &str) -> Program {
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().unwrap();
        Parser::new(tokens).parse_program().unwrap()
    }

    #[test]
    fn parses_integer_variable_definition() {
        let program = parse("def i as integer");

        assert_eq!(
            program.statements,
            vec![Statement::VariableDefinition(VariableDefinition {
                name: "i".to_string(),
                type_annotation: Type::Integer,
                initializer: None,
            })]
        );
    }

    #[test]
    fn parses_float_variable_definition_with_initializer() {
        let program = parse("def price as float(10.5)");

        assert_eq!(
            program.statements,
            vec![Statement::VariableDefinition(VariableDefinition {
                name: "price".to_string(),
                type_annotation: Type::Float,
                initializer: Some(Expression::Float(10.5)),
            })]
        );
    }

    #[test]
    fn parses_addition_expression() {
        let program = parse("1 + 2");

        assert_eq!(
            program.statements,
            vec![Statement::Expression(Expression::Binary {
                left: Box::new(Expression::Integer(1)),
                operator: BinaryOperator::Add,
                right: Box::new(Expression::Integer(2)),
            })]
        );
    }

    #[test]
    fn parses_sum_function_definition() {
        let program = parse("def sum as function(a as integer, b as integer) (\n  a + b\n)");

        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Statement::FunctionDefinition(function) => {
                assert_eq!(function.name, "sum");
                assert_eq!(function.params.len(), 2);
                assert_eq!(function.body.len(), 1);
            }
            statement => panic!("expected function definition, found {statement:?}"),
        }
    }

    #[test]
    fn parses_function_call() {
        let program = parse("sum(1, 2)");

        assert_eq!(
            program.statements,
            vec![Statement::Expression(Expression::FunctionCall {
                name: "sum".to_string(),
                args: vec![Expression::Integer(1), Expression::Integer(2)],
            })]
        );
    }

    #[test]
    fn parses_boolean_variable_definition() {
        let program = parse("def ok as boolean");

        assert_eq!(
            program.statements,
            vec![Statement::VariableDefinition(VariableDefinition {
                name: "ok".to_string(),
                type_annotation: Type::Boolean,
                initializer: None,
            })]
        );
    }

    #[test]
    fn parses_datetime_variable_definition() {
        let program = parse("def now as datetime");

        assert_eq!(
            program.statements,
            vec![Statement::VariableDefinition(VariableDefinition {
                name: "now".to_string(),
                type_annotation: Type::DateTime,
                initializer: None,
            })]
        );
    }

    #[test]
    fn parses_variable_definition_with_initializer() {
        let program = parse("def ok as boolean(true)");

        assert_eq!(
            program.statements,
            vec![Statement::VariableDefinition(VariableDefinition {
                name: "ok".to_string(),
                type_annotation: Type::Boolean,
                initializer: Some(Expression::Boolean(true)),
            })]
        );
    }

    #[test]
    fn parses_variable_definition_with_empty_initializer_as_default() {
        let program = parse("def message as string()");

        assert_eq!(
            program.statements,
            vec![Statement::VariableDefinition(VariableDefinition {
                name: "message".to_string(),
                type_annotation: Type::String,
                initializer: None,
            })]
        );
    }

    #[test]
    fn parses_response_variable_definition_with_multiline_initializer() {
        let program = parse("def res as response(\n  r.path(\"https://example.com\").do()\n)");

        assert_eq!(
            program.statements,
            vec![Statement::VariableDefinition(VariableDefinition {
                name: "res".to_string(),
                type_annotation: Type::Response,
                initializer: Some(Expression::MemberFunctionCall {
                    object: Box::new(Expression::MemberFunctionCall {
                        object: Box::new(Expression::Identifier("r".to_string())),
                        name: "path".to_string(),
                        args: vec![Expression::String("https://example.com".to_string())],
                    }),
                    name: "do".to_string(),
                    args: Vec::new(),
                }),
            })]
        );
    }

    #[test]
    fn parses_response_variable_definition_with_block_initializer() {
        let program = parse(
            "def res as response(\n  def accept_header as string(\"application/json\")\n  request(GET).with_var(accept_header).do()\n)",
        );

        assert_eq!(program.statements.len(), 1);
        let Statement::VariableDefinition(variable) = &program.statements[0] else {
            panic!("expected variable definition");
        };
        assert_eq!(variable.name, "res");
        assert_eq!(variable.type_annotation, Type::Response);
        let Some(Expression::Block(statements)) = &variable.initializer else {
            panic!("expected block initializer");
        };
        assert_eq!(statements.len(), 2);
    }

    #[test]
    fn parses_request_expression_with_fluent_methods() {
        let program = parse(
            "def res as response(\n  request(GET)\n    .path(\"https://example.com\")\n    .header(tuple(\"Accept\", \"application/json\"))\n    .do()\n)",
        );

        assert_eq!(
            program.statements,
            vec![Statement::VariableDefinition(VariableDefinition {
                name: "res".to_string(),
                type_annotation: Type::Response,
                initializer: Some(Expression::MemberFunctionCall {
                    object: Box::new(Expression::MemberFunctionCall {
                        object: Box::new(Expression::MemberFunctionCall {
                            object: Box::new(Expression::Request {
                                method: "GET".to_string(),
                            }),
                            name: "path".to_string(),
                            args: vec![Expression::String("https://example.com".to_string())],
                        }),
                        name: "header".to_string(),
                        args: vec![Expression::Tuple(vec![
                            Expression::String("Accept".to_string()),
                            Expression::String("application/json".to_string()),
                        ])],
                    }),
                    name: "do".to_string(),
                    args: Vec::new(),
                }),
            })]
        );
    }

    #[test]
    fn parses_variable_definition_initialized_with_function_call() {
        let program = parse("def n as integer(sum(10, 12))");

        assert_eq!(
            program.statements,
            vec![Statement::VariableDefinition(VariableDefinition {
                name: "n".to_string(),
                type_annotation: Type::Integer,
                initializer: Some(Expression::FunctionCall {
                    name: "sum".to_string(),
                    args: vec![Expression::Integer(10), Expression::Integer(12)],
                }),
            })]
        );
    }

    #[test]
    fn parses_assignment() {
        let program = parse("a = 10");

        assert_eq!(
            program.statements,
            vec![Statement::Assignment(Assignment {
                target: AssignmentTarget::Identifier("a".to_string()),
                operator: AssignmentOperator::Assign,
                expression: Expression::Integer(10),
            })]
        );
    }

    #[test]
    fn parses_compound_assignment() {
        let program = parse("a += 10\nb -= 2");

        assert_eq!(
            program.statements,
            vec![
                Statement::Assignment(Assignment {
                    target: AssignmentTarget::Identifier("a".to_string()),
                    operator: AssignmentOperator::AddAssign,
                    expression: Expression::Integer(10),
                }),
                Statement::Assignment(Assignment {
                    target: AssignmentTarget::Identifier("b".to_string()),
                    operator: AssignmentOperator::SubtractAssign,
                    expression: Expression::Integer(2),
                }),
            ]
        );
    }

    #[test]
    fn parses_empty_array_literal() {
        let program = parse("array()");

        assert_eq!(
            program.statements,
            vec![Statement::Expression(Expression::Array(Vec::new()))]
        );
    }

    #[test]
    fn parses_assert_call() {
        let program = parse("assert(1 + 2 == 3)");

        assert_eq!(
            program.statements,
            vec![Statement::Expression(Expression::FunctionCall {
                name: "assert".to_string(),
                args: vec![Expression::Binary {
                    left: Box::new(Expression::Binary {
                        left: Box::new(Expression::Integer(1)),
                        operator: BinaryOperator::Add,
                        right: Box::new(Expression::Integer(2)),
                    }),
                    operator: BinaryOperator::Equal,
                    right: Box::new(Expression::Integer(3)),
                }],
            })]
        );
    }

    #[test]
    fn parses_assert_call_with_boolean_expression() {
        let program = parse("assert(1 + 2 == 3)");

        assert_eq!(
            program.statements,
            vec![Statement::Expression(Expression::FunctionCall {
                name: "assert".to_string(),
                args: vec![Expression::Binary {
                    left: Box::new(Expression::Binary {
                        left: Box::new(Expression::Integer(1)),
                        operator: BinaryOperator::Add,
                        right: Box::new(Expression::Integer(2)),
                    }),
                    operator: BinaryOperator::Equal,
                    right: Box::new(Expression::Integer(3)),
                }],
            })]
        );
    }

    #[test]
    fn parses_if_else_statement() {
        let program = parse("if true (\n  print(\"ok\")\n) else (\n  print(\"error\")\n)");

        assert_eq!(
            program.statements,
            vec![Statement::IfStatement(IfStatement {
                condition: Expression::Boolean(true),
                then_body: vec![Statement::Expression(Expression::FunctionCall {
                    name: "print".to_string(),
                    args: vec![Expression::String("ok".to_string())],
                })],
                else_body: vec![Statement::Expression(Expression::FunctionCall {
                    name: "print".to_string(),
                    args: vec![Expression::String("error".to_string())],
                })],
            })]
        );
    }

    #[test]
    fn parses_match_expression() {
        let program = parse("match n (\n  1 => \"one\",\n  _ => \"other\"\n)");

        assert_eq!(
            program.statements,
            vec![Statement::Expression(Expression::Match {
                value: Box::new(Expression::Identifier("n".to_string())),
                arms: vec![
                    MatchArm {
                        pattern: MatchPattern::Integer(1),
                        expression: Expression::String("one".to_string()),
                    },
                    MatchArm {
                        pattern: MatchPattern::Wildcard,
                        expression: Expression::String("other".to_string()),
                    },
                ],
            })]
        );
    }

    #[test]
    fn parses_assignment_with_match_expression() {
        let program = parse("message = match status (\n  200 => \"ok\",\n  _ => \"unexpected\"\n)");

        assert_eq!(
            program.statements,
            vec![Statement::Assignment(Assignment {
                target: AssignmentTarget::Identifier("message".to_string()),
                operator: AssignmentOperator::Assign,
                expression: Expression::Match {
                    value: Box::new(Expression::Identifier("status".to_string())),
                    arms: vec![
                        MatchArm {
                            pattern: MatchPattern::Integer(200),
                            expression: Expression::String("ok".to_string()),
                        },
                        MatchArm {
                            pattern: MatchPattern::Wildcard,
                            expression: Expression::String("unexpected".to_string()),
                        },
                    ],
                },
            })]
        );
    }

    #[test]
    fn parses_import_definition() {
        let program = parse("def math as imported(\"imports/math\")");

        assert_eq!(
            program.statements,
            vec![Statement::ImportDefinition(ImportDefinition {
                name: "math".to_string(),
                path: "imports/math".to_string(),
            })]
        );
    }

    #[test]
    fn parses_member_function_call() {
        let program = parse("math.add(10, 12)");

        assert_eq!(
            program.statements,
            vec![Statement::Expression(Expression::MemberFunctionCall {
                object: Box::new(Expression::Identifier("math".to_string())),
                name: "add".to_string(),
                args: vec![Expression::Integer(10), Expression::Integer(12)],
            })]
        );
    }

    #[test]
    fn parses_chained_member_function_call() {
        let program = parse("r.path(\"https://example.com\").do()");

        assert_eq!(
            program.statements,
            vec![Statement::Expression(Expression::MemberFunctionCall {
                object: Box::new(Expression::MemberFunctionCall {
                    object: Box::new(Expression::Identifier("r".to_string())),
                    name: "path".to_string(),
                    args: vec![Expression::String("https://example.com".to_string())],
                }),
                name: "do".to_string(),
                args: Vec::new(),
            })]
        );
    }

    #[test]
    fn parses_member_assignment() {
        let program = parse("math.variable = \"Marcelo\"");

        assert_eq!(
            program.statements,
            vec![Statement::Assignment(Assignment {
                target: AssignmentTarget::Member {
                    object: "math".to_string(),
                    member: "variable".to_string(),
                },
                operator: AssignmentOperator::Assign,
                expression: Expression::String("Marcelo".to_string()),
            })]
        );
    }

    #[test]
    fn parses_equality_expression() {
        let program = parse("1 + 2 == 3");

        assert_eq!(
            program.statements,
            vec![Statement::Expression(Expression::Binary {
                left: Box::new(Expression::Binary {
                    left: Box::new(Expression::Integer(1)),
                    operator: BinaryOperator::Add,
                    right: Box::new(Expression::Integer(2)),
                }),
                operator: BinaryOperator::Equal,
                right: Box::new(Expression::Integer(3)),
            })]
        );
    }

    #[test]
    fn parses_comparison_expression() {
        let program = parse("1 + 2 >= 3");

        assert_eq!(
            program.statements,
            vec![Statement::Expression(Expression::Binary {
                left: Box::new(Expression::Binary {
                    left: Box::new(Expression::Integer(1)),
                    operator: BinaryOperator::Add,
                    right: Box::new(Expression::Integer(2)),
                }),
                operator: BinaryOperator::GreaterEqual,
                right: Box::new(Expression::Integer(3)),
            })]
        );
    }

    #[test]
    fn parses_not_equal_expression() {
        let program = parse("true != false");

        assert_eq!(
            program.statements,
            vec![Statement::Expression(Expression::Binary {
                left: Box::new(Expression::Boolean(true)),
                operator: BinaryOperator::NotEqual,
                right: Box::new(Expression::Boolean(false)),
            })]
        );
    }

    #[test]
    fn parses_boolean_operator_expression() {
        let program = parse("true and false or true");

        assert_eq!(
            program.statements,
            vec![Statement::Expression(Expression::Binary {
                left: Box::new(Expression::Binary {
                    left: Box::new(Expression::Boolean(true)),
                    operator: BinaryOperator::And,
                    right: Box::new(Expression::Boolean(false)),
                }),
                operator: BinaryOperator::Or,
                right: Box::new(Expression::Boolean(true)),
            })]
        );
    }

    #[test]
    fn parses_not_expression() {
        let program = parse("not true or false");

        assert_eq!(
            program.statements,
            vec![Statement::Expression(Expression::Binary {
                left: Box::new(Expression::Unary {
                    operator: UnaryOperator::Not,
                    expression: Box::new(Expression::Boolean(true)),
                }),
                operator: BinaryOperator::Or,
                right: Box::new(Expression::Boolean(false)),
            })]
        );
    }
}
