use parser_combinators::primitives::{Stream, State, Parser};
use parser_combinators::{ParseResult, parser};
use parser_combinators::combinator::{optional, ParserExt, sep_by, many, many1};
use parser_combinators::combinator::{chainl1, between};

use util::join;

use super::Block;
use super::token::{Token, ParseToken};
use super::token::TokenType as Tok;
use super::token::lift;

type ChainFun = fn(Expression, Expression) -> Expression;


#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub default_value: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Expression {
    Name(String),
    Str(String),
    Num(String),
    New(Box<Expression>),
    Attr(Box<Expression>, String),
    Mul(Box<Expression>, Box<Expression>),
    Div(Box<Expression>, Box<Expression>),
    Add(Box<Expression>, Box<Expression>),
    Sub(Box<Expression>, Box<Expression>),
    Call(Box<Expression>, Vec<Expression>),
}

#[derive(Debug, Clone)]
pub enum LinkDest {
    Stream(Expression),
    Mapping(Expression, Expression),
}

#[derive(Debug, Clone)]
pub enum Link {
    One(String, LinkDest),
    Multi(Vec<(String, Option<String>)>, LinkDest),
}

#[derive(Debug, Clone)]
pub enum Statement {
    Element {
        name: String,
        classes: Vec<(String, Option<Expression>)>,
        attributes: Vec<(String, Expression)>,
        body: Vec<Statement>,
    },
    Text(String),
    Output(Expression),
    Store(String, Expression),
    Link(Vec<Link>),
    Condition(Vec<(Expression, Vec<Statement>)>, Option<Vec<Statement>>),
}


fn param<'a, I: Stream<Item=Token<'a>>>(input: State<I>)
    -> ParseResult<Param, I>
{
    lift(Tok::Ident).and(optional(lift(Tok::Equals)
        .with(lift(Tok::String))))  // TODO(tailhook) more expressions
    .map(|((_, name, _), opt)| Param {
        name: String::from(name),
        default_value: opt.map(|x| String::from(x.1)),
    })
    .parse_state(input)
}

fn dash_name<'a, I: Stream<Item=Token<'a>>>(input: State<I>)
    -> ParseResult<String, I>
{
    sep_by::<Vec<_>, _, _>(
        lift(Tok::Ident).map(ParseToken::into_string),
        lift(Tok::Dash))
    .map(|x| join(x.iter(), "-"))
    .parse_state(input)
}

fn element_start<'a, I: Stream<Item=Token<'a>>>(input: State<I>)
    -> ParseResult<(String, Vec<(String, Option<Expression>)>), I>
{
    let element_head = lift(Tok::Ident)
        .map(ParseToken::into_string)
        .and(many::<Vec<_>, _>(lift(Tok::Dot)
            .with(parser(dash_name))
            .and(optional(lift(Tok::Question)
                .with(between(lift(Tok::OpenParen), lift(Tok::CloseParen),
                    parser(expression)))))
        ));
    let div_head = many1::<Vec<_>, _>(
        lift(Tok::Dot)
        .with(parser(dash_name))
        .and(optional(lift(Tok::Question)
            .with(between(lift(Tok::OpenParen), lift(Tok::CloseParen),
                parser(expression))))))
        .map(|items| (String::from("div"), items));
    element_head
    .or(div_head)
    .parse_state(input)
}

fn attributes<'a, I: Stream<Item=Token<'a>>>(input: State<I>)
    -> ParseResult<Option<Vec<(String, Expression)>>, I>
{
    optional(lift(Tok::OpenBracket)
        .with(sep_by::<Vec<_>, _, _>(
            lift(Tok::Ident).map(ParseToken::into_string)
                .skip(lift(Tok::Equals))
                .and(parser(expression)),
            lift(Tok::Comma)))
        .skip(lift(Tok::CloseBracket)))
    .parse_state(input)
}

fn element<'a, I: Stream<Item=Token<'a>>>(input: State<I>)
    -> ParseResult<Statement, I>
{
    parser(element_start)
    .and(parser(attributes))
    .skip(lift(Tok::Newline))
    .and(parser(chunk))
    .map(|(((name, classes), opt_attributes), opt_body)| Statement::Element {
        name: name,
        classes: classes,
        attributes: opt_attributes.unwrap_or(vec!()),
        body: opt_body.unwrap_or(vec!()),
        })
    .parse_state(input)
}

