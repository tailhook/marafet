use parser_combinators::primitives::{Stream, State, Parser};
use parser_combinators::{ParseResult, parser};
use parser_combinators::combinator::{optional, ParserExt, sep_by, many, many1};
use parser_combinators::combinator::{chainl1, between, choice};

use util::join;

use super::Block;
use super::parse_html_expr;
use super::token::{Token, ParseToken};
use super::token::TokenType as Tok;
use super::token::lift;

type ChainFun = fn(Expression, Expression) -> Expression;


#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub default_value: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum Comparator {
    Eq,
    NotEq,
    Less,
    LessEq,
    Greater,
    GreaterEq,
}

#[derive(Debug, Clone)]
pub enum Expression {
    Name(String),
    Str(String),
    Format(Vec<Fmt>),
    Num(String),
    New(Box<Expression>),
    Not(Box<Expression>),
    And(Box<Expression>, Box<Expression>),
    Or(Box<Expression>, Box<Expression>),
    Attr(Box<Expression>, String),
    Mul(Box<Expression>, Box<Expression>),
    Div(Box<Expression>, Box<Expression>),
    Add(Box<Expression>, Box<Expression>),
    Sub(Box<Expression>, Box<Expression>),
    Comparison(Comparator, Box<Expression>, Box<Expression>),
    Item(Box<Expression>, Box<Expression>),
    Call(Box<Expression>, Vec<Expression>),
    Dict(Vec<(String, Expression)>),
    List(Vec<Expression>),
}

#[derive(Debug, Clone)]
pub enum LinkDest {
    Stream(Expression),
    Mapping(Expression, Expression),
}

#[derive(Debug, Clone)]
pub enum Link {
    One(String, Option<Expression>, LinkDest),
    Multi(Vec<(String, Option<Expression>, Option<String>)>, LinkDest),
}

#[derive(Debug, Clone)]
pub enum Fmt {
    Raw(String),
    Float(Expression, u32), // TODO(tailhook) zero-padding
    Int(Expression),  // TODO(tailhook) zero-padding
    Str(Expression),  // TODO(tailhook) need formatting or padding?
}

#[derive(Debug, Clone)]
pub enum Statement {
    Element {
        name: String,
        classes: Vec<(String, Option<Expression>)>,
        attributes: Vec<(String, Expression)>,
        body: Vec<Statement>,
    },
    Format(Vec<Fmt>),
    Output(Expression),
    Store(String, Expression),
    Let(String, Expression),
    Link(Vec<Link>),
    Condition(Vec<(Expression, Vec<Statement>)>, Option<Vec<Statement>>),
    ForOf(String, Expression, Vec<Statement>),
}


fn param<'a, I: Stream<Item=Token<'a>>>(input: State<I>)
    -> ParseResult<Param, I>
{
    lift(Tok::Ident).and(optional(lift(Tok::Equals)
        .with(lift(Tok::String))))  // TODO(tailhook) more expressions
    .map(|((_, name, _), opt)| Param {
        name: String::from(name),
        default_value: opt.map(ParseToken::unescape)
    })
    .parse_state(input)
}

fn dash_name<'a, I: Stream<Item=Token<'a>>>(input: State<I>)
    -> ParseResult<String, I>
{
    sep_by::<Vec<_>, _, _>(
        lift(Tok::Ident).or(lift(Tok::Number)).map(ParseToken::into_string),
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
            parser(dash_name)
                .skip(lift(Tok::Equals))
                .and(lift(Tok::String)
                    .map(parse_format_string)
                    .map(Expression::Format)
                    .or(parser(expression))),
            lift(Tok::Comma)))
        .skip(lift(Tok::CloseBracket)))
    .parse_state(input)
}

fn element<'a, I: Stream<Item=Token<'a>>>(input: State<I>)
    -> ParseResult<Statement, I>
{
    parser(element_start)
    .and(parser(attributes))
    .and(
        lift(Tok::String)
            .skip(lift(Tok::Newline))
            .map(|x| Some(vec![Statement::Format(parse_format_string(x))]))
        .or(lift(Tok::Newline)
            .with(parser(chunk))))
    .map(|(((name, classes), opt_attributes), opt_body)| Statement::Element {
        name: name,
        classes: classes,
        attributes: opt_attributes.unwrap_or(vec!()),
        body: opt_body.unwrap_or(vec!()),
        })
    .parse_state(input)
}

