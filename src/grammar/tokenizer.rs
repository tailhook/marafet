use parser_combinators::primitives::{Stream, SourcePosition};

use super::token::{Token, TokenType};


#[derive(Clone)]
pub struct Tokenizer<'a> {
    data: &'a str,
    offset: usize,
    line: i32,
    column: i32,
    braces: Vec<char>,
    indents: Vec<usize>,
}

impl<'a> Stream for Tokenizer<'a> {
    type Item = Token<'a>;
    fn uncons(mut self) -> Result<(Token<'a>, Self), ()> {
        let mut iter = self.data[self.offset..].char_indices();
        let pos = SourcePosition { line: self.line, column: self.column };
        loop {
            match iter.next() {
                Some((off, ch)) => {
                    match ch {
                        '\n' => {
                            return Ok(((TokenType::Newline, "\n", pos),
                                  Tokenizer {
                                    offset: iter.next()
                                        .map(|(off, _)| off + self.offset)
                                        .unwrap_or(self.data.len()),
                                    line: self.line + 1,
                                    column: 1,
                                    .. self
                                }));
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
                            let oldoff = self.offset;
                            return Ok(((typ, &self.data[off..off+1], pos),
                                  Tokenizer {
                                    offset: iter.next()
                                        .map(|(off, _)| off + oldoff)
                                        .unwrap_or(self.data.len()),
                                    column: self.column + 1,
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
                            let oldoff = self.offset;
                            if Some(val) == br {
                                return Ok(((typ, &self.data[off..off+1], pos),
                                      Tokenizer {
                                        offset: iter.next()
                                            .map(|(off, _)| off + oldoff)
                                            .unwrap_or(self.data.len()),
                                        column: self.column + 1,
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
                            return Ok(((typ, &self.data[off..off+1], pos),
                                  Tokenizer {
                                    offset: iter.next()
                                        .map(|(off, _)| off + self.offset)
                                        .unwrap_or(self.data.len()),
                                    column: self.column + 1,
                                    .. self
                                }));
                        }
                        '#' => {
                            loop {
                                match iter.next() {
                                    Some((_, '\n')) | None => break,
                                    Some(_) => {}
                                }
                            }
                            continue;
                        }
                        '"' => {
                            unimplemented!();
                        }
                        ' ' if self.column == 1 => {
                            let mut offset = self.data.len();
                            for (off1, ch1) in iter {
                                if ch1 != ' ' {
                                    offset = self.offset + off1;
                                }
                            }
                            let indent = offset - self.offset;  // SP is 1 byte
                            let mut nindents = self.indents;
                            if indent > nindents[nindents.len()-1] {
                                nindents.push(indent);
                                return Ok(((TokenType::Indent,
                                    &self.data[self.offset..offset],
                                    pos),
                                      Tokenizer {
                                        offset: offset,
                                        column: self.column + indent as i32,
                                        indents: nindents,
                                        .. self
                                    }));
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
                                return Ok(((TokenType::Dedent,
                                    &self.data[self.offset..offset],
                                    pos),
                                      Tokenizer {
                                        offset: offset,
                                        column: self.column + indent as i32,
                                        indents: nindents,
                                        .. self
                                    }));
                            }
                        }
                        ' '|'\t' => {  // Skip whitespace
                            self.column += 1;
                            continue;
                        }
                        'a'...'z'|'A'...'Z' => {
                            let mut offset = self.data.len();
                            for (off1, ch1) in iter {
                                match ch1 {
                                    'a'...'z'|'A'...'Z'|'0'...'9' => continue,
                                    _ => {
                                        offset = self.offset + off1;
                                        break;
                                    }
                                }
                                unreachable!();
                            }
                            let value = &self.data[self.offset..offset];
                            let tok = match value {
                                "css" => TokenType::Css,
                                "html" => TokenType::Html,
                                _ => TokenType::Ident,
                            };
                            return Ok(((tok, value, pos),
                                  Tokenizer {
                                    offset: offset,
                                    column: self.column + value.len() as i32,
                                    .. self
                                }));
                        }
                        _ => {
                            return Err(()); // Unexpected character
                        }
                    }
                }
                None => {
                    match self.indents.pop() {
                        Some(level) => {
                            return Ok(((TokenType::Dedent, "", pos),
                                  Tokenizer {
                                    .. self
                                }));
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
            offset: 0,
            line: 1,
            column: 1,
            braces: vec!(),
            indents: vec!(0),
        };
    }
    pub fn end_of_file(&self) -> bool {
        return self.offset == self.data.len();
    }
    pub fn error_message(&self) -> String {
        return format!("Unexpected character {:?} at: {}:{}",
            &self.data[self.offset..][..1], self.line, self.column);
    }
}
