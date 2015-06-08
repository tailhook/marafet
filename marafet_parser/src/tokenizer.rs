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

impl<'a> Iterator for Tokenizer<'a> {
    type Item = Token<'a>;
    fn next(&mut self) -> Option<Token<'a>> {
        let tok = self._next();
        println!("Next token {:?}", tok);
        return tok;
    }
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
    fn _next(&mut self) -> Option<Token<'a>> {
        'outer: loop {
            match self.iter.peek() {
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
                            return Some((TokenType::Newline, "\n", pos));
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
                        ':'|'.'|'='|'-'|',' => {
                            let typ = match ch {
                                ':' => TokenType::Colon,
                                '.' => TokenType::Dot,
                                '=' => TokenType::Equals,
                                '-' => TokenType::Dash,
                                ',' => TokenType::Comma,
                                _ => unreachable!(),
                            };
                            return Some((typ, &self.data[off..off+1], pos));
                        }
                        '#' => {
                            loop {
                                match self.iter.next() {
                                    Some(('\n', _, _, _)) | None => break,
                                    Some(_) => {}
                                }
                            }
                            continue;
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
                        ' ' if column == 1 => {
                            let mut offset;
                            let mut indent;
                            loop {
                                match self.iter.peek() {
                                    Some(('\n', _, _, _)) => {
                                        self.iter.next();  // skip empty line
                                        continue 'outer;
                                    }
                                    Some((' ', _, _, _)) => {
                                        self.iter.next();
                                        continue;
                                    }
                                    Some(('#', _, _, _)) => {
                                        continue 'outer;
                                    }
                                    Some((_, off, _, col)) => {
                                        offset = off;
                                        indent = (col - 1) as usize;
                                        break;
                                    }
                                    None => {
                                        continue 'outer;  // WS at EOF
                                    }
                                }
                            }
                            let chunk = &self.data[off..offset];
                            let mut typ;
                            let curindent = *self.indents.last().unwrap();
                            if indent == curindent {
                                continue;
                            } else if indent > curindent {
                                self.indents.push(indent);
                                typ = TokenType::Indent;
                            } else {
                                loop {
                                    let nindent = self.indents.pop().unwrap();
                                    if nindent == indent {
                                        self.indents.push(indent);
                                        break;
                                    } else if nindent < indent {
                                        // TODO(tailhook) how to report err?
                                        return None;
                                    }
                                }
                                typ = TokenType::Dedent;
                            }
                            return Some((typ, chunk, pos));
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
                                "for" => TokenType::For,
                                "in" => TokenType::In,
                                "of" => TokenType::Of,
                                "as" => TokenType::As,
                                "else" => TokenType::Else,
                                "events" => TokenType::Events,
                                "store" => TokenType::Store,
                                "link" => TokenType::Link,
                                "new" => TokenType::New,
                                _ => TokenType::Ident,
                            };
                            return Some((tok, value, pos));
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