fn parse_format_string(tok: Token) -> Vec<Fmt> {
    let value = tok.unescape();
    let mut iter = value.char_indices();
    let mut buf = vec![];
    let mut cur = 0;
    loop {
        let mut start = None;
        let mut end = None;
        let mut fmt = None;
        for (idx, ch) in &mut iter {
            if ch == '{' && idx < value.len()-1
                && &value[idx+1..idx+2] != "{"
            {
                start = Some(idx);
                break;
            }
        }
        for (idx, ch) in &mut iter {
            if ch == ':' && fmt.is_none() {
                fmt = Some(idx);
            } else if ch == '}' {
                end = Some(idx);
                break;
            }
        }
        if start.is_none() || end.is_none() {
            if cur < value.len() {
                buf.push(Fmt::Raw(String::from(&value[cur..])));
            }
            break;
        }
        let start = start.unwrap();
        let end = end.unwrap();
        if start > cur {
            buf.push(Fmt::Raw(String::from(&value[cur..start])));
        }
        if end > start {
            let expr = parse_html_expr(&value[start+1..fmt.unwrap_or(end)])
                .unwrap();  // TODO(tailhook) real error reporting
            if let Some(fmt_off) = fmt {
                // TODO(tailhook) formatting
                buf.push(Fmt::Str(expr));
            } else {
                buf.push(Fmt::Str(expr));
            }
        }
        cur = end+1;
    }
    return buf;
}

fn literal<'a, I: Stream<Item=Token<'a>>>(input: State<I>)
    -> ParseResult<Statement, I>
{
    lift(Tok::String).skip(lift(Tok::Newline))
    .map(parse_format_string)
    .map(Statement::Format)
    .parse_state(input)
}

