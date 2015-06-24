use std::io::Write;
use std::collections::HashMap;

use parser::html::Expression as Expr;
use parser::html::Statement as Stmt;
use parser::html::Statement::{Store};
use util::join;
use super::Generator;
use super::ast::{Statement, Param, Expression};
use super::ast::Expression as E;
use super::ast::Statement as S;

fn attr<'x, S: AsRef<str>>(e: Expression, v: S) -> Expression {
    E::Attr(Box::new(e), v.as_ref().to_string())
}


impl<'a, W:Write+'a> Generator<'a, W> {

    fn event_func(&self, e: &Expr) -> Expression {
        // TODO(tailhook) add conditional ev(event) parameter
        E::Function(None, vec![], vec![
            S::Return(self.compile_expr(e)),
        ])
    }

    pub fn element(&self, name: &String,
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
        let mut events = HashMap::new();
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
                                events.entry(s.clone()).or_insert(vec!()).push(
                                    E::Attr(Box::new(self.compile_expr(expr)),
                                        String::from("handle_event")),
                                    );
                            }
                            &L::Multi(ref names, D::Stream(ref expr)) => {
                                let v = format!("_stream_{}",
                                    statements.len());
                                statements.push(Statement::Var(
                                    v.clone(),
                                    self.compile_expr(expr)));
                                for &(ref aname, ref ename) in names {
                                    let ev = ename.as_ref()
                                             .unwrap_or(aname).clone();
                                    events.entry(ev.clone()).or_insert(vec!())
                                        .push(attr(
                                            attr(E::Name(v.clone()), &aname),
                                            "handle_event"));
                                }
                            }
                            &L::One(ref s, D::Mapping(ref val, ref dest)) => {
                                events.entry(s.clone()).or_insert(vec!()).push(
                                    attr(
                                    E::Call(Box::new(
                                        attr(self.compile_expr(dest), "map")),
                                        vec![ self.event_func(val) ]),
                                    "handle_event",
                                ));
                            }
                            &L::Multi(ref names, D::Mapping(ref val, ref dest))
                            => {
                                let v = format!("_stream_{}",
                                    statements.len());
                                statements.push(Statement::Var(
                                    v.clone(),
                                    self.compile_expr(dest)));
                                for &(ref aname, ref event) in names {
                                    let ename = event.as_ref()
                                             .unwrap_or(aname).clone();
                                    events.entry(ename).or_insert(vec!()).push(
                                        attr(
                                        E::Call(Box::new(attr(
                                            attr(E::Name(v.clone()), aname),
                                            "map")),
                                            vec![ self.event_func(val) ]),
                                        "handle_event"),
                                    );
                                }
                            }
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
            properties.push( (String::from("events"), Expression::Object(
                events.into_iter().map(|(k, mut v)| {
                    if v.len() == 1 {
                        (k, v.pop().unwrap())
                    } else {
                        (k, Expression::List(v))
                    }
                }).collect()
            )));
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

}
