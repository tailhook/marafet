extern crate marafet_parser as parser;
extern crate marafet_util as util;

use std::io::{Write, Result};
use std::collections::HashSet;

use parser::{Ast};

use emit::Emit;

mod bare_elements;
mod ast;
mod emit;
mod code;
mod css;
mod amd;


pub struct Settings<'a> {
    pub block_name: &'a str,
    pub use_amd: bool,
    pub amd_name: &'a str,
    pub css_text: Option<&'a str>,
}

struct Generator<'a, W: 'a> {
    block_name: &'a str,
    indent: u32,
    bare_element_names: HashSet<String>,
    buf: &'a mut W,
    use_amd: bool,
    amd_name: &'a str,
    css_text: Option<&'a str>,
}

pub fn generate<W>(buf: &mut W, ast: &Ast, settings: &Settings) -> Result<()>
    where W: Write
{
    let mut gen = Generator {
        block_name: settings.block_name,
        use_amd: settings.use_amd,
        amd_name: settings.amd_name,
        css_text: settings.css_text,
        indent: 4,  // TODO(tailhook) allow customize
        bare_element_names: bare_elements::visitor(ast),
        buf: buf,
    };
    let mut code = gen.code(ast);
    if let Some(css) = gen.css_text {
        code = gen.add_css(code, css);
    }
    if gen.use_amd {
        code = gen.wrap_amd(code, ast);
    }
    // TODO(tailhook) optimize
    try!(gen.emit(&code));
    Ok(())
}
