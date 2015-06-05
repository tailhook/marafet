use std::io::{Write, Result};
use std::string::ToString;
use std::collections::HashMap;

use super::grammar::Block;
use super::grammar::css::{Rule, Selector};
use super::util::join;


impl ToString for Selector {
    fn to_string(&self) -> String {
        let mut buf = Vec::new();
        if let Some(ref element) = self.element {
            write!(&mut buf, "{}", element).unwrap();
        }
        for cls in self.classes.iter() {
            write!(&mut buf, ".{}", cls).unwrap();
        }
        return String::from_utf8(buf).unwrap();
    }
}



fn output_rule<W: Write>(buf: &mut W, vars: &HashMap<&String, &String>,
    rule: &Rule) -> Result<()>
{
    let selectors = join(rule.selectors.iter().map(|x| x.to_string()), ", ");
    try!(write!(buf, "{} {{\n", selectors));
    for &(ref k, ref v) in rule.properties.iter() {
        try!(write!(buf, "    {}: {};\n", k, v));
    }
    try!(write!(buf, "}}\n\n"));
    Ok(())
}

pub fn generate<W>(buf: &mut W, blk: &Block, values: &HashMap<String, String>)
    -> Result<()>
    where W: Write
{
    let mut vars = HashMap::new();
    if let &Block::Css(ref params, ref rules) = blk {
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
    } else {
        panic!("Wrong block type supplied to css::generate");
    }
    Ok(())
}
