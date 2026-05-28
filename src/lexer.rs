use crate::error::{DefError, DefResult};

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Def,
    As,
    Function,
    Imported,
    EnvVars,
    Match,
    For,
    In,
    While,
    Do,
    If,
    Else,
    And,
    Or,
    Not,
    TypeInteger,
    TypeFloat,
    TypeString,
    TypeBoolean,
    TypeArray,
    TypeTuple,
    TypeDateTime,
    TypeRequest,
    TypeMock,
    Identifier(String),
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Equal,
    PlusEqual,
    MinusEqual,
    EqualEqual,
    BangEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    FatArrow,
    Dot,
    LeftBracket,
    RightBracket,
    LeftParen,
    RightParen,
    Comma,
    Newline,
    Eof,
}

pub struct Lexer {
    input: Vec<char>,
    position: usize,
    line: usize,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            position: 0,
            line: 1,
        }
    }

    pub fn tokenize(&mut self) -> DefResult<Vec<(Token, usize)>> {
        let mut tokens = Vec::new();

        while let Some(ch) = self.peek() {
            let line = self.line;
            match ch {
                ' ' | '\t' | '\r' => {
                    self.advance();
                }
                '\n' => {
                    self.advance();
                    self.line += 1;
                    tokens.push((Token::Newline, line));
                }
                '+' => {
                    if self.peek_next() == Some('=') {
                        self.advance();
                        self.advance();
                        tokens.push((Token::PlusEqual, line));
                    } else {
                        self.advance();
                        tokens.push((Token::Plus, line));
                    }
                }
                '-' => {
                    if self.peek_next() == Some('=') {
                        self.advance();
                        self.advance();
                        tokens.push((Token::MinusEqual, line));
                    } else {
                        self.advance();
                        tokens.push((Token::Minus, line));
                    }
                }
                '*' => {
                    self.advance();
                    tokens.push((Token::Star, line));
                }
                '%' => {
                    self.advance();
                    tokens.push((Token::Percent, line));
                }
                '/' => {
                    if self.peek_next() == Some('/') {
                        self.skip_comment();
                    } else {
                        self.advance();
                        tokens.push((Token::Slash, line));
                    }
                }
                '=' => {
                    if self.peek_next() == Some('>') {
                        self.advance();
                        self.advance();
                        tokens.push((Token::FatArrow, line));
                    } else if self.peek_next() == Some('=') {
                        self.advance();
                        self.advance();
                        tokens.push((Token::EqualEqual, line));
                    } else {
                        self.advance();
                        tokens.push((Token::Equal, line));
                    }
                }
                '!' => {
                    if self.peek_next() == Some('=') {
                        self.advance();
                        self.advance();
                        tokens.push((Token::BangEqual, line));
                    } else {
                        return Err(DefError::Lex(format!(
                            "unexpected character '!' at line {line}"
                        )));
                    }
                }
                '>' => {
                    if self.peek_next() == Some('=') {
                        self.advance();
                        self.advance();
                        tokens.push((Token::GreaterEqual, line));
                    } else {
                        self.advance();
                        tokens.push((Token::Greater, line));
                    }
                }
                '<' => {
                    if self.peek_next() == Some('=') {
                        self.advance();
                        self.advance();
                        tokens.push((Token::LessEqual, line));
                    } else {
                        self.advance();
                        tokens.push((Token::Less, line));
                    }
                }
                '(' => {
                    self.advance();
                    tokens.push((Token::LeftParen, line));
                }
                '[' => {
                    self.advance();
                    tokens.push((Token::LeftBracket, line));
                }
                ']' => {
                    self.advance();
                    tokens.push((Token::RightBracket, line));
                }
                ')' => {
                    self.advance();
                    tokens.push((Token::RightParen, line));
                }
                ',' => {
                    self.advance();
                    tokens.push((Token::Comma, line));
                }
                '.' => {
                    self.advance();
                    tokens.push((Token::Dot, line));
                }
                '"' => tokens.push((self.read_string(line)?, line)),
                ch if ch.is_ascii_digit() => tokens.push((self.read_numeric_literal()?, line)),
                ch if is_identifier_start(ch) => tokens.push((self.read_identifier(), line)),
                other => {
                    return Err(DefError::Lex(format!(
                        "unexpected character '{other}' at line {line}"
                    )));
                }
            }
        }

        tokens.push((Token::Eof, self.line));
        Ok(tokens)
    }

    fn read_string(&mut self, start_line: usize) -> DefResult<Token> {
        self.advance();
        let mut value = String::new();

        while let Some(ch) = self.peek() {
            if ch == '"' {
                self.advance();
                return Ok(Token::String(value));
            }
            value.push(ch);
            self.advance();
        }

        Err(DefError::Lex(format!(
            "unterminated string literal at line {start_line}"
        )))
    }

    fn read_numeric_literal(&mut self) -> DefResult<Token> {
        let start = self.position;
        let mut seen_dot = false;

        while let Some(ch) = self.peek() {
            if ch == '.' && !seen_dot {
                seen_dot = true;
                self.advance();
            } else if ch.is_ascii_digit() {
                self.advance();
            } else {
                break;
            }
        }

        let text: String = self.input[start..self.position].iter().collect();
        if seen_dot {
            let value = text
                .parse::<f64>()
                .map_err(|_| DefError::Lex(format!("invalid float literal '{text}'")))?;
            Ok(Token::Float(value))
        } else {
            let value = text
                .parse::<i64>()
                .map_err(|_| DefError::Lex(format!("invalid integer literal '{text}'")))?;
            Ok(Token::Integer(value))
        }
    }

    fn read_identifier(&mut self) -> Token {
        let start = self.position;

        while let Some(ch) = self.peek() {
            if is_identifier_part(ch) {
                self.advance();
            } else {
                break;
            }
        }

        let text: String = self.input[start..self.position].iter().collect();
        match text.as_str() {
            "def" => Token::Def,
            "as" => Token::As,
            "function" => Token::Function,
            "imported" => Token::Imported,
            "envvars" => Token::EnvVars,
            "match" => Token::Match,
            "for" => Token::For,
            "in" => Token::In,
            "while" => Token::While,
            "do" => Token::Do,
            "if" => Token::If,
            "else" => Token::Else,
            "and" => Token::And,
            "or" => Token::Or,
            "not" => Token::Not,
            "integer" => Token::TypeInteger,
            "float" => Token::TypeFloat,
            "string" => Token::TypeString,
            "boolean" => Token::TypeBoolean,
            "array" => Token::TypeArray,
            "tuple" => Token::TypeTuple,
            "datetime" => Token::TypeDateTime,
            "request" => Token::TypeRequest,
            "mock" => Token::TypeMock,
            "true" => Token::Boolean(true),
            "false" => Token::Boolean(false),
            _ => Token::Identifier(text),
        }
    }

    fn peek(&self) -> Option<char> {
        self.input.get(self.position).copied()
    }

    fn peek_next(&self) -> Option<char> {
        self.input.get(self.position + 1).copied()
    }

    fn advance(&mut self) {
        self.position += 1;
    }

    fn skip_comment(&mut self) {
        while let Some(ch) = self.peek() {
            if ch == '\n' {
                break;
            }
            self.advance();
        }
    }
}

