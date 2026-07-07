#[derive(Debug, Clone)]
pub struct Stmt {
    pub inner: Statement,
    pub line: usize,
}

impl PartialEq for Stmt {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub statements: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    VariableDefinition(VariableDefinition),
    ImportDefinition(ImportDefinition),
    EnvVarsLoad(EnvVarsLoad),
    Assignment(Assignment),
    ForLoop(ForLoop),
    WhileLoop(WhileLoop),
    IfStatement(IfStatement),
    FunctionDefinition(FunctionDefinition),
    Expression(Expression),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ForLoop {
    pub variable: String,
    pub iterable: Expression,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WhileLoop {
    pub condition: Expression,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IfStatement {
    pub condition: Expression,
    pub then_body: Vec<Stmt>,
    pub else_body: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportDefinition {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnvVarsLoad {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VariableDefinition {
    pub name: String,
    pub is_const: bool,
    pub type_annotation: Type,
    pub initializer: Option<Expression>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Assignment {
    pub target: AssignmentTarget,
    pub operator: AssignmentOperator,
    pub expression: Expression,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignmentOperator {
    Assign,
    AddAssign,
    SubtractAssign,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AssignmentTarget {
    Identifier(String),
    Member { object: String, member: String },
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDefinition {
    pub name: String,
    pub params: Vec<Parameter>,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub name: String,
    pub type_annotation: Type,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Integer,
    Float,
    String,
    Boolean,
    Array,
    Tuple,
    DateTime,
    Request,
    Response,
    Mock,
    File,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    Array(Vec<Expression>),
    Tuple(Vec<Expression>),
    Request {
        method: String,
    },
    Mock {
        method: String,
        url: Box<Expression>,
    },
    File {
        mode: String,
    },
    Identifier(String),
    Binary {
        left: Box<Expression>,
        operator: BinaryOperator,
        right: Box<Expression>,
    },
    Unary {
        operator: UnaryOperator,
        expression: Box<Expression>,
    },
    FunctionCall {
        name: String,
        args: Vec<Expression>,
    },
    MemberAccess {
        object: Box<Expression>,
        member: String,
    },
    MemberFunctionCall {
        object: Box<Expression>,
        name: String,
        args: Vec<Expression>,
    },
    Index {
        object: Box<Expression>,
        index: Box<Expression>,
    },
    Match {
        value: Box<Expression>,
        arms: Vec<MatchArm>,
    },
    Block(Vec<Stmt>),
    Range {
        start: Box<Expression>,
        end: Box<Expression>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: MatchPattern,
    pub expression: Expression,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MatchPattern {
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    Wildcard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Equal,
    NotEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOperator {
    Not,
}
