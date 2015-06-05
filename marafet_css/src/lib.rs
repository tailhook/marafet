extern crate marafet_parser as parser;
extern crate marafet_util as util;

use std::io::{Write, Result};
use std::collections::HashMap;

use parser::{Ast, Block};
use parser::css::{Rule, Selector};
use util::join;


pub struct Settings<'a> {
    pub block_name: &'a String,
    pub vars: &'a HashMap<String, String>,
}

struct Generator<'a, W: 'a> {
    block_name: &'a String,
    vars: &'a HashMap<&'a String, &'a String>,
    buf: &'a mut W,
}



impl<'a, W:Write+'a> Generator<'a, W> {

    fn selector_to_string(&self, sel: &Selector) -> String {
        let mut buf = Vec::new();
        if let Some(ref element) = sel.element {
            write!(&mut buf, "{}", element).unwrap();
        }
        write!(&mut buf, ".{}", self.block_name).unwrap();
        for cls in sel.classes.iter() {
            write!(&mut buf, ".{}", cls).unwrap();
        }
        return String::from_utf8(buf).unwrap();
    }

    fn output_rule(&mut self, rule: &Rule) -> Result<()>
    {
        let selectors = join(rule.selectors.iter()
                             .map(|x| self.selector_to_string(x)), ", ");
        try!(write!(self.buf, "{} {{\n", selectors));
        for &(ref k, ref v) in rule.properties.iter() {
            try!(write!(self.buf, "    {}: {};\n", k, v));
        }
        try!(write!(self.buf, "}}\n\n"));
        Ok(())
    }
}

pub fn generate<W>(buf: &mut W, ast: &Ast, settings: &Settings) -> Result<()>
    where W: Write
{
    for block in ast.blocks.iter() {
        if let &Block::Css(ref params, ref rules) = block {
            let mut vars = HashMap::new();
            for param in params.iter() {
                if let Some(ref val) = param.default_value {
                    vars.insert(&param.name, val);
                }
            }
            for (key, val) in settings.vars.iter() {
                vars.insert(key, val);
            }
            for rule in rules.iter() {
                let mut gen = Generator {
                    block_name: settings.block_name,
                    vars: &vars,
                    buf: buf,
                };
                try!(gen.output_rule(rule));
            }
        }
    }
    Ok(())
}
