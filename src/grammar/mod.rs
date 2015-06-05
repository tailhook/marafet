use parser_combinators::primitives::{Stream, State, Parser};
use parser_combinators::combinator::{many, ParserExt};
use parser_combinators::{ParseResult, parser};

use self::token::{Token, lift};
use self::token::TokenType::{Css, Html, Eof};
pub use self::tokenizer::Tokenizer;

mod token;
mod tokenizer;
pub mod css;
pub mod html;

#[derive(Debug)]
pub enum Block {
    Css(Vec<css::Param>, Vec<css::Rule>),
    Html(String, Vec<html::Param>, Vec<html::Statement>),
}

#[derive(Debug)]
pub struct Ast {
    pub blocks: Vec<Block>,
}

pub fn body<'a, I: Stream<Item=Token<'a>>>(input: State<I>)
    -> ParseResult<Ast, I>
{
    let css = lift(Css).with(parser(css::block));
    let html = lift(Html).with(parser(html::block));
    let block = css.or(html);
    let mut blocks = many::<Vec<_>, _>(block).skip(lift(Eof));
    return blocks.map(|blocks| Ast {
        blocks: blocks,
    }).parse_state(input);
}
