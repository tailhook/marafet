use std::io::{Write};

use parser::html;
use parser::html::Expression as Expr;
use parser::html::Statement as Stmt;
use parser::html::{Link, LinkDest};
use parser::html::Statement::{Element, Text, Store, Condition};
use parser::{Ast, Block};
use util::join;

use super::ast::{Code, Statement, Param, Expression};
use super::ast::Expression as E;

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
                let mut properties = vec![
                        (String::from("tag"), Expression::Str(name.clone())),
                ];
                let mut statements = vec![];
                let mut events = vec![];
                for item in body.iter() {
                    match item {
                        &Store(ref name, ref value) => {
                            let prop = String::from("store_") +name;
                            statements.push(Statement::Var(name.clone(),
                                E::Or(
                                    Box::new(E::And(
                                        Box::new(E::Name(String::from("old_node"))),
                                        Box::new(E::Attr(Box::new(
                                                E::Name(String::from("old_node"))),
                                                prop.clone())))),
                                    Box::new(self.compile_expr(value)))));
                            properties.push((prop, E::Name(name.clone())));
                        }
                        &Stmt::Link(ref links) => {
                            for lnk in links {
                                match lnk {
                                    &Link::One(ref s, LinkDest::Stream(ref expr)) => {
                                        events.push((s.clone(),
                                            E::Attr(Box::new(self.compile_expr(expr)),
                                                String::from("handle_event")),
                                            ));
                                    }
                                    &Link::Multi(ref names, LinkDest::Stream(ref expr)) => {
                                        let v = format!("_stream_{}",
                                            statements.len());
                                        statements.push(Statement::Var(
                                            v.clone(),
                                            self.compile_expr(expr)));
                                        for &(ref attr, ref event) in names {
                                            events.push((
                                                event.as_ref().unwrap_or(attr)
                                                    .clone(),
                                                E::Attr(
                                                    Box::new(E::Attr(
                                                        Box::new(E::Name(v.clone())),
                                                        attr.clone())),
                                                    String::from("handle_event"))));
                                        }
                                    }
                                    &Link::One(_, LinkDest::Mapping(_, _)) => unimplemented!(),
                                    &Link::Multi(_, LinkDest::Mapping(_, _)) => unimplemented!(),
                                }
                            }
                        }
                        _ => {}
                    }
                }
                if classes.len() > 0 || attributes.len() > 0 {
                    properties.push(
                        (String::from("attrs"),
                            self.attrs(name, classes, attributes)));
                }
                if body.len() > 0 {
                    properties.push(
                        (String::from("children"), self.fragment(&body)));
                }
                if events.len() > 0 {
                    properties.push(
                        (String::from("events"), Expression::Object(events)));
                }
                if statements.len() > 0 {
                    statements.push(
                        Statement::Return(Expression::Object(properties)));
                    return Expression::Function(None,
                        vec![Param {
                            name: String::from("old_node"),
                            default_value: None,
                        }],
                        statements);
                } else {
                    return Expression::Object(properties);
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
            &Stmt::Store(_, _) => unreachable!(),  // not an actual child
            &Stmt::Link(_) => unreachable!(),  // not an actual child
        }
    }
    fn fragment(&self, statements: &Vec<html::Statement>) -> Expression {
        let stmt = statements.iter().filter(|x| match x {
            &&Stmt::Store(_, _) | &&Stmt::Link(_) => false,
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
