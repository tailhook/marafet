pub use parser::html::Comparator;

#[derive(Clone)]
pub struct Code {
    pub statements: Vec<Statement>,
}

#[derive(Clone)]
pub enum Statement {
    Expr(Expression),
    Return(Expression),
    Function(String, Vec<Param>, Vec<Statement>),
    Var(String, Expression),
}


#[derive(Clone)]
pub struct Param {
    pub name: String,
    pub default_value: Option<Expression>,
}

#[derive(Clone)]
pub enum Expression {
    Str(String),
    Num(String),
    Object(Vec<(String, Expression)>),
    List(Vec<Expression>),
    Name(String),
    Attr(Box<Expression>, String),
    Call(Box<Expression>, Vec<Expression>),
    Function(Option<String>, Vec<Param>, Vec<Statement>),
    AssignAttr(Box<Expression>, String, Box<Expression>),
    Ternary(Box<Expression>, Box<Expression>, Box<Expression>),
    New(Box<Expression>),
    Or(Box<Expression>, Box<Expression>),
    And(Box<Expression>, Box<Expression>),
    Add(Box<Expression>, Box<Expression>),
    Sub(Box<Expression>, Box<Expression>),
    Div(Box<Expression>, Box<Expression>),
    Mul(Box<Expression>, Box<Expression>),
    Comparison(Comparator, Box<Expression>, Box<Expression>),
}

