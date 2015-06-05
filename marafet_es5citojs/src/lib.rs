extern crate marafet_parser as parser;

use std::io::{Write, Result};

use parser::{Ast, Block};


pub fn generate<W>(buf: &mut W, ast: &Ast) -> Result<()>
    where W: Write
{
    for block in ast.blocks.iter() {
        if let &Block::Html(ref name, ref args, ref statements) = block {
            write!(buf, "function {}(", name);
            // TODO(tailhook) args
            write!(buf, ") {{\n");
            // TODO(tailhook) variables?
            write!(buf, "    return ");
            write!(buf, ";\n");
            write!(buf, "}}\n");
        }
    }
    Ok(())
}
