use std::io::{Write};
use std::collections::HashMap;

use parser::{Ast, Block};

use super::Generator;
use super::ast::{Code, Param};
use super::ast::Statement::{Var, Expr};
use super::ast::Expression::{Call, Name, Str, Attr, Function, List};
use super::ast::Expression::{AssignAttr};


fn string_to_ident(src: &str) -> String {
    let mut res = String::new();
    res.push_str("_mod_");
    for ch in src.chars() {
        match ch {
            'a'...'z'|'A'...'Z'|'0'...'9'|'_' => res.push(ch),
            _ => res.push('_'),
        }
    }
    return res;
}

impl<'a, W:Write+'a> Generator<'a, W> {
    pub fn wrap_amd(&self, code: Code, ast: &Ast) -> Code {
        let mut code_prefix = vec![];
        let mut code_suffix = vec![];
        let mut dependencies = vec![
            Str(String::from("require")),
            Str(String::from("exports")),
            ];
        let mut arguments = vec![
            Param { name: String::from("require"), default_value: None },
            Param { name: String::from("exports"), default_value: None },
        ];
        let mut modules = HashMap::new();
        for block in ast.blocks.iter() {
            match block {
                &Block::ImportModule(ref name, ref source) => {
                    dependencies.push(Str(source.clone()));
                    arguments.push(Param {
                        name: name.clone(),
                        default_value: None,
                        });
                }
                &Block::ImportVars(ref items, ref source) => {
                    if !modules.contains_key(source) {
                        let varname = string_to_ident(source);
                        arguments.push(Param {
                            name: varname.clone(),
                            default_value: None,
                            });
                        modules.insert(source, varname);
                        dependencies.push(Str(source.clone()));
                    }
                    let varname = &modules[source];
                    for &(ref name, ref alias) in items.iter() {
                        code_prefix.push(
                            Var(alias.as_ref().unwrap_or(name).clone(),
                                Attr(Box::new(Name(varname.clone())),
                                     name.clone())));
                    }
                }
                &Block::Html(ref name, _, _) => {
                    code_suffix.push(Expr(AssignAttr(
                        Box::new(Name(String::from("exports"))),
                        name.clone(),
                        Box::new(Name(String::from(name.clone()))),
                        )));
                }
                // TODO(tailhook) may be export css too?
                &Block::Css(_, _) => {}
            }
        }
        let mut body = code_prefix;
        body.extend(code.statements.into_iter());
        body.extend(code_suffix.into_iter());
        return Code {
            statements: vec![
                Expr(Call(Box::new(Name(String::from("define"))), vec![
                    Str(self.amd_name.to_string()),
                    List(dependencies),
                    Function(None, arguments, body)
                ])),
            ],
        };
    }
}
