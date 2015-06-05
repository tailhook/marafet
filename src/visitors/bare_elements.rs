use std::collections::HashSet;

use super::super::grammar::Ast;
use super::super::grammar::Block;


pub fn visitor(ast: &Ast) -> HashSet<String> {
    let mut res = HashSet::new();
    for block in ast.blocks.iter() {
        if let &Block::Css(ref params, ref rules) = block {
            for rule in rules.iter() {
                for sel in rule.selectors.iter() {
                    if sel.classes.len() == 0 && sel.element.is_some() {
                        res.insert(sel.element.as_ref().unwrap().clone());
                    }
                }
            }
        }
    }
    return res;
}
