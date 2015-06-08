use std::io::{Write};

use parser::html;
use parser::html::Statement::{Element, Text};
use parser::{Ast, Block};
use util::join;

use super::ast::{Code, Statement, Param, Expression};

use super::Generator;


impl<'a, W:Write+'a> Generator<'a, W> {
    fn attrs(&self, name: &String, cls: &Vec<String>) -> Expression {
        let mut attrs = vec!();
        if cls.len() > 0 {
            let namestr = &self.block_name.to_string();
            let nclasses = vec![namestr].into_iter()
                .chain(cls.iter());
            attrs.push((String::from("class"),
                        Expression::Str(join(nclasses, " "))));
        } else if self.bare_element_names.contains(name) {
            attrs.push((String::from("class"),
                Expression::Str(self.block_name.to_string())));
        }
        Expression::Object(attrs)
    }
    fn element(&self, st: &html::Statement) -> Expression {
        match st {
            &Element { ref name, ref classes, ref body } => {
                if classes.len() == 0 && body.len() == 0 {
                    Expression::Object(vec![
                        (String::from("tag"), Expression::Str(name.clone())),
                        ])
                } else {
                    Expression::Object(vec![
                        (String::from("tag"), Expression::Str(name.clone())),
                        (String::from("attrs"), self.attrs(name, classes)),
                        (String::from("children"), self.fragment(&body)),
                        ])
                }
            }
            &Text(ref value) => {
                Expression::Str(value.clone())
            }
        }
    }
    fn fragment(&self, statements: &Vec<html::Statement>) -> Expression {
        if statements.len() == 1 {
            return self.element(&statements[0]);
        } else {
            return Expression::Object(vec![(
                String::from("children"),
                Expression::List(
                    statements.iter()
                    .map(|s| self.element(s))
                    .collect())
                )]);
        }
    }
    pub fn code(&self, ast: &Ast) -> Code {
        let mut stmt = vec!();
        for blk in ast.blocks.iter() {
            if let &Block::Html {ref name, ref params, ref statements, ..} = blk {
                stmt.push(Statement::Function(name.clone(),
                    params.iter().map(|p| Param {
                        name: p.name.clone(),
                        default_value: p.default_value.as_ref().map(
                            |v| Expression::Str(v.clone())),
                    }).collect(),
                    vec![
                        Statement::Return(self.fragment(statements)),
                    ]
                ));
            }
        }
        return Code {
            statements: stmt,
        }
    }

}
