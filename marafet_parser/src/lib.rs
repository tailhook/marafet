extern crate parser_combinators;
extern crate unicode_segmentation;
extern crate marafet_util as util;

use parser_combinators::primitives::{Stream, State, Parser};
use parser_combinators::combinator::{many, ParserExt};
use parser_combinators::{ParseResult, parser, from_iter};

use self::token::{Token, lift};
use self::token::TokenType::{Css, Html, Eof};
use self::tokenizer::Tokenizer;

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

fn body<'a, I: Stream<Item=Token<'a>>>(input: State<I>)
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

pub fn parse_string(text: &str) -> Result<Ast, String> {
    parser(body)
    .parse(from_iter(Tokenizer::new(text)))
    .map_err(|x| format!("Parse error: {}", x))
    .map(|(ast, _)| ast)
}
