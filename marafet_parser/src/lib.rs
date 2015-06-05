extern crate parser_combinators;
extern crate unicode_segmentation;
extern crate marafet_util as util;

use parser_combinators::primitives::{Stream, State, Parser};
use parser_combinators::combinator::{many, ParserExt};
use parser_combinators::{ParseResult, parser, from_iter, optional, sep_by};

use self::token::{Token, ParseToken, lift};
use self::token::TokenType::{Css, Html, Eof};
use self::token::TokenType::{Import, From, Comma, Newline};
use self::token::TokenType::{OpenBrace, CloseBrace, Ident, As};
use self::token::TokenType::String as StrTok;
use self::tokenizer::Tokenizer;

mod token;
mod tokenizer;
pub mod css;
pub mod html;

#[derive(Debug, Clone)]
pub enum Block {
    Css(Vec<css::Param>, Vec<css::Rule>),
    Html(String, Vec<html::Param>, Vec<html::Statement>),
    ImportModule(String, String),
    ImportVars(Vec<(String, Option<String>)>, String),
}

#[derive(Debug, Clone)]
pub struct Ast {
    pub blocks: Vec<Block>,
}

fn import_braces<'a, I: Stream<Item=Token<'a>>>(input: State<I>)
    -> ParseResult<Vec<(String, Option<String>)>, I>
{
    lift(OpenBrace).with(sep_by::<Vec<_>, _, _>(
            lift(Ident).map(ParseToken::into_string)
            .and(optional(lift(As).with(lift(Ident)
                                        .map(ParseToken::into_string)))),
            lift(Comma)))
        .skip(lift(CloseBrace))
    .parse_state(input)
}

fn import<'a, I: Stream<Item=Token<'a>>>(input: State<I>)
    -> ParseResult<Block, I>
{
    let vars = parser(import_braces)
        .skip(lift(From)).and(lift(StrTok).map(ParseToken::unescape))
        .skip(lift(Newline))
        .map(|(names, module)| Block::ImportVars(names, module));
    let module = lift(Ident).map(ParseToken::into_string)
        .skip(lift(From)).and(lift(StrTok).map(ParseToken::unescape))
        .skip(lift(Newline))
        .map(|(name, module)| Block::ImportModule(name, module));
    vars.or(module)
    .parse_state(input)
}

fn body<'a, I: Stream<Item=Token<'a>>>(input: State<I>)
    -> ParseResult<Ast, I>
{
    let css = lift(Css).with(parser(css::block));
    let html = lift(Html).with(parser(html::block));
    let import = lift(Import).with(parser(import));
    let block = css.or(html).or(import);
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