fn is_identifier_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_'
}

fn is_identifier_part(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokenize(input: &str) -> Vec<Token> {
        Lexer::new(input)
            .tokenize()
            .unwrap()
            .into_iter()
            .map(|(t, _)| t)
            .collect()
    }

    #[test]
    fn lexes_integer_variable_definition() {
        assert_eq!(
            tokenize("def i as integer"),
            vec![
                Token::Def,
                Token::Identifier("i".to_string()),
                Token::As,
                Token::TypeInteger,
                Token::Eof
            ]
        );
    }

    #[test]
    fn lexes_arithmetic_expression() {
        assert_eq!(
            tokenize("1 + 2"),
            vec![
                Token::Integer(1),
                Token::Plus,
                Token::Integer(2),
                Token::Eof
            ]
        );
    }

    #[test]
    fn lexes_float_type_and_literal() {
        assert_eq!(
            tokenize("def price as float(10.5)"),
            vec![
                Token::Def,
                Token::Identifier("price".to_string()),
                Token::As,
                Token::TypeFloat,
                Token::LeftParen,
                Token::Float(10.5),
                Token::RightParen,
                Token::Eof
            ]
        );
    }

    #[test]
    fn lexes_boolean_variable_definition_and_literal() {
        assert_eq!(
            tokenize("def ok as boolean\ntrue"),
            vec![
                Token::Def,
                Token::Identifier("ok".to_string()),
                Token::As,
                Token::TypeBoolean,
                Token::Newline,
                Token::Boolean(true),
                Token::Eof
            ]
        );
    }

    #[test]
    fn lexes_datetime_variable_definition() {
        assert_eq!(
            tokenize("def now as datetime"),
            vec![
                Token::Def,
                Token::Identifier("now".to_string()),
                Token::As,
                Token::TypeDateTime,
                Token::Eof,
            ]
        );
    }

    #[test]
    fn lexes_assignment() {
        assert_eq!(
            tokenize("a = 10"),
            vec![
                Token::Identifier("a".to_string()),
                Token::Equal,
                Token::Integer(10),
                Token::Eof
            ]
        );
    }

    #[test]
    fn lexes_compound_and_equality_operators() {
        assert_eq!(
            tokenize("a += 10\nb -= 2\na == b\na != b\na > b\na >= b\na < b\na <= b"),
            vec![
                Token::Identifier("a".to_string()),
                Token::PlusEqual,
                Token::Integer(10),
                Token::Newline,
                Token::Identifier("b".to_string()),
                Token::MinusEqual,
                Token::Integer(2),
                Token::Newline,
                Token::Identifier("a".to_string()),
                Token::EqualEqual,
                Token::Identifier("b".to_string()),
                Token::Newline,
                Token::Identifier("a".to_string()),
                Token::BangEqual,
                Token::Identifier("b".to_string()),
                Token::Newline,
                Token::Identifier("a".to_string()),
                Token::Greater,
                Token::Identifier("b".to_string()),
                Token::Newline,
                Token::Identifier("a".to_string()),
                Token::GreaterEqual,
                Token::Identifier("b".to_string()),
                Token::Newline,
                Token::Identifier("a".to_string()),
                Token::Less,
                Token::Identifier("b".to_string()),
                Token::Newline,
                Token::Identifier("a".to_string()),
                Token::LessEqual,
                Token::Identifier("b".to_string()),
                Token::Eof
            ]
        );
    }

    #[test]
    fn lexes_boolean_operators() {
        assert_eq!(
            tokenize("not true and false or true"),
            vec![
                Token::Not,
                Token::Boolean(true),
                Token::And,
                Token::Boolean(false),
                Token::Or,
                Token::Boolean(true),
                Token::Eof
            ]
        );
    }

    #[test]
    fn ignores_comments_until_newline() {
        assert_eq!(
            tokenize("def a as integer // ignored\na = 10"),
            vec![
                Token::Def,
                Token::Identifier("a".to_string()),
                Token::As,
                Token::TypeInteger,
                Token::Newline,
                Token::Identifier("a".to_string()),
                Token::Equal,
                Token::Integer(10),
                Token::Eof
            ]
        );
    }

    #[test]
    fn lexes_match_expression() {
        assert_eq!(
            tokenize("match n (\n  1 => \"one\",\n  _ => \"other\"\n)"),
            vec![
                Token::Match,
                Token::Identifier("n".to_string()),
                Token::LeftParen,
                Token::Newline,
                Token::Integer(1),
                Token::FatArrow,
                Token::String("one".to_string()),
                Token::Comma,
                Token::Newline,
                Token::Identifier("_".to_string()),
                Token::FatArrow,
                Token::String("other".to_string()),
                Token::Newline,
                Token::RightParen,
                Token::Eof
            ]
        );
    }
}