fn call<'a, I: Stream<Item=Token<'a>>>(input: State<I>)
    -> ParseResult<Expression, I>
{
    enum Sub {
        GetAttr(String),
        GetItem(Expression),
        Call(Vec<Expression>),
    }
    parser(atom)
    .and(many::<Vec<_>,_>(
        lift(Tok::Dot).with(lift(Tok::Ident))
            .map(ParseToken::into_string).map(Sub::GetAttr)
        .or(between(lift(Tok::OpenBracket), lift(Tok::CloseBracket),
                    parser(expression)).map(Sub::GetItem))
        .or(between(lift(Tok::OpenParen), lift(Tok::CloseParen),
                    sep_by::<Vec<_>, _, _>(parser(expression),
                                           lift(Tok::Comma))
                    .map(Sub::Call)))))
    .map(|(expr, suffixes)|
        suffixes.into_iter().fold(expr, |expr, sub| match sub {
            Sub::GetAttr(x) => Expression::Attr(Box::new(expr), x),
            Sub::GetItem(x) => Expression::Item(Box::new(expr), Box::new(x)),
            Sub::Call(x) => Expression::Call(Box::new(expr), x),
        }))
    .parse_state(input)
}
fn dict<'a, I: Stream<Item=Token<'a>>>(input: State<I>)
    -> ParseResult<Expression, I>
{
    between(lift(Tok::OpenBrace), lift(Tok::CloseBrace),
        sep_by::<Vec<_>, _, _>(
            lift(Tok::String).map(ParseToken::unescape)
            .or(lift(Tok::Ident).map(ParseToken::into_string))
           .skip(lift(Tok::Colon))
           .and(parser(expression)),
           lift(Tok::Comma)))
    .map(Expression::Dict)
    .parse_state(input)
}
fn list<'a, I: Stream<Item=Token<'a>>>(input: State<I>)
    -> ParseResult<Expression, I>
{
    between(lift(Tok::OpenBracket), lift(Tok::CloseBracket),
        sep_by::<Vec<_>, _, _>(parser(expression), lift(Tok::Comma)))
    .map(Expression::List)
    .parse_state(input)
}
fn atom<'a, I: Stream<Item=Token<'a>>>(input: State<I>)
    -> ParseResult<Expression, I>
{
    lift(Tok::Ident).map(ParseToken::into_string).map(Expression::Name)
    .or(lift(Tok::New).with(parser(expression))
        .map(|x| Expression::New(Box::new(x))))
    .or(lift(Tok::String)
        .map(ParseToken::unescape).map(Expression::Str))
    .or(lift(Tok::Number)
        .map(ParseToken::into_string).map(Expression::Num))
    .or(parser(dict))
    .or(parser(list))
    .or(between(lift(Tok::OpenParen), lift(Tok::CloseParen),
                parser(expression)))
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

fn sum<'a, I: Stream<Item=Token<'a>>>(input: State<I>)
    -> ParseResult<Expression, I>
{
    let factor = lift(Tok::Multiply).map(|_| multiply as ChainFun)
                 .or(lift(Tok::Divide).map(|_| divide as ChainFun));
    let sum = lift(Tok::Plus).map(|_| add as ChainFun)
              .or(lift(Tok::Dash).map(|_| subtract as ChainFun));
    let factor = chainl1(parser(call), factor);
    chainl1(factor, sum)
    .parse_state(input)
}


pub fn comparison<'a, I: Stream<Item=Token<'a>>>(input: State<I>)
    -> ParseResult<Expression, I>
{
    parser(sum).and(many(choice([
        lift(Tok::Eq),
        lift(Tok::NotEq),
        lift(Tok::Less),
        lift(Tok::LessEq),
        lift(Tok::Greater),
        lift(Tok::GreaterEq),
        ]).and(parser(sum))))
    .map(|(expr, tails): (Expression, Vec<(_, Expression)>)| {
        let mut expr = expr;
        let mut prev = expr.clone();
        for ((tok, _, _), value) in tails.into_iter() {
            let comp = match tok {
                Tok::Eq => Comparator::Eq,
                Tok::NotEq => Comparator::NotEq,
                Tok::Less => Comparator::Less,
                Tok::LessEq => Comparator::LessEq,
                Tok::Greater => Comparator::Greater,
                Tok::GreaterEq => Comparator::GreaterEq,
                _ => unreachable!(),
            };
            expr = Expression::Comparison(comp,
                Box::new(prev),
                Box::new(value.clone()));
            prev = value;
        }
        return expr;
    })
    .parse_state(input)
}

pub fn boolean<'a, I: Stream<Item=Token<'a>>>(input: State<I>)
    -> ParseResult<Expression, I>
{
    let not = parser(comparison)
        .or(lift(Tok::Not).with(parser(comparison))
            .map(Box::new).map(Expression::Not));
    let and = chainl1(not, lift(Tok::And)
        .map(|_| |a, b| Expression::And(Box::new(a), Box::new(b))));
    let mut or = chainl1(and, lift(Tok::Or)
        .map(|_| |a, b| Expression::Or(Box::new(a), Box::new(b))));
    or.parse_state(input)
}

pub fn expression<'a, I: Stream<Item=Token<'a>>>(input: State<I>)
    -> ParseResult<Expression, I>
{
    parser(boolean).parse_state(input)
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

fn let_var<'a, I: Stream<Item=Token<'a>>>(input: State<I>)
    -> ParseResult<Statement, I>
{
    lift(Tok::Let)
    .with(lift(Tok::Ident).map(ParseToken::into_string))
    .skip(lift(Tok::Equals))
    .and(parser(expression)).skip(lift(Tok::Newline))
    .map(|(name, value)| Statement::Let(name, value))
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
        .and(optional(
            lift(Tok::Colon).with(
                lift(Tok::Ident).map(ParseToken::into_string))))
        .and(optional(between(lift(Tok::OpenBracket), lift(Tok::CloseBracket),
            parser(expression))))
        .map(|((n, a), x)| (n, x, a)),
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
    .and(optional(between(lift(Tok::OpenBracket), lift(Tok::CloseBracket),
        parser(expression))))
    .skip(lift(Tok::Equals))
    .and(parser(link_dest))
    .map(|((name, filt), dest)| Link::One(name.into_string(), filt, dest))
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

fn iteration<'a, I: Stream<Item=Token<'a>>>(input: State<I>)
    -> ParseResult<Statement, I>
{
    lift(Tok::For)
    .with(lift(Tok::Ident))
    .skip(lift(Tok::Of))
    .and(parser(expression))
    .skip(lift(Tok::Colon))
    .skip(lift(Tok::Newline))
    .and(parser(chunk))
    .map(|((name, array), opt_body)| Statement::ForOf(
        name.into_string(), array,
        opt_body.unwrap_or(vec!())))
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
    .or(parser(let_var))
    .or(parser(link))
    .or(parser(condition))
    .or(parser(iteration))
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

