use super::grammar::Ast;

mod bare_elements;
mod class_names;


pub fn add_block_name(ast: Ast, name: &str) -> Ast {
    let elements = bare_elements::visitor(&ast);
    Ast {
        blocks: ast.blocks.into_iter()
            .map(|blk| class_names::add_name_to_block(blk, name, &elements))
            .collect(),
        .. ast
    }
}

