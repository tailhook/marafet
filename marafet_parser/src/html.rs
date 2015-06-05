use parser_combinators::primitives::{Stream, State, Parser};
use parser_combinators::{ParseResult, parser};
use parser_combinators::combinator::{optional, ParserExt, sep_by, many, many1};

use super::Block;
use super::token::{Token, ParseToken};
use super::token::TokenType as Tok;
use super::token::lift;


#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub default_value: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Statement {
    Element {
        name: String,
        classes: Vec<String>,
        body: Vec<Statement>,
    },
    Text(String),
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

fn element<'a, I: Stream<Item=Token<'a>>>(input: State<I>)
    -> ParseResult<Statement, I>
{
    let element_head = lift(Tok::Ident)
        .map(ParseToken::into_string)
        .and(many::<Vec<_>, _>(lift(Tok::Dot)
            .with(lift(Tok::Ident).map(ParseToken::into_string))));
    let div_head = many1::<Vec<_>, _>(lift(Tok::Dot)
            .with(lift(Tok::Ident).map(ParseToken::into_string)))
        .map(|items| (String::from("div"), items));
    let head = element_head.or(div_head).skip(lift(Tok::Newline));

    head
    .and(optional(lift(Tok::Indent)
        .with(many1(parser(statement)))
        .skip(lift(Tok::Dedent))))
    .map(|((name, classes), opt_body)| Statement::Element {
        name: name,
        classes: classes,
        body: opt_body.unwrap_or(vec!()),
        })
    .parse_state(input)
}

fn statement<'a, I: Stream<Item=Token<'a>>>(input: State<I>)
    -> ParseResult<Statement, I>
{

    let literal = lift(Tok::String).skip(lift(Tok::Newline))
        .map(|tok| Statement::Text(tok.unescape()));

    parser(element)
    .or(literal)
    .parse_state(input)
}

pub fn block<'a, I: Stream<Item=Token<'a>>>(input: State<I>)
    -> ParseResult<Block, I>
{
    lift(Tok::Ident).map(ParseToken::into_string)
    .and(optional(lift(Tok::OpenParen)
        .with(sep_by::<Vec<_>, _, _>(parser(param), lift(Tok::Comma)))
        .skip(lift(Tok::CloseParen))))
    .skip(lift(Tok::Colon))
    .skip(lift(Tok::Newline))
    .and(optional(
        lift(Tok::Indent)
        .with(many1(parser(statement)))
        .skip(lift(Tok::Dedent))))
    .map(|((name, opt_params), opt_stmtlist)| {
        Block::Html(
            name,
            opt_params.unwrap_or(vec!()),
            opt_stmtlist.unwrap_or(vec!()),
            )
    })
    .parse_state(input)
}

