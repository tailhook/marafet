extern crate marafet_parser as parser;
extern crate marafet_util as util;

use std::io::{Write, Result};
use std::collections::HashMap;

use parser::{Ast, Block};
use parser::css::{Rule, Selector};
use util::join;


fn selector_to_string(sel: &Selector) -> String {
    let mut buf = Vec::new();
    if let Some(ref element) = sel.element {
        write!(&mut buf, "{}", element).unwrap();
    }
    for cls in sel.classes.iter() {
        write!(&mut buf, ".{}", cls).unwrap();
    }
    return String::from_utf8(buf).unwrap();
}

fn output_rule<W: Write>(buf: &mut W, vars: &HashMap<&String, &String>,
    rule: &Rule) -> Result<()>
{
    let selectors = join(rule.selectors.iter().map(selector_to_string), ", ");
    try!(write!(buf, "{} {{\n", selectors));
    for &(ref k, ref v) in rule.properties.iter() {
        try!(write!(buf, "    {}: {};\n", k, v));
    }
    try!(write!(buf, "}}\n\n"));
    Ok(())
}

pub fn generate<W>(buf: &mut W, ast: &Ast, values: &HashMap<String, String>)
    -> Result<()>
    where W: Write
{
    let mut vars = HashMap::new();
    for block in ast.blocks.iter() {
        if let &Block::Css(ref params, ref rules) = block {
            for param in params.iter() {
                if let Some(ref val) = param.default_value {
                    vars.insert(&param.name, val);
                }
            }
            for (key, val) in values.iter() {
                vars.insert(key, val);
            }
            for rule in rules.iter() {
                try!(output_rule(buf, &vars, rule));
            }
        }
    }
    Ok(())
}
