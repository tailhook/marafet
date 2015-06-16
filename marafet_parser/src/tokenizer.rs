use std::str::{Chars};
use std::iter::Peekable;

use parser_combinators::primitives::{SourcePosition};
use unicode_segmentation::{UnicodeSegmentation, GraphemeIndices};

use super::token::{Token, TokenType};

#[derive(Clone)]
struct CodeIter<'a> {
    iter: GraphemeIndices<'a>,
    buf: Option<Peekable<Chars<'a>>>,
    grapheme: Option<&'a str>,
    offset: usize,
    line: i32,
    column: i32,
}

impl <'a> Iterator for CodeIter<'a> {
    type Item = (char, usize, i32, i32);

    fn next(&mut self) -> Option<(char, usize, i32, i32)> {
        if self.buf.is_none() {
            if self.peek().is_none() {
                return None;
            }
        }
        if let Some(ref mut buf) = self.buf {
            match buf.next() {
                Some(ch) => {
                    return Some((ch, self.offset, self.line, self.column));
                }
                None => {}
            }
        }
        self.buf = None;
        return self.next();
    }
}

impl <'a> CodeIter<'a> {
    fn peek(&mut self) -> Option<(char, usize, i32, i32)> {
        if let Some(ref mut buf) = self.buf {
            if let Some(ch) = buf.peek() {
                return Some((*ch, self.offset, self.line, self.column));
            }
        }
        if let Some(gr) = self.grapheme {
            if gr == "\n" {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
            self.offset += gr.as_bytes().len();
            self.buf = None;
            self.grapheme = None;
        }
        match self.iter.next() {
            Some((_, grapheme)) => {
                self.grapheme = Some(grapheme);
                self.buf = Some(grapheme.chars().peekable());
                self.peek()
            }
            None => None,
        }
    }
}


#[derive(Clone)]
pub struct Tokenizer<'a> {
    data: &'a str,
    iter: CodeIter<'a>,
    braces: Vec<char>,
    indents: Vec<usize>,
}

impl<'a> Tokenizer<'a> {

