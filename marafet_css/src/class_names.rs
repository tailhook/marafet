use std::collections::HashSet;

use super::super::grammar::{Block};
use super::super::grammar::css::{Rule, Selector};
use super::super::grammar::Block::{Css, Html};


pub fn add_name_to_block(block: Block, name: &str, elements: &HashSet<String>)
    -> Block
{
    match block {
        Css(params, mut rules) => {
            Css(params, rules.into_iter().map(|r| Rule {
                selectors: r.selectors.into_iter().map(|s| {
                    let mut nclasses = vec!(String::from(name));
                    nclasses.extend(s.classes.into_iter());
                    Selector {
                        classes: nclasses,
                        .. s
                    }
                }).collect(),
                .. r
            }).collect())
        }
        Html(name, params, statements) => {
            Html(name, params, statements)
        }
    }
}
