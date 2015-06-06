use std::io::{Write};

use super::Generator;

use super::ast::{Code};
use super::ast::Statement::{Var, Expr};
use super::ast::Expression::{Call, Name, Str, Attr};


impl<'a, W:Write+'a> Generator<'a, W> {

    pub fn add_css(&self, code: Code, css: &str) -> Code {
        let stmt = vec![
            // var _style = document.createElement('style')
            Var(String::from("_style"), Call(
                    Box::new(Attr(Box::new(Name(String::from("document"))),
                                  String::from("createElement"))),
                    vec![Str(String::from("style"))])),
            // _style.appendChild(document.createTextNode(css_text))
            Expr(Call(
                    Box::new(Attr(Box::new(Name(String::from("_style"))),
                                  String::from("appendChild"))),
                    vec![Call(
                        Box::new(Attr(Box::new(Name(String::from("document"))),
                                  String::from("createTextNode"))),
                        vec![Str(css.to_string())])])),
            // document.head.appendChild(_style)
            Expr(Call(Box::new(
                    Attr(Box::new(Attr(Box::new(Name(String::from("document"))),
                                       String::from("head"))),
                         String::from("appendChild"))),
                    vec![Name(String::from("_style"))])),
        ];
        return Code {
            statements: stmt.into_iter()
                        .chain(code.statements.into_iter())
                        .collect(),
        }
    }

}
