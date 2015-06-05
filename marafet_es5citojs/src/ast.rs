#[derive(Clone)]
pub struct Code {
    pub statements: Vec<Statement>,
}

#[derive(Clone)]
pub enum Statement {
    Expr(Expression),
    Return(Expression),
    Function(String, Vec<Param>, Vec<Statement>),
}


#[derive(Clone)]
pub struct Param {
    pub name: String,
    pub default_value: Option<Expression>,
}

#[derive(Clone)]
pub enum Expression {
    Str(String),
    Object(Vec<(String, Expression)>),
    List(Vec<Expression>),
}