    pub fn new(val: &'a str) -> Tokenizer<'a> {
        return Tokenizer {
            data: val,
            iter: CodeIter {
                iter: UnicodeSegmentation::grapheme_indices(val, true),
                buf: None,
                grapheme: None,
                offset: 0,
                line: 1,
                column: 1,
                },
            braces: vec!(),
            indents: vec!(0),
        };
    }
}

impl<'a> Iterator for Tokenizer<'a> {
    type Item = Token<'a>;
    fn next(&mut self) -> Option<Token<'a>> {
        'outer: loop {
            match self.iter.peek() {
                Some(('\n', _, _, 1)) => {
                    self.iter.next();
                    continue 'outer;
                }
                Some((' ', _, _, 1)) if self.braces.len() == 0 => {
                    let mut niter = self.iter.clone();
                    loop {
                        niter.next();
                        match niter.peek() {
                            Some((' ', _, _, _)) => continue,
                            Some(('#', _, _, _)) => {
                                self.iter = niter;
                                loop {
                                    match self.iter.next() {
                                        Some(('\n', _, _, _)) | None
                                        => continue 'outer,
                                        _ => continue,
                                    }
                                }
                            }
                            Some(('\n', _, _, _)) | None => {
                                self.iter = niter;
                                continue 'outer;
                            }
                            Some((_, _, line, col)) => {
                                let indent = (col - 1) as usize;
                                let curindent = *self.indents.last().unwrap();
                                let typ;
                                if indent == curindent {
                                    self.iter = niter;    // always commit
                                    continue 'outer;
                                } else if indent > curindent {
                                    self.indents.push(indent);
                                    self.iter = niter;    // always commit
                                    typ = TokenType::Indent;
                                } else {
                                    self.indents.pop();
                                    let nind = *self.indents.last().unwrap();
                                    if nind < indent {
                                        // TODO(tailhook) how to report err?
                                        return None;
                                    } else if nind == indent {
                                        self.iter = niter;  // commit if last
                                    }
                                    typ = TokenType::Dedent;
                                }
                                let pos = SourcePosition {
                                    line: line,
                                    column: col,
                                    };
                                return Some((typ, "", pos));
                            }
                        }
                    }
                }
                Some(('#', _, _, 1)) => {
                    loop {
                        match self.iter.next() {
                            Some(('\n', _, _, _)) | None => continue 'outer,
                            _ => continue,
                        }
                    }
                }
                Some((ch, _, _, 1)) if ch != ' ' && self.indents.len() > 1 => {
                    self.indents.pop().unwrap();
                    let pos = SourcePosition {
                        line: self.iter.line,
                        column: self.iter.column,
                        };
                    return Some((TokenType::Dedent, "", pos));
                }
                _ => {}
            }
            match self.iter.next() {
                Some((ch, off, line, column)) => {
                    let pos = SourcePosition {
                        line: line,
                        column: column,
                        };
                    match ch {
                        '\n' => {
                            if column == 1 {
                                continue;  // skip empty line
                            }
                            if self.braces.len() == 0 {
                                return Some((TokenType::Newline, "\n", pos));
                            } else {
                                continue 'outer;
                            }
                        }
                        '('|'{'|'[' => {
                            let typ = match ch {
                                '(' => TokenType::OpenParen,
                                '{' => TokenType::OpenBrace,
                                '[' => TokenType::OpenBracket,
                                _ => unreachable!(),
                            };
                            self.braces.push(ch);
                            return Some((typ, &self.data[off..off+1], pos));
                        }
                        ')'|'}'|']' => {
                            let (typ, val) = match ch {
                                ')' => (TokenType::CloseParen, '('),
                                '}' => (TokenType::CloseBrace, '{'),
                                ']' => (TokenType::CloseBracket, '['),
                                _ => unreachable!(),
                            };
                            let br = self.braces.pop();
                            if Some(val) == br {
                                return Some((typ, &self.data[off..off+1], pos));
                            } else {
                                // TODO(tailhook) how to communicate error?
                                return None;
                            }
                        }
                        ':'|'.'|'='|','|'-'|'+'|'*'|'/'|'?'|'>'|'<'|'!' => {
                            let typ = match ch {
                                '+' => TokenType::Plus,
                                '*' => TokenType::Multiply,
                                '/' => TokenType::Divide,
                                '?' => TokenType::Question,
                                ':' => TokenType::Colon,
                                '.' => TokenType::Dot,
                                '=' => {
                                    match self.iter.peek() {
                                        Some(('=', _, _, _)) => {
                                            self.iter.next();
                                            TokenType::Eq
                                        }
                                        _ => TokenType::Equals,
                                    }
                                }
                                '!' => {
                                    match self.iter.peek() {
                                        Some(('=', _, _, _)) => {
                                            self.iter.next();
                                            TokenType::NotEq
                                        }
                                        _ => TokenType::Not, //TODO remove?
                                    }
                                }
                                '>' => {
                                    match self.iter.peek() {
                                        Some(('=', _, _, _)) => {
                                            self.iter.next();
                                            TokenType::GreaterEq
                                        }
                                        _ => TokenType::Greater,
                                    }
                                }
                                '<' => {
                                    match self.iter.peek() {
                                        Some(('=', _, _, _)) => {
                                            self.iter.next();
                                            TokenType::LessEq
                                        }
                                        _ => TokenType::Less,
                                    }
                                }
                                '-' => {
                                    match self.iter.peek() {
                                        Some(('>', _, _, _)) => {
                                            self.iter.next();
                                            TokenType::ArrowRight
                                        }
                                        _ => TokenType::Dash,
                                    }
                                }
                                ',' => TokenType::Comma,
                                _ => unreachable!(),
                            };
                            return Some((typ,
                                &self.data[off..self.iter.offset],
                                pos));
                        }
                        '#' => {
                            loop {
                                match self.iter.next() {
                                    Some(('\n', _, _, _)) | None => break,
                                    Some(_) => {}
                                }
                            }
                            if self.braces.len() == 0 {
                                return Some((TokenType::Newline, "\n", pos));
                            } else {
                                continue 'outer;
                            }
                        }
                        '"'|'\'' => {
                            let dlm = ch;
                            loop {
                                match self.iter.next() {
                                    Some((ch, _, _, _)) if ch == dlm => {
                                        break;
                                    }
                                    Some(('\\', _, _, _)) => {
                                        // any char allowed after slash
                                        self.iter.next();
                                    }
                                    Some(_) => {}
                                    None => {
                                        // TODO(tailhook) return error
                                        break;
                                    }
                                }
                            }
                            let value = &self.data[off..self.iter.offset+1];
                            return Some((TokenType::String, value, pos));
                        }
                        ' '|'\t' => {  // Skip whitespace
                            continue;
                        }
                        'a'...'z'|'A'...'Z'|'_' => {
                            let mut offset = self.data.len();
                            loop {
                                match self.iter.peek() {
                                    Some((x, off, _, _)) => {
                                        match x {
                                            'a'...'z'|'A'...'Z'|'0'...'9'|'_'
                                            => {}
                                            _ => {
                                                offset = off;
                                                break;
                                            }
                                        }
                                    }
                                    None => break,
                                }
                                self.iter.next();
                            }
                            let value = &self.data[off..offset];
                            let tok = match value {
                                "css" => TokenType::Css,
                                "html" => TokenType::Html,
                                "import" => TokenType::Import,
                                "from" => TokenType::From,
                                "if" => TokenType::If,
                                "elif" => TokenType::Elif,
                                "for" => TokenType::For,
                                "in" => TokenType::In,
                                "of" => TokenType::Of,
                                "as" => TokenType::As,
                                "else" => TokenType::Else,
                                "events" => TokenType::Events,
                                "store" => TokenType::Store,
                                "let" => TokenType::Let,
                                "link" => TokenType::Link,
                                "new" => TokenType::New,
                                "not" => TokenType::Not,
                                "and" => TokenType::And,
                                "or" => TokenType::Or,
                                _ => TokenType::Ident,
                            };
                            return Some((tok, value, pos));
                        }
                        '0'...'9' => {
                            let mut offset = self.data.len();
                            loop {
                                match self.iter.peek() {
                                    Some((x, off, _, _)) => {
                                        match x {
                                            'a'...'z'|'A'...'Z'
                                            |'0'...'9'|'_'|'.'
                                            => {}
                                            _ => {
                                                offset = off;
                                                break;
                                            }
                                        }
                                    }
                                    None => break,
                                }
                                self.iter.next();
                            }
                            let value = &self.data[off..offset];
                            return Some((TokenType::Number, value, pos));
                        }
                        _ => {
                            return None; // Unexpected character
                        }
                    }
                }
                None => {
                    let pos = SourcePosition {
                        line: self.iter.line,
                        column: self.iter.column,
                        };
                    match self.indents.pop() {
                        Some(level) if level > 0 => {
                            return Some((TokenType::Dedent, "", pos));
                        }
                        _ => {
                            return Some((TokenType::Eof, "", pos));
                        }
                    }
                }
            }
        }
    }
}
