use std::io::{Write};

use parser::html;
use parser::html::Expression as Expr;
use parser::html::Statement::{Element, Text, Store, Link, Condition};
use parser::{Ast, Block};
use util::join;

use super::ast::{Code, Statement, Param, Expression};

use super::Generator;


impl<'a, W:Write+'a> Generator<'a, W> {

    fn compile_expr(&self, expr: &Expr) -> Expression
    {
        match expr {
            &Expr::Name(ref name) => Expression::Name(name.clone()),
            &Expr::Str(ref value) => Expression::Str(value.clone()),
            &Expr::New(ref expr)
            => Expression::New(Box::new(self.compile_expr(expr))),
            &Expr::Attr(ref expr, ref value)
            => Expression::Attr(Box::new(self.compile_expr(expr)),
                                value.clone()),
            &Expr::Call(ref expr, ref args)
            => Expression::Call(Box::new(self.compile_expr(expr)),
                args.iter().map(|x| self.compile_expr(x)).collect()),
        }
    }

    fn attrs(&self, name: &String, cls: &Vec<String>,
        attrs: &Vec<(String, Expr)>)
        -> Expression
    {
        let mut rattrs = vec!();
        if cls.len() > 0 {
            let namestr = &self.block_name.to_string();
            let nclasses = vec![namestr].into_iter()
                .chain(cls.iter());
            rattrs.push((String::from("class"),
                        Expression::Str(join(nclasses, " "))));
        } else if self.bare_element_names.contains(name) {
            rattrs.push((String::from("class"),
                Expression::Str(self.block_name.to_string())));
        }
        for &(ref name, ref value) in attrs {
            rattrs.push((name.clone(), self.compile_expr(value)));
        }
        Expression::Object(rattrs)
    }
    fn element(&self, st: &html::Statement) -> Expression {
        match st {
            &Element { ref name, ref classes, ref body, ref attributes } => {
                if classes.len() == 0 && body.len() == 0 {
                    Expression::Object(vec![
                        (String::from("tag"), Expression::Str(name.clone())),
                        ])
                } else {
                    Expression::Object(vec![
                        (String::from("tag"), Expression::Str(name.clone())),
                        (String::from("attrs"),
                            self.attrs(name, classes, attributes)),
                        (String::from("children"), self.fragment(&body)),
                        ])
                }
            }
            &Text(ref value) => {
                Expression::Str(value.clone())
            }
            &Condition(ref conditions, ref fallback) => {
                conditions.iter().rev()
                .fold(fallback.as_ref()
                    .map(|x| self.fragment(x))
                    .unwrap_or(Expression::Str(String::new())),
                    |old, &(ref cond, ref value)| Expression::Ternary(
                        Box::new(self.compile_expr(cond)),
                        Box::new(self.fragment(value)),
                        Box::new(old),
                    ))
            }
            &Store(_, _) => unreachable!(),  // not an actual child
            &Link(_) => unreachable!(),  // not an actual child
        }
    }
    fn fragment(&self, statements: &Vec<html::Statement>) -> Expression {
        let stmt = statements.iter().filter(|x| match x {
            &&Store(_, _) | &&Link(_) => false,
            _ => true,
            }).collect::<Vec<_>>();
        if statements.len() == 1 {
            return self.element(&statements[0]);
        } else {
            return Expression::Object(vec![(
                String::from("children"),
                Expression::List(
                    stmt.iter()
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
