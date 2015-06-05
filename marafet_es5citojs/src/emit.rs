use std::io::{Write, Result};

use util::join;

use super::Generator;
use super::ast::{Code, Statement, Expression};

pub trait Emit {
    fn emit(&mut self, code: &Code) -> Result<()>;
}

impl<'a, W:Write+'a> Generator<'a, W> {
    fn write_indent(&mut self, indent: u32) -> Result<()> {
        // TODO(tailhook) Is there a beter way ?
        for i in 0..indent {
            try!(self.buf.write_all(b" "));
        }
        Ok(())
    }
    fn emit_expression(&mut self, expr: &Expression, indent: u32)
        -> Result<()>
    {
        try!(self.buf.write_all(b"<expr>"));
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
                    self.emit_expression(expr, nindent);
                    try!(self.buf.write_all(b"\n"));
                }
                &Statement::Return(ref expr) => {
                    try!(self.write_indent(indent));
                    try!(self.buf.write_all(b"return "));
                    self.emit_expression(expr, nindent);
                    try!(self.buf.write_all(b"\n"));
                }
                &Statement::Function(ref name, ref params, ref body) => {
                    try!(self.write_indent(indent));
                    write!(self.buf, "function {name}({params}) {{\n",
                        name=name,
                        params=join(params.iter().map(|x| &x.name), ", "));
                    // TODO(tailhook) default values
                    try!(self.emit_statements(&body, nindent));
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
