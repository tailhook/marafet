extern crate combine;
extern crate unicode_segmentation;
extern crate marafet_util as util;

use combine::combinator::{many, ParserExt};
use combine::{Parser, ParseResult, parser, optional, sep_end_by};

use self::token::{ParseToken, lift};
use self::token::TokenType::{Css, Html, Eof};
use self::token::TokenType::{Import, From, Comma, Newline};
use self::token::TokenType::{OpenBrace, CloseBrace, Ident, As};
use self::token::TokenType::String as StrTok;
use self::tokenizer::Tokenizer;

mod token;
mod tokenizer;
pub mod css;
pub mod html;

// I'm not sure why they should be public but compiler insists
pub type Stream<'a> = Tokenizer<'a>;
pub type State<'a> = combine::State<Stream<'a>>;
pub type Result<'a, T> = combine::primitives::ParseResult<T, Stream<'a>>;

#[derive(Debug, Clone)]
pub enum Block {
    Css(Vec<css::Param>, Vec<css::Rule>),
    Html {
        name: String,
        params: Vec<html::Param>,
        events: Vec<String>,
        statements: Vec<html::Statement>,
    },
    ImportModule(String, String),
    ImportVars(Vec<(String, Option<String>)>, String),
}

#[derive(Debug, Clone)]
pub struct Ast {
    pub blocks: Vec<Block>,
}

fn import_braces<'x>(input: State<'x>)
    -> Result<'x, Vec<(String, Option<String>)>>
{
    lift(OpenBrace).with(sep_end_by::<Vec<_>, _, _>(
            lift(Ident).map(ParseToken::into_string)
            .and(optional(lift(As).with(lift(Ident)
                                        .map(ParseToken::into_string)))),
            lift(Comma)))
        .skip(lift(CloseBrace))
    .parse_state(input)
}

fn import<'x>(input: State<'x>) -> Result<'x, Block>
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

fn body<'x>(input: State<'x>) -> Result<'x, Ast>
{
    let css = lift(Css).with(parser(css::block));
    let html = lift(Html).with(parser(html::block));
    let import = lift(Import).with(parser(import));
    let block = css.or(html).or(import);
    let blocks = many::<Vec<_>, _>(block).skip(lift(Eof));
    return blocks.map(|blocks| Ast {
        blocks: blocks,
    }).parse_state(input);
}

pub fn parse_string(text: &str) -> ::std::result::Result<Ast, String> {
    parser(body)
    .parse(Tokenizer::new(text))
    .map_err(|x| format!("Parse error: {:?}", x))
    .map(|(ast, _)| ast)
}

pub fn parse_html_expr(text: &str)
    -> ::std::result::Result<html::Expression, String>
{
    parser(html::expression)
    .parse(Tokenizer::new(text))
    .map_err(|x| format!("Parse error: {:?}", x))
    .map(|(ast, _)| ast)
}

