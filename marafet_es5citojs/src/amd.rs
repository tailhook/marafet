use std::io::{Write, Result};

use parser::{Ast, Block};

use super::Generator;
use super::ast::{Code};
use super::ast::Statement::{Var, Expr};
use super::ast::Expression::{Call, Name, Str, Attr, Function};

impl<'a, W:Write+'a> Generator<'a, W> {
    pub fn wrap_amd(&self, code: Code, ast: &Ast) -> Code {
        let statements = ast.blocks.iter().flat_map(|b| match b {
            &Block::ImportModule(ref name, ref source)
            => vec!(Var(name.clone(),
                Call(Box::new(Name(String::from("require"))),
                     vec![Str(source.clone())]))
                ).into_iter(),
            &Block::ImportVars(ref items, ref source)
            => {
                let mut res = vec![
                    Var(String::from("_imp"),
                        Call(Box::new(Name(String::from("require"))),
                             vec![Str(source.clone())]))
                ];
                res.extend(
                    items.iter().map(|&(ref name, ref alias)|
                        Var(alias.clone().unwrap_or(name.clone()),
                            Attr(Box::new(Name(String::from("_imp"))),
                                 name.clone()))));
                res.into_iter()
                }
            _ => vec!().into_iter(),
        })
        .chain(code.statements.into_iter()).collect();
        return Code {
            statements: vec![
                Expr(Call(
                    Box::new(Name(String::from("define"))),
                    vec![Function(None, vec!(), statements)])),
            ],
        };
    }
}
