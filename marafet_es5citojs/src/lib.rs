extern crate marafet_parser as parser;
extern crate marafet_util as util;

use std::io::{Write, Result};
use std::collections::HashSet;

use parser::{Ast, Block};
use parser::html;

use emit::Emit;

mod bare_elements;
mod ast;
mod emit;
mod code;
mod amd;


pub struct Settings<'a> {
    pub block_name: &'a str,
    pub use_amd: bool,
    pub amd_name: &'a str,
}

struct Generator<'a, W: 'a> {
    block_name: &'a str,
    indent: u32,
    bare_element_names: HashSet<String>,
    buf: &'a mut W,
    use_amd: bool,
    amd_name: &'a str,
}

pub fn generate<W>(buf: &mut W, ast: &Ast, settings: &Settings) -> Result<()>
    where W: Write
{
    let mut gen = Generator {
        block_name: settings.block_name,
        use_amd: settings.use_amd,
        amd_name: &settings.amd_name,
        indent: 4,  // TODO(tailhook) allow customize
        bare_element_names: bare_elements::visitor(ast),
        buf: buf,
    };
    let mut code = gen.code(ast);
    if gen.use_amd {
        code = gen.wrap_amd(code, ast);
    }
    // TODO(tailhook) optimize
    gen.emit(&code);
    Ok(())
}
