use std::marker::PhantomData;

use combine::primitives::{Parser, SourcePosition, Positioner};
use combine::primitives::{Info, ParseError, Consumed, Error};
use combine::primitives::Stream as StreamTrait;

use super::{Stream, State, Result};


#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum TokenType {
    Css,
    Html,
    Import,
    From,
    If,
    Elif,
    For,
    In,
    Of,
    Key,
    As,
    Else,
    Events,
    Link,
    Store,
    Let,
    New,
    And,
    Or,
    Not,

    Comma,          // ,
    Equals,         // =
    Eq,             // ==
    NotEq,          // !=
    Greater,        // >
    Less,           // <
    GreaterEq,      // >=
    LessEq,         // <=
    Colon,          // :
    Dot,            // .
    Dash,           // -
    Plus,           // +
    Multiply,       // *
    Divide,         // /
    Question,       // ?
    ArrowRight,     // ->
    OpenParen,      // (
    OpenBracket,    // [
    OpenBrace,      // {
    CloseParen,     // )
    CloseBracket,   // ]
    CloseBrace,     // }
    Ident,
    CssWord,
    Number,
    String,
    Newline,
    Indent,
    Dedent,
    Eof,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Token<'a>(pub TokenType, pub &'a str, pub SourcePosition);

impl<'a> Positioner for Token<'a> {
    type Position = SourcePosition;
    fn start() -> SourcePosition {
        SourcePosition {
            line: 1,
            column: 1,
        }
    }
    fn update(&self, pos: &mut SourcePosition) {
        *pos = self.2;
    }
}

pub trait ParseToken {
    fn into_string(self) -> String;
    fn unescape(self) -> String;
}

impl<'a> ParseToken for Token<'a> {
    fn into_string(self) -> String {
        return String::from(self.1);
    }
    fn unescape(self) -> String {
        let slice = self.1;
        let quote = slice.chars().next().unwrap();
        if quote != '"' && quote != '\'' {
            panic!("Only string tokens can be unescaped");
        }
        let mut result = String::new();
        let mut iter = slice[1..].chars();
        loop {
            let ch = if let Some(ch) = iter.next() { ch } else { break; };
            match ch {
                '\\' => {
                    if let Some(ch) = iter.next() {
                        match ch {
                            'x' => unimplemented!(),
                            '\n' => unimplemented!(),
                            'r' => result.push('\r'),
                            'n' => result.push('\n'),
                            't' => result.push('\t'),
                            _ => result.push(ch),
                        }
                    } else {
                        panic!("Slash at end of line");
                    }
                }
                '"'|'\'' => {
                    if quote == ch {
                        break;
                    } else {
                        result.push(ch);
                    }
                }
                _ => {
                    result.push(ch);
                }
            }
        }
        assert!(iter.next().is_none());
        return result;
    }
}

pub struct TokenParser<I> {
    token: TokenType,
    ph: PhantomData<I>,
}

impl TokenType {
    fn info(&self) -> Info<Token<'static>, Token<'static>> {
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
            TokenType::CssWord => Info::Borrowed("css word or identifier"),
            TokenType::Number => Info::Borrowed("number"),
            TokenType::String => Info::Borrowed("quoted string"),
            TokenType::Newline => Info::Borrowed("new line"),
            TokenType::Indent => Info::Borrowed("indentation"),
            TokenType::Dedent => Info::Borrowed("unindent"),
            TokenType::Comma => Info::Borrowed("comma"),
            TokenType::Equals => Info::Borrowed("equals (assignment)"),
            TokenType::Eq => Info::Borrowed("double equals"),
            TokenType::NotEq => Info::Borrowed("not equals"),
            TokenType::Greater => Info::Borrowed("greater"),
            TokenType::Less => Info::Borrowed("less"),
            TokenType::GreaterEq => Info::Borrowed("greater or equal"),
            TokenType::LessEq => Info::Borrowed("less or equal"),
            TokenType::Colon => Info::Borrowed("colon"),
            TokenType::Dot => Info::Borrowed("dot"),
            TokenType::Dash => Info::Borrowed("dash (i.e. minus)"),
            TokenType::Plus => Info::Borrowed("plus"),
            TokenType::Multiply => Info::Borrowed("multiply"),
            TokenType::Divide => Info::Borrowed("division"),
            TokenType::Question => Info::Borrowed("question mark"),
            TokenType::ArrowRight => Info::Borrowed("arrow right"),
            TokenType::Eof => Info::Borrowed("end of file"),
            TokenType::Import => Info::Borrowed("import"),
            TokenType::From => Info::Borrowed("from"),
            TokenType::If => Info::Borrowed("if"),
            TokenType::Elif => Info::Borrowed("elif"),
            TokenType::For => Info::Borrowed("for"),
            TokenType::In => Info::Borrowed("in"),
            TokenType::Of => Info::Borrowed("of"),
            TokenType::Key => Info::Borrowed("key"),
            TokenType::As => Info::Borrowed("as"),
            TokenType::Else => Info::Borrowed("else"),
            TokenType::Events => Info::Borrowed("events"),
            TokenType::Link => Info::Borrowed("link"),
            TokenType::Store => Info::Borrowed("store"),
            TokenType::Let => Info::Borrowed("let"),
            TokenType::New => Info::Borrowed("new"),
            TokenType::Not => Info::Borrowed("not"),
            TokenType::And => Info::Borrowed("and"),
            TokenType::Or => Info::Borrowed("or"),
        }
    }
}

impl<'a> Parser for TokenParser<Stream<'a>> {
    type Input = Stream<'a>;
    type Output = Token<'a>;
    fn parse_lazy<'x>(&mut self, input: State<'x>) -> Result<'x, Token<'x>> {
        match input.input.clone().uncons() {
            Ok((c, s)) => {
                if c.0 == self.token { input.update(c, s) }
                else {
                    Err(Consumed::Empty(ParseError::empty(input.position)))
                }
            }
            Err(err) => Err(Consumed::Empty(ParseError::new(input.position, err)))
        }

    }
    fn add_error(&mut self, error: &mut ParseError<Stream<'a>>) {
        error.errors.push(Error::Expected(self.token.info()));
    }
}

pub fn lift<'a>(tok: TokenType) -> TokenParser<Stream<'a>> {
    return TokenParser { token: tok, ph: PhantomData };
}
