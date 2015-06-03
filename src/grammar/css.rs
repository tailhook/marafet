use parser_combinators::primitives::{Stream, State, Parser};
use parser_combinators::{ParseResult, parser};
use parser_combinators::combinator::{optional, ParserExt, sep_by, many, many1};

use super::Block;
use super::token::Token;
use super::token::TokenType as Tok;
use super::token::lift;

#[derive(Debug)]
pub struct Rule {
    selectors: Vec<String>,
    properties: Vec<(String, String)>,
}

#[derive(Debug)]
pub struct Param {
    name: String,
    default_value: Option<String>,
}

fn param<'a, I: Stream<Item=Token<'a>>>(input: State<I>)
    -> ParseResult<Param, I>
{
    lift(Tok::Ident).and(optional(lift(Tok::Equals).with(lift(Tok::String))))
            .map(|((_, name, _), opt)| Param {
                name: String::from(name),
                default_value: opt.map(|x| String::from(x.1)),
            })
    .parse_state(input)
}

fn join<S1:AsRef<str>, S2:AsRef<str>, I:Iterator<Item=S1>>(mut iter: I, sep: S2)
    -> String
{
    let mut buf = String::new();
    match iter.next() {
        Some(x) => buf.push_str(x.as_ref()),
        None => {}
    }
    for i in iter {
        buf.push_str(sep.as_ref());
        buf.push_str(i.as_ref());
    }
    return buf;
}

fn dash_name<'a, I: Stream<Item=Token<'a>>>(input: State<I>)
    -> ParseResult<String, I>
{
    sep_by::<Vec<_>, _, _>(
        lift(Tok::Ident),
        lift(Tok::Dash),
    ).map(|names| join(names.into_iter().map(|(_, val, _)| val), "-"))
    .parse_state(input)
}

fn property_value<'a, I: Stream<Item=Token<'a>>>(input: State<I>)
    -> ParseResult<String, I>
{
    // TODO(tailhook) add numbers slashes and other things
    many::<Vec<_>, _>(
        lift(Tok::Ident)
    ).map(|names| join(names.into_iter().map(|(_, val, _)| val), " "))
    .parse_state(input)
}

fn rule<'a, I: Stream<Item=Token<'a>>>(input: State<I>)
    -> ParseResult<Rule, I>
{
    sep_by::<Vec<_>, _, _>(
            many1::<Vec<_>, _>(lift(Tok::Dot).and(parser(dash_name))),
            lift(Tok::Comma),
        ).skip(lift(Tok::Newline))
    .and(optional(
        lift(Tok::Indent)
        .with(many::<Vec<_>, _>(
            parser(dash_name)
            .skip(lift(Tok::Colon))
            .and(optional(parser(property_value)))
            .skip(lift(Tok::Newline))
            )
        ).skip(lift(Tok::Dedent))
    ))
    .map(|(selectors, properties)| {
        Rule {
            selectors: selectors.into_iter()
                .map(|vec| join(
                    vec.into_iter()
                    .map(|((_, marker, _), name)|
                         String::from(marker)+name.as_ref()),
                    ""))
                .collect(),
            properties: properties.unwrap_or(vec!())
                .into_iter()
                .map(|(key, val)| (key, val.unwrap_or(String::new())))
                //.map(|key| (key.clone(), key))
                .collect(),
        }
    })
    .parse_state(input)
}


pub fn block<'a, I: Stream<Item=Token<'a>>>(input: State<I>)
    -> ParseResult<Block, I>
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
