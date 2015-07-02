use std::io::{Write, Result};

use util::join;
use parser::html::Comparator;

use super::Generator;
use super::ast::{Code, Statement, Expression};

pub trait Emit {
    fn emit(&mut self, code: &Code) -> Result<()>;
}

fn write_str<W:Write, S: AsRef<str>>(w: &mut W, val: S) -> Result<()> {
    try!(w.write_all(b"\""));
    for ch in val.as_ref().chars() {
        match ch {
            '\r' => { try!(write!(w, "\\r")); }
            '\n' => { try!(write!(w, "\\n")); }
            '\t' => { try!(write!(w, "\\t")); }
            '\"' => { try!(write!(w, "\\\"")); }
            '\'' => { try!(write!(w, "\\\'")); }
            '\x00'...'\x1f' => { try!(write!(w, "\\x{:02}", ch as u8)) }
            _ => { try!(write!(w, "{}", ch)) }
        }
    }
    try!(w.write_all(b"\""));
    Ok(())
}

fn is_ident<S: AsRef<str>>(s: S) -> bool {
    let s = s.as_ref();
    if s.len() == 0 {
        return false;
    }
    let mut iter = s.chars();
    match iter.next().unwrap() {
        'a'...'z'|'A'...'Z'|'_' => {},
        _ => return false,
    }
    for ch in iter {
        match ch {
            'a'...'z'|'A'...'Z'|'0'...'9'|'_' => {}
            _ => return false,
        }
    }
    return true;
}

