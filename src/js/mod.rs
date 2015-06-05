use std::io::{Write, Result};

use super::grammar::Block;


pub fn generate<W>(buf: &mut W, blk: &Block) -> Result<()>
    where W: Write
{
    if let &Block::Html(ref name, ref args, ref statements) = blk {
        write!(buf, "function {}(", name);
        // TODO(tailhook) args
        write!(buf, ") {{\n");
        // TODO(tailhook) variables?
        write!(buf, "    return ");
        write!(buf, ";\n");
        write!(buf, "}}\n");
    } else {
        panic!("Wrong block type supplied to css::generate");
    }
    Ok(())
}