fn literal<'a, I: Stream<Item=Token<'a>>>(input: State<I>)
    -> ParseResult<Statement, I>
{
    lift(Tok::String).skip(lift(Tok::Newline))
    .map(|tok| Statement::Text(tok.unescape()))
    .parse_state(input)
}

fn expr_params<'a, I: Stream<Item=Token<'a>>>(input: State<I>)
    -> ParseResult<Option<Vec<Expression>>, I>
{
    optional(lift(Tok::OpenParen)
        .with(sep_by::<Vec<_>, _, _>(
            parser(expression),
            lift(Tok::Comma)))
        .skip(lift(Tok::CloseParen)))
    .parse_state(input)
}

fn call<'a, I: Stream<Item=Token<'a>>>(input: State<I>)
    -> ParseResult<Expression, I>
{
    lift(Tok::Ident).map(ParseToken::into_string)
    .and(many::<Vec<_>, _>(lift(Tok::Dot)
        .with(lift(Tok::Ident).map(ParseToken::into_string))))
    .map(|(head, tail)| {
        tail.iter().fold(Expression::Name(head),
            |expr, name| Expression::Attr(Box::new(expr), name.clone()))
    })
    .and(parser(expr_params))
    .map(|(mut expr, opt_paren)| {
        if let Some(paren) = opt_paren {
            expr = Expression::Call(Box::new(expr), paren);
        }
        expr
    })
    .parse_state(input)
}
fn atom<'a, I: Stream<Item=Token<'a>>>(input: State<I>)
    -> ParseResult<Expression, I>
{
    parser(call)
    .or(lift(Tok::New).with(parser(call))
        .map(|x| Expression::New(Box::new(x))))
    .or(lift(Tok::String)
        .map(ParseToken::into_string).map(Expression::Str))
    .or(lift(Tok::Number)
        .map(ParseToken::into_string).map(Expression::Num))
    .parse_state(input)
}

fn multiply(l: Expression, r: Expression) -> Expression {
    Expression::Mul(Box::new(l), Box::new(r))
}
fn divide(l: Expression, r: Expression) -> Expression {
    Expression::Div(Box::new(l), Box::new(r))
}
fn add(l: Expression, r: Expression) -> Expression {
    Expression::Add(Box::new(l), Box::new(r))
}
fn subtract(l: Expression, r: Expression) -> Expression {
    Expression::Sub(Box::new(l), Box::new(r))
}

fn expression<'a, I: Stream<Item=Token<'a>>>(input: State<I>)
    -> ParseResult<Expression, I>
{
    let factor = lift(Tok::Multiply).map(|_| multiply as ChainFun)
                 .or(lift(Tok::Divide).map(|_| divide as ChainFun));
    let sum = lift(Tok::Plus).map(|_| add as ChainFun)
              .or(lift(Tok::Dash).map(|_| subtract as ChainFun));
    let factor = chainl1(parser(atom), factor);
    chainl1(factor, sum)
    .parse_state(input)
}

fn store<'a, I: Stream<Item=Token<'a>>>(input: State<I>)
    -> ParseResult<Statement, I>
{
    lift(Tok::Store)
    .with(lift(Tok::Ident).map(ParseToken::into_string))
    .skip(lift(Tok::Equals))
    .and(parser(expression)).skip(lift(Tok::Newline))
    .map(|(name, value)| Statement::Store(name, value))
    .parse_state(input)
}

fn link_dest<'a, I: Stream<Item=Token<'a>>>(input: State<I>)
    -> ParseResult<LinkDest, I>
{
    parser(expression)
    .and(optional(lift(Tok::ArrowRight)
                  .with(parser(expression))))
    .map(|(x, y)| match y {
        Some(dest) => LinkDest::Mapping(x, dest),
        None => LinkDest::Stream(x),
    })
    .parse_state(input)
}

fn multi_link<'a, I: Stream<Item=Token<'a>>>(input: State<I>)
    -> ParseResult<Link, I>
{
    lift(Tok::OpenBrace)
    .with(sep_by::<Vec<_>, _, _>(
        lift(Tok::Ident).map(ParseToken::into_string)
        .and(optional(lift(Tok::Colon).with(
            lift(Tok::Ident).map(ParseToken::into_string)))),
        lift(Tok::Comma)))
    .skip(lift(Tok::CloseBrace))
    .skip(lift(Tok::Equals))
    .and(parser(link_dest))
    .map(|(lst, dest)| Link::Multi(lst, dest))
    .parse_state(input)
}

