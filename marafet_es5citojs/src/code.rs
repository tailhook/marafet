use std::io::{Write};

use parser::html;
use parser::html::Expression as Expr;
use parser::html::Statement as Stmt;
use parser::html::{Fmt};
use parser::html::Statement::{Element, Store, Condition, Output};
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

            // TODO(tailhook) validate numeric suffixes
            &Expr::Num(ref value) => Expression::Num(value.clone()),

            &Expr::New(ref expr)
            => Expression::New(Box::new(self.compile_expr(expr))),
            &Expr::Not(ref expr)
            => Expression::Not(Box::new(self.compile_expr(expr))),
            &Expr::And(ref a, ref b)
            => Expression::And(Box::new(self.compile_expr(a)),
                               Box::new(self.compile_expr(b))),
            &Expr::Or(ref a, ref b)
            => Expression::Or(Box::new(self.compile_expr(a)),
                               Box::new(self.compile_expr(b))),
            &Expr::Attr(ref expr, ref value)
            => Expression::Attr(Box::new(self.compile_expr(expr)),
                                value.clone()),
            &Expr::Call(ref expr, ref args)
            => Expression::Call(Box::new(self.compile_expr(expr)),
                args.iter().map(|x| self.compile_expr(x)).collect()),
            &Expr::Add(ref a, ref b)
            => Expression::Add(Box::new(self.compile_expr(a)),
                               Box::new(self.compile_expr(b))),
            &Expr::Sub(ref a, ref b)
            => Expression::Sub(Box::new(self.compile_expr(a)),
                               Box::new(self.compile_expr(b))),
            &Expr::Mul(ref a, ref b)
            => Expression::Mul(Box::new(self.compile_expr(a)),
                               Box::new(self.compile_expr(b))),
            &Expr::Div(ref a, ref b)
            => Expression::Div(Box::new(self.compile_expr(a)),
                               Box::new(self.compile_expr(b))),
            &Expr::Comparison(op, ref a, ref b)
            => Expression::Comparison(op,
                Box::new(self.compile_expr(a)),
                Box::new(self.compile_expr(b))),
            &Expr::Format(ref value) => {
                self.compile_format(value)
            }
            &Expr::Dict(ref items)
            => Expression::Object(items.iter()
                .map(|&(ref name, ref expr)|
                    (name.clone(), self.compile_expr(expr)))
                .collect()),
        }
    }
    fn compile_format(&self, items: &Vec<Fmt>) -> Expression
    {
        let mut exprs = items.iter().map(|e| match e {
            &Fmt::Raw(ref x) => Expression::Str(x.clone()),
            &Fmt::Str(ref e) => Expression::Call(
                Box::new(Expression::Name(String::from("String"))),
                vec![self.compile_expr(e)]),
            &Fmt::Int(ref e) => Expression::Call(
                Box::new(Expression::Name(String::from("String"))),
                vec![self.compile_expr(e)]),
            &Fmt::Float(ref e, _) => Expression::Call(
                Box::new(Expression::Name(String::from("String"))),
                vec![self.compile_expr(e)]),
        }).collect::<Vec<_>>();
        let first = exprs.remove(0);
        exprs.into_iter().fold(first, |acc, item| {
            Expression::Add(Box::new(acc), Box::new(item))
        })
    }

    fn attrs(&self, name: &String, cls: &Vec<(String, Option<Expr>)>,
        attrs: &Vec<(String, Expr)>)
        -> Expression
    {
        let mut class_literals = vec!();
        let mut class_expr = vec!();
        if cls.len() > 0 || self.bare_element_names.contains(name) {
            class_literals.push(self.block_name.to_string());
        }
        for &(ref cname, ref opt_cond) in cls {
            if let &Some(ref cond) = opt_cond {
                class_expr.push(Expression::Ternary(
                    Box::new(self.compile_expr(cond)),
                    Box::new(Expression::Str(cname.clone())),
                    Box::new(Expression::Str(String::from("")))));
            } else {
                class_literals.push(cname.clone());
            }
        }
        let mut rattrs = vec!();
        for &(ref name, ref value) in attrs {
            if &name[..] == "class" {
                class_expr.push(self.compile_expr(value));
            } else {
                rattrs.push((name.clone(), self.compile_expr(value)));
            }
        }
        if class_literals.len() > 0 {
            class_expr.insert(0,
                Expression::Str(join(class_literals.iter(), " ")));
        }
        if class_expr.len() > 0 {
            let first = class_expr.remove(0);
            rattrs.push((String::from("class"), class_expr.into_iter()
                .fold(first, |acc, val|
                    Expression::Add(Box::new(
                        Expression::Add(
                            Box::new(acc),
                            Box::new(Expression::Str(String::from(" "))))),
                        Box::new(val)))));
        }
        Expression::Object(rattrs)
    }
    fn element(&self, name: &String,
        classes: &Vec<(String, Option<Expr>)>,
        attributes: &Vec<(String, Expr)>,
        body: &Vec<Stmt>)
        -> Expression
    {
        use parser::html::Link as L;
        use parser::html::LinkDest as D;

        let mut properties = vec![
                (String::from("tag"), Expression::Str(name.clone())),
        ];
        let mut statements = vec![];
        let mut events = vec![];
        for item in body.iter() {
            match item {
                &Stmt::Let(ref name, ref value) => {
                    statements.push(Statement::Var(name.clone(),
                        self.compile_expr(value)));
                }
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
                            &L::One(ref s, D::Stream(ref expr)) => {
                                events.push((s.clone(),
                                    E::Attr(Box::new(self.compile_expr(expr)),
                                        String::from("handle_event")),
                                    ));
                            }
                            &L::Multi(ref names, D::Stream(ref expr)) => {
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
                            &L::One(_, D::Mapping(_, _)) => unimplemented!(),
                            &L::Multi(_, D::Mapping(_, _)) => unimplemented!(),
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
    fn statement(&self, st: &html::Statement) -> Expression {
        match st {
            &Element { ref name, ref classes, ref body, ref attributes } => {
                self.element(name, classes, attributes, body)
            }
            &Stmt::Format(ref value) => {
                self.compile_format(value)
            }
            &Output(ref expr) => {
                self.compile_expr(expr)
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
            &Stmt::ForOf(ref name, ref expr, ref body) => {
                let expr = self.compile_expr(expr);
                Expression::Call(
                    Box::new(Expression::Attr(
                        Box::new(expr),
                        String::from("map"))),
                    vec![Expression::Function(None,
                        vec![Param {name: name.clone(), default_value: None}],
                        vec![Statement::Return(self.fragment(body))])])
            }
            &Stmt::Store(_, _) => unreachable!(),  // not an actual child
            &Stmt::Link(_) => unreachable!(),  // not an actual child
            &Stmt::Let(_, _) => unreachable!(),  // not an actual child
        }
    }
    fn fragment(&self, statements: &Vec<html::Statement>) -> Expression {
        let stmt = statements.iter().filter(|x| match x {
            &&Stmt::Store(_, _) | &&Stmt::Link(_) | &&Stmt::Let(_, _) => false,
            _ => true,
            }).collect::<Vec<_>>();
        if stmt.len() == 1 {
            return self.statement(&stmt[0]);
        } else {
            return Expression::Object(vec![(
                String::from("children"),
                Expression::List(
                    stmt.iter()
                    .map(|s| self.statement(s))
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
