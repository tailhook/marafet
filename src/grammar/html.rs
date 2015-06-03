use parser_combinators::primitives::{Stream, State, Parser};
use parser_combinators::{ParseResult, parser};
use parser_combinators::combinator::{ParserExt};

use super::Block;
use super::token::Token;
use super::token::TokenType as Tok;
use super::token::lift;



pub fn block<'a, I: Stream<Item=Token<'a>>>(input: State<I>)
    -> ParseResult<Block, I>
{
    lift(Tok::Ident)
        .map(|(_, name, _): (_, &str, _)| {
            Block::Html(name.to_string(), vec!())
        }).parse_state(input)
}

