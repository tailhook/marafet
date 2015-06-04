use std::io::{Write, Result};

use super::grammar::Block;


pub fn generate<W>(buf: &mut W, blk: &Block) -> Result<()>
    where W: Write
{
    if let &Block::Html(ref name, ref args, ref statements) = blk {
        name;
        args;
        statements;
    } else {
        panic!("Wrong block type supplied to css::generate");
    }
    Ok(())
}
