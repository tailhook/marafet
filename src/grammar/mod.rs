use parser_combinators::primitives::{Stream, State, Parser};
use parser_combinators::combinator::{Try, Or, many, Many, ParserExt, Map};
use parser_combinators::{ParseResult, parser};

use self::token::lift;
use self::token::{Token, TokenParser};
use self::token::TokenType::{Css, Html};
pub use self::tokenizer::Tokenizer;

mod token;
mod tokenizer;
mod css;
mod html;

#[derive(Debug)]
pub enum Block {
    Css(Vec<()>),
    Html(String, Vec<()>),
}


pub fn body<'a>(input: State<Tokenizer<'a>>)
    -> ParseResult<Vec<Block>, Tokenizer<'a>>
{
    let css = lift(Css).with(parser(css::block));
    let html = lift(Html).with(parser(html::block));
    let block = css.or(html);
    let mut blocks = many::<Vec<_>, _>(block);
    return blocks.parse_state(input);
}
