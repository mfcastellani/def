use chrono::{DateTime, Local};

use crate::ast::Type;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    Array(Vec<Value>),
    Tuple { key: String, value: Box<Value> },
    DateTime(DateTime<Local>),
    Request(RequestValue),
    RequestHandle(String),
    Response(ResponseValue),
    Mock(MockValue),
    Uninitialized(Type),
    Nil,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MockValue {
    pub method: String,
    pub url: String,
    pub status: i64,
    pub body: String,
    pub headers: Vec<(String, String)>,
    pub vars: Vec<(String, String)>,
    pub delay_ms: u64,
    pub configured: bool,
    /// Future: path to a snapshot file used by `def server` to replay recorded responses.
    pub snapshot_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum BackoffStrategy {
    #[default]
    None,
    Fixed(u64),
    Linear(u64),
    Exponential(u64),
}

#[derive(Debug, Clone, PartialEq)]
pub struct RequestValue {
    pub method: String,
    pub path: Option<String>,
    pub status: Option<i64>,
    pub headers: Vec<(String, String)>,
    pub query_strings: Vec<(String, String)>,
    pub body: Option<String>,
    pub vars: Vec<(String, String)>,
    pub retries: u32,
    pub backoff: BackoffStrategy,
    pub timeout_ms: Option<u64>,
    pub timeout_message: Option<String>,
    pub mocks: Vec<MockValue>,
    pub snapshot: bool,
    pub mock_with_snapshot: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResponseValue {
    pub status: i64,
    pub body: String,
    pub headers: Vec<(String, String)>,
    pub duration_ms: i64,
    pub method: String,
    pub url: String,
}

impl Value {
    pub fn value_type(&self) -> Option<Type> {
        match self {
            Value::Integer(_) => Some(Type::Integer),
            Value::Float(_) => Some(Type::Float),
            Value::String(_) => Some(Type::String),
            Value::Boolean(_) => Some(Type::Boolean),
            Value::Array(_) => Some(Type::Array),
            Value::Tuple { .. } => Some(Type::Tuple),
            Value::DateTime(_) => Some(Type::DateTime),
            Value::Request(_) => Some(Type::Request),
            Value::RequestHandle(_) => Some(Type::Request),
            Value::Response(_) => Some(Type::Response),
            Value::Mock(_) => Some(Type::Mock),
            Value::Uninitialized(type_annotation) => Some(type_annotation.clone()),
            Value::Nil => None,
        }
    }
}
