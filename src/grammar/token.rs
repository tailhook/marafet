use std::marker::PhantomData;

use parser_combinators::primitives::{Parser, Stream, State, SourcePosition};
use parser_combinators::primitives::{Info};
use parser_combinators::primitives::{ParseResult, ParseError, Error, Consumed};


#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum TokenType {
    Css,
    Html,
    Comma,          // ,
    Equals,         // =
    Colon,          // :
    Dot,            // .
    Dash,           // -
    OpenParen,      // (
    OpenBracket,    // [
    OpenBrace,      // {
    CloseParen,     // )
    CloseBracket,   // ]
    CloseBrace,     // }
    Ident,
    String,
    Newline,
    Indent,
    Dedent,
    Eof,
}

pub type Token<'a> = (TokenType, &'a str, SourcePosition);

pub struct TokenParser<I> {
    token: TokenType,
    ph: PhantomData<I>,
}

impl TokenType {
    fn info(&self) -> Info {
        match *self {
            TokenType::Css => Info::Borrowed("css NAME[(PARAMS..)]"),
            TokenType::Html => Info::Borrowed("html NAME[(PARAMS)]"),
            TokenType::OpenParen => Info::Borrowed("("),
            TokenType::OpenBracket => Info::Borrowed("["),
            TokenType::OpenBrace => Info::Borrowed("{"),
            TokenType::CloseParen => Info::Borrowed(")"),
            TokenType::CloseBracket => Info::Borrowed("]"),
            TokenType::CloseBrace => Info::Borrowed("}"),
            TokenType::Ident => Info::Borrowed("identifier"),
            TokenType::String => Info::Borrowed("quoted string"),
            TokenType::Newline => Info::Borrowed("new line"),
            TokenType::Indent => Info::Borrowed("indentation"),
            TokenType::Dedent => Info::Borrowed("unindent"),
            TokenType::Comma => Info::Borrowed("comma"),
            TokenType::Equals => Info::Borrowed("equals"),
            TokenType::Colon => Info::Borrowed("colon"),
            TokenType::Dot => Info::Borrowed("dot"),
            TokenType::Dash => Info::Borrowed("dash (i.e. minus)"),
            TokenType::Eof => Info::Borrowed("end of file"),
        }
    }
}

impl<'a, I: Stream<Item=Token<'a>>> Parser for TokenParser<I> {
    type Input = I;
    type Output = Token<'a>;

    fn parse_state(&mut self, input: State<I>) -> ParseResult<Token<'a>, I> {
        match input.clone().uncons(|pos, &(_, _, p)| { *pos = p; }) {
            Ok((tok, s)) => {
                let (typ, val, pos) = tok;
                if self.token == typ { Ok((tok, s)) }
                else {
                    let mut errors = vec![
                        Error::Expected(self.token.info())];
                    if val.len() > 0 {
                        errors.insert(0,
                            Error::Unexpected(val.chars().next().unwrap()));
                    }
                    Err(Consumed::Empty(
                        ParseError::from_errors(pos, errors)))
                }
            }
            Err(err) => Err(err)
        }
    }
}

pub fn lift<'a, I: Stream<Item=Token<'a>>>(tok: TokenType) -> TokenParser<I> {
    return TokenParser { token: tok, ph: PhantomData };
}