impl<'a, W:Write+'a> Generator<'a, W> {
    fn write_indent(&mut self, indent: u32) -> Result<()> {
        // TODO(tailhook) Is there a beter way ?
        for _ in 0..indent {
            try!(self.buf.write_all(b" "));
        }
        Ok(())
    }
    fn emit_expression(&mut self, expr: &Expression, indent: u32)
        -> Result<()>
    {
        let nindent = self.indent + indent;
        match expr {
            &Expression::Str(ref s) => {
                try!(write_str(self.buf, &s[..]));
            }
            &Expression::Num(ref s) => {
                try!(write!(self.buf, "{}", s));
            }
            &Expression::Object(ref pairs) => {
                try!(self.buf.write_all(b"{"));
                if pairs.len() == 0 {
                } else if pairs.len() == 1 {
                    if is_ident(&pairs[0].0[..]) {
                        try!(write!(self.buf, "{}: ", pairs[0].0));
                    } else {
                        try!(write_str(self.buf, &pairs[0].0));
                        try!(write!(self.buf, ": "));
                    }
                    try!(self.emit_expression(&pairs[0].1, indent));
                } else {
                    try!(self.buf.write_all(b"\n"));
                    for &(ref key, ref value) in pairs.iter() {
                        try!(self.write_indent(nindent));
                        if is_ident(&key[..]) {
                            try!(write!(self.buf, "{}: ", key));
                        } else {
                            try!(write_str(self.buf, key));
                            try!(write!(self.buf, ": "));
                        }
                        try!(self.emit_expression(value, nindent));
                        try!(self.buf.write_all(b",\n"));
                    }
                    try!(self.write_indent(indent));
                }
                try!(self.buf.write_all(b"}"));
            }
            &Expression::List(ref lst) => {
                try!(self.buf.write_all(b"["));
                if lst.len() == 0 {
                } else if lst.len() == 1 {
                    try!(self.emit_expression(&lst[0], indent));
                } else {
                    try!(self.buf.write_all(b"\n"));
                    for item in lst.iter() {
                        try!(self.write_indent(nindent));
                        try!(self.emit_expression(item, nindent));
                        try!(self.buf.write_all(b",\n"));
                    }
                    try!(self.write_indent(indent));
                }
                try!(self.buf.write_all(b"]"));
            }
            &Expression::Name(ref s) => {
                try!(write!(self.buf, "{}", s));
            }
            &Expression::Attr(ref parent, ref attr) => {
                try!(self.emit_expression(parent, indent));
                try!(write!(self.buf, ".{}", attr));
            }
            &Expression::Item(ref parent, ref item) => {
                try!(self.emit_expression(parent, indent));
                try!(write!(self.buf, "["));
                try!(self.emit_expression(item, indent));
                try!(write!(self.buf, "]"));
            }
            &Expression::Call(ref parent, ref args) => {
                try!(self.emit_expression(parent, indent));
                try!(self.buf.write_all(b"("));
                if args.len() > 0 {
                    try!(self.emit_expression(&args[0], indent));
                    for i in args[1..].iter() {
                        try!(self.buf.write_all(b", "));
                        try!(self.emit_expression(i, indent));
                    }
                }
                try!(self.buf.write_all(b")"));
            }
            &Expression::New(ref val) => {
                try!(write!(self.buf, "new "));
                try!(self.emit_expression(val, indent));
            }
            &Expression::Not(ref val) => {
                try!(write!(self.buf, "!"));
                try!(self.emit_expression(val, indent));
            }
            &Expression::Or(ref left, ref right) => {
                try!(self.emit_expression(left, indent));
                try!(write!(self.buf, " || "));
                try!(self.emit_expression(right, indent));
            }
            &Expression::And(ref left, ref right) => {
                try!(self.emit_expression(left, indent));
                try!(write!(self.buf, " && "));
                try!(self.emit_expression(right, indent));
            }
            &Expression::Add(ref left, ref right) => {
                try!(self.emit_expression(left, indent));
                try!(write!(self.buf, " + "));
                try!(self.emit_expression(right, indent));
            }
            &Expression::Sub(ref left, ref right) => {
                try!(self.emit_expression(left, indent));
                try!(write!(self.buf, " - "));
                try!(self.emit_expression(right, indent));
            }
            &Expression::Mul(ref left, ref right) => {
                try!(self.emit_expression(left, indent));
                try!(write!(self.buf, " * "));
                try!(self.emit_expression(right, indent));
            }
            &Expression::Div(ref left, ref right) => {
                try!(self.emit_expression(left, indent));
                try!(write!(self.buf, " / "));
                try!(self.emit_expression(right, indent));
            }
            &Expression::Comparison(op, ref left, ref right) => {
                try!(self.emit_expression(left, indent));
                try!(write!(self.buf, " {} ", match op {
                    Comparator::Eq => "===",
                    Comparator::NotEq => "!==",
                    Comparator::Less => "<",
                    Comparator::LessEq => "<=",
                    Comparator::Greater => ">",
                    Comparator::GreaterEq => ">=",
                }));
                try!(self.emit_expression(right, indent));
            }
            &Expression::Function(ref name, ref params, ref body) => {
                try!(write!(self.buf, "function {name}({params}) {{\n",
                    name=name.as_ref().unwrap_or(&String::from("")),
                    params=join(params.iter().map(|x| &x.name), ", ")));
                // TODO(tailhook) default values
                try!(self.emit_statements(&body, nindent));
                try!(self.write_indent(indent));
                try!(self.buf.write_all(b"}"));
            }
            &Expression::AssignAttr(ref expr, ref attr, ref value) => {
                try!(self.emit_expression(expr, indent));
                try!(write!(self.buf, ".{} = ", attr));
                try!(self.emit_expression(value, indent));
            }
            &Expression::Ternary(ref cond, ref left, ref right) => {
                try!(write!(self.buf, "(("));
                try!(self.emit_expression(cond, indent));
                try!(write!(self.buf, ")?("));
                try!(self.emit_expression(left, indent));
                try!(write!(self.buf, "):("));
                try!(self.emit_expression(right, indent));
                try!(write!(self.buf, "))"));
            }
        }
        Ok(())
    }
    fn emit_statements(&mut self, stmts: &Vec<Statement>, indent: u32)
        -> Result<()>
    {
        let nindent = indent + self.indent;
        for stmt in stmts.iter() {
            match stmt {
                &Statement::Expr(ref expr) => {
                    try!(self.write_indent(indent));
                    try!(self.emit_expression(expr, nindent));
                    try!(self.buf.write_all(b"\n"));
                }
                &Statement::Return(ref expr) => {
                    try!(self.write_indent(indent));
                    try!(self.buf.write_all(b"return "));
                    try!(self.emit_expression(expr, nindent));
                    try!(self.buf.write_all(b";\n"));
                }
                &Statement::Var(ref name, ref expr) => {
                    try!(self.write_indent(indent));
                    try!(write!(self.buf, "var {} = ", name));
                    try!(self.emit_expression(expr, nindent));
                    try!(self.buf.write_all(b";\n"));
                }
                &Statement::Function(ref name, ref params, ref body) => {
                    try!(self.write_indent(indent));
                    try!(write!(self.buf, "function {name}({params}) {{\n",
                        name=name,
                        params=join(params.iter().map(|x| &x.name), ", ")));
                    // TODO(tailhook) default values
                    try!(self.emit_statements(&body, nindent));
                    try!(self.write_indent(indent));
                    try!(self.buf.write_all(b"}\n"));
                }
            }
        }
        Ok(())
    }
}

impl<'a, W:Write+'a> Emit for Generator<'a, W> {
    fn emit(&mut self, code: &Code) -> Result<()> {
        try!(self.emit_statements(&code.statements, 0));
        Ok(())
    }
}
