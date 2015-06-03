use std::str::{Chars};
use std::iter::Peekable;

use parser_combinators::primitives::{Stream, SourcePosition};
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
            Some((offset, grapheme)) => {
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

impl<'a> Stream for Tokenizer<'a> {
    type Item = Token<'a>;
    fn uncons(mut self) -> Result<(Token<'a>, Self), ()> {
        'outer: loop {
            match self.iter.next() {
                Some((ch, off, line, column)) => {
                    let pos = SourcePosition {
                        line: line,
                        column: column,
                        };
                    match ch {
                        '\n' => {
                            return Ok(((TokenType::Newline, "\n", pos), self));
                        }
                        '('|'{'|'[' => {
                            let typ = match ch {
                                '(' => TokenType::OpenParen,
                                '{' => TokenType::OpenBrace,
                                '[' => TokenType::OpenBracket,
                                _ => unreachable!(),
                            };
                            let mut nbraces = self.braces;
                            nbraces.push(ch);
                            return Ok((
                                (typ, &self.data[off..off+1], pos),
                                Tokenizer {
                                    braces: nbraces,
                                    .. self
                                }));
                        }
                        ')'|'}'|']' => {
                            let (typ, val) = match ch {
                                ')' => (TokenType::CloseParen, '('),
                                '}' => (TokenType::CloseBrace, '}'),
                                ']' => (TokenType::CloseBracket, ']'),
                                _ => unreachable!(),
                            };
                            let mut nbraces = self.braces;
                            let br = nbraces.pop();
                            if Some(val) == br {
                                return Ok((
                                    (typ, &self.data[off..off+1], pos),
                                    Tokenizer {
                                        braces: nbraces,
                                        .. self
                                    }));
                            } else {
                                // TODO(tailhook) how to communicate error?
                                return Err(());
                            }
                        }
                        ':'|'.'|'=' => {
                            let typ = match ch {
                                ':' => TokenType::Colon,
                                '.' => TokenType::Dot,
                                '=' => TokenType::Equals,
                                _ => unreachable!(),
                            };
                            return Ok((
                                (typ, &self.data[off..off+1], pos),
                                self,
                                ));
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
                        '"' => {
                            unimplemented!();
                        }
                        ' ' if column == 1 => {
                            let mut offset = self.data.len();
                            let mut indent = 0;
                            loop {
                                match self.iter.peek() {
                                    Some(('\n', off, _, _)) => {
                                        continue 'outer;
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
                                self.iter.next();
                            }
                            let mut nindents = self.indents;
                            let chunk = &self.data[off..offset];
                            let mut typ;
                            if indent > nindents[nindents.len()-1] {
                                nindents.push(indent);
                                typ = TokenType::Indent;
                            } else {
                                loop {
                                    let nindent = nindents.pop().unwrap();
                                    if nindent == indent {
                                        nindents.push(indent);
                                        break;
                                    } else if nindent < indent {
                                        // TODO(tailhook) how to report err?
                                        return Err(());
                                    }
                                }
                                typ = TokenType::Dedent;
                            }
                            return Ok((
                                (typ, chunk, pos),
                                Tokenizer {
                                    indents: nindents,
                                    .. self
                                }
                            ));
                        }
                        ' '|'\t' => {  // Skip whitespace
                            continue;
                        }
                        'a'...'z'|'A'...'Z' => {
                            let mut offset = self.data.len();
                            loop {
                                match self.iter.peek() {
                                    Some((x, off, _, _)) => {
                                        match x {
                                            'a'...'z'|'A'...'Z'|'0'...'9' => {}
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
                                _ => TokenType::Ident,
                            };
                            return Ok(((tok, value, pos), self));
                        }
                        _ => {
                            return Err(()); // Unexpected character
                        }
                    }
                }
                None => {
                    match self.indents.pop() {
                        Some(level) => {
                            let pos = SourcePosition {
                                line: -1,
                                column: 0,
                                };
                            return Ok(((TokenType::Dedent, "", pos), self));
                        }
                        None => {
                            return Err(());
                        }
                    }
                }
            }
        }
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
    pub fn end_of_file(&mut self) -> bool {
        self.iter.peek().is_none()
    }
    pub fn error_message(&self) -> String {
        return format!("Unexpected character {:?} at: {}:{}",
            &self.data[self.iter.offset..][..1],
            self.iter.line,
            self.iter.column);
    }
}
