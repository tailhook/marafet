use parser_combinators::primitives::{Stream, State, Parser};
use parser_combinators::{ParseResult, parser};
use parser_combinators::combinator::{optional, ParserExt, sep_by, many};

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
pub struct VarDef {
    name: String,
    default_value: Option<String>,
}

fn var_def<'a, I: Stream<Item=Token<'a>>>(input: State<I>)
    -> ParseResult<VarDef, I>
{
    lift(Tok::Ident).and(optional(lift(Tok::Equals).with(lift(Tok::String))))
            .map(|((_, name, _), opt)| VarDef {
                name: String::from(name),
                default_value: opt.map(|x| String::from(x.1)),
            })
    .parse_state(input)
}

fn rule<'a, I: Stream<Item=Token<'a>>>(input: State<I>)
    -> ParseResult<Rule, I>
{
    lift(Tok::Dot).and(lift(Tok::Ident)).and(lift(Tok::Newline))
            .map(|_| Rule {
                selectors: vec!(),
                properties: vec!(),
            })
    .parse_state(input)
}


pub fn block<'a, I: Stream<Item=Token<'a>>>(input: State<I>)
    -> ParseResult<Block, I>
{
    optional(lift(Tok::OpenParen)
        .with(sep_by::<Vec<_>, _, _>(parser(var_def), lift(Tok::Comma)))
        .skip(lift(Tok::CloseParen)))
        .skip(lift(Tok::Colon))
        .skip(lift(Tok::Newline))
        .and(optional(
            lift(Tok::Indent)
            .with(many::<Vec<_>, _>(parser(rule)))
            .skip(lift(Tok::Dedent))
        ))
        .map(|(opt_params, opt_rules)| {
            println!("PARAMS {:?} RULES {:?}", opt_params, opt_rules);
            Block::Css(vec!())
        })
        .parse_state(input)
}
