use parser_combinators::primitives::{Stream, State, Parser};
use parser_combinators::{ParseResult, parser};
use parser_combinators::combinator::{optional, ParserExt, sep_by, many, many1};

use super::Block;
use super::token::Token;
use super::token::TokenType as Tok;
use super::token::lift;


#[derive(Debug)]
pub struct Param {
    name: String,
    default_value: Option<String>,
}

#[derive(Debug)]
pub enum Item {
    Element(String),
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

pub fn block<'a, I: Stream<Item=Token<'a>>>(input: State<I>)
    -> ParseResult<Block, I>
{
    lift(Tok::Ident)
    .and(optional(lift(Tok::OpenParen)
        .with(sep_by::<Vec<_>, _, _>(parser(param), lift(Tok::Comma)))
        .skip(lift(Tok::CloseParen))))
    .skip(lift(Tok::Colon))
    .skip(lift(Tok::Newline))
    .map(|((_, name, _), opt_params)| {
        Block::Html(
            name.to_string(),
            opt_params.unwrap_or(vec!()),
            vec!(),
            )
    })
    .parse_state(input)
}

