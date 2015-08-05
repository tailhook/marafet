use combine::{parser, Parser};
use combine::combinator::{optional, ParserExt, sep_by, many};
use combine::combinator::{between};

use super::Block;
use super::token::{Token, ParseToken};
use super::token::TokenType as Tok;
use super::token::lift;
use util::join;
use super::{State, Result};

#[derive(Debug, Clone)]
pub struct Selector {
    pub element: Option<String>,
    pub classes: Vec<String>,
    pub state: Option<String>,
    // TODO(tailhook) implement other selectors
}

#[derive(Debug, Clone)]
pub struct Rule {
    pub selectors: Vec<Selector>,
    pub properties: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub default_value: Option<String>,
}

fn param<'a>(input: State<'a>) -> Result<'a, Param>
{
    lift(Tok::CssWord).and(optional(lift(Tok::Equals).with(lift(Tok::String))))
            .map(|(Token(_, name, _), opt)| Param {
                name: String::from(name),
                default_value: opt.map(|x| String::from(x.1)),
            })
    .parse_state(input)
}

#[allow(unconditional_recursion)]
fn property_value<'a>(input: State<'a>) -> Result<'a, String>
{
    // TODO(tailhook) add numbers slashes and other things
    many::<Vec<_>, _>(
        lift(Tok::CssWord).map(ParseToken::into_string)
            .and(optional(
                between(lift(Tok::OpenParen), lift(Tok::CloseParen),
                    parser(property_value))))
        .map(|(word, opt_brackets)| {
            if let Some(expr) = opt_brackets {
                format!("{}({})", word, expr)
            } else {
                word
            }
        }).or(lift(Tok::Comma).map(ParseToken::into_string))
    ).map(|names| join(names.into_iter(), " "))
    .parse_state(input)
}

fn selector<'a>(input: State<'a>) -> Result<'a, Selector>
{
    optional(lift(Tok::CssWord).map(ParseToken::into_string))
        .and(many::<Vec<_>, _>(
            lift(Tok::Dot).with(lift(Tok::CssWord)
                .map(ParseToken::into_string))))
        .and(optional(lift(Tok::Colon)
            .with(lift(Tok::CssWord).map(ParseToken::into_string))))
    .map(|((element, classes), opt_state)| Selector {
        element: element,
        classes: classes,
        state: opt_state,
    })
    .parse_state(input)
}

fn rule<'a>(input: State<'a>) -> Result<'a, Rule>
{
    sep_by::<Vec<_>, _, _>(
            parser(selector),
            lift(Tok::Comma),
        ).skip(lift(Tok::Newline))
    .and(optional(
        lift(Tok::Indent)
        .with(many::<Vec<_>, _>(
            lift(Tok::CssWord).map(ParseToken::into_string)
            .skip(lift(Tok::Colon))
            .and(optional(parser(property_value)))
            .skip(lift(Tok::Newline))
            )
        ).skip(lift(Tok::Dedent))
    ))
    .map(|(selectors, properties)| {
        Rule {
            selectors: selectors,
            properties: properties.unwrap_or(vec!())
                .into_iter()
                .map(|(key, val)| (key, val.unwrap_or(String::new())))
                .collect(),
        }
    })
    .parse_state(input)
}


pub fn block<'a>(input: State<'a>) -> Result<'a, Block>
{
    optional(lift(Tok::OpenParen)
        .with(sep_by::<Vec<_>, _, _>(parser(param), lift(Tok::Comma)))
        .skip(lift(Tok::CloseParen)))
        .skip(lift(Tok::Colon))
        .skip(lift(Tok::Newline))
        .and(optional(
            lift(Tok::Indent)
            .with(many::<Vec<_>, _>(parser(rule)))
            .skip(lift(Tok::Dedent))
        ))
        .map(|(opt_params, opt_rules)| {
            Block::Css(
                opt_params.unwrap_or(vec!()),
                opt_rules.unwrap_or(vec!()),
            )
        })
        .parse_state(input)
}
