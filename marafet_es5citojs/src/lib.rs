extern crate marafet_parser as parser;
extern crate marafet_util as util;

use std::io::{Write, Result};
use std::collections::HashSet;

use parser::{Ast, Block};

use ast::{Code, Statement, Param, Expression};
use emit::Emit;

mod bare_elements;
mod ast;
mod emit;


pub struct Settings<'a> {
    pub block_name: &'a String,
}

struct Generator<'a, W: 'a> {
    block_name: &'a String,
    indent: u32,
    element_names: HashSet<String>,
    buf: &'a mut W,
}


impl<'a, W:Write+'a> Generator<'a, W> {

    fn code(&self, ast: &Ast) -> Code {
        let mut stmt = vec!();
        for block in ast.blocks.iter() {
            if let &Block::Html(ref name, ref args, ref statements) = block {
                stmt.push(Statement::Function(name.clone(),
                    args.iter().map(|p| Param {
                        name: p.name.clone(),
                        default_value: p.default_value.as_ref().map(
                            |v| Expression::Str(v.clone())),
                    }).collect(),
                    vec![
                        Statement::Return(
                            Expression::Str(String::from("hello"))),
                    ]
                ));
            }
        }
        return Code {
            statements: stmt,
        }
    }

}


pub fn generate<W>(buf: &mut W, ast: &Ast, settings: &Settings) -> Result<()>
    where W: Write
{
    let mut gen = Generator {
        block_name: settings.block_name,
        indent: 4,  // TODO(tailhook) allow customize
        element_names: bare_elements::visitor(ast),
        buf: buf,
    };
    let code = gen.code(ast);
    // TODO(tailhook) optimize
    gen.emit(&code);
    Ok(())
}