fn single_link<'a, I: Stream<Item=Token<'a>>>(input: State<I>)
    -> ParseResult<Link, I>
{
    lift(Tok::Ident)
    .skip(lift(Tok::Equals))
    .and(parser(link_dest))
    .map(|(name, dest)| Link::One(name.into_string(), dest))
    .parse_state(input)
}

fn link<'a, I: Stream<Item=Token<'a>>>(input: State<I>)
    -> ParseResult<Statement, I>
{
    lift(Tok::Link)
    .with(sep_by::<Vec<_>, _, _>(
        parser(single_link).or(parser(multi_link)),
        lift(Tok::Comma)))
    .skip(lift(Tok::Newline))
    .map(Statement::Link)
    .parse_state(input)
}

fn condition<'a, I: Stream<Item=Token<'a>>>(input: State<I>)
    -> ParseResult<Statement, I>
{
    lift(Tok::If)
    .with(parser(expression))
    .skip(lift(Tok::Colon))
    .skip(lift(Tok::Newline))
    .and(parser(chunk))
    .and(optional(many::<Vec<_>,_>(
        lift(Tok::Elif)
        .with(parser(expression))
        .skip(lift(Tok::Colon))
        .skip(lift(Tok::Newline))
        .and(parser(chunk))
        )))
    .and(optional(lift(Tok::Else)
        .skip(lift(Tok::Colon))
        .skip(lift(Tok::Newline))
        .with(parser(chunk))
        ))
    .map(|(((cond, body), opt_elifs), opt_else)| Statement::Condition(
        vec![(cond, body.unwrap_or(vec!()))]
        .into_iter()
        .chain(opt_elifs.map(
            |v| v.into_iter()
                 .map(|(expr, opt_body)| (expr, opt_body.unwrap_or(vec!())))
                 .collect()
            ).unwrap_or(vec!()).into_iter())
        .collect(),
        opt_else.and_then(|x| x)))
    .parse_state(input)
}

fn output<'a, I: Stream<Item=Token<'a>>>(input: State<I>)
    -> ParseResult<Statement, I>
{
    lift(Tok::Equals)
    .with(parser(expression))
    .skip(lift(Tok::Newline))
    .map(Statement::Output)
    .parse_state(input)
}

fn statement<'a, I: Stream<Item=Token<'a>>>(input: State<I>)
    -> ParseResult<Statement, I>
{
    parser(element)
    .or(parser(literal))
    .or(parser(store))
    .or(parser(link))
    .or(parser(condition))
    .or(parser(output))
    .parse_state(input)
}

fn params<'a, I: Stream<Item=Token<'a>>>(input: State<I>)
    -> ParseResult<Option<Vec<Param>>, I>
{
    optional(lift(Tok::OpenParen)
        .with(sep_by::<Vec<_>, _, _>(parser(param), lift(Tok::Comma)))
        .skip(lift(Tok::CloseParen)))
    .parse_state(input)
}

fn events<'a, I: Stream<Item=Token<'a>>>(input: State<I>)
    -> ParseResult<Option<Vec<String>>, I>
{
    optional(lift(Tok::Events)
        .with(sep_by::<Vec<_>, _, _>(
            lift(Tok::Ident).map(ParseToken::into_string),
            lift(Tok::Comma),
        )))
    .parse_state(input)
}

pub fn chunk<'a, I: Stream<Item=Token<'a>>>(input: State<I>)
    -> ParseResult<Option<Vec<Statement>>, I>
{
    optional(
        lift(Tok::Indent)
        .with(many1(parser(statement)))
        .skip(lift(Tok::Dedent)))
    .parse_state(input)
}

pub fn block<'a, I: Stream<Item=Token<'a>>>(input: State<I>)
    -> ParseResult<Block, I>
{
    lift(Tok::Ident).map(ParseToken::into_string)
    .and(parser(params))
    .and(parser(events))
    .skip(lift(Tok::Colon))
    .skip(lift(Tok::Newline))
    .and(parser(chunk))
    .map(|(((name, opt_params), opt_events), opt_stmtlist)| {
        Block::Html {
            name: name,
            params: opt_params.unwrap_or(vec!()),
            events: opt_events.unwrap_or(vec!()),
            statements: opt_stmtlist.unwrap_or(vec!()),
        }
    })
    .parse_state(input)
}

