use std::str::{Chars};
use std::iter::Peekable;

use combine::primitives::{SourcePosition, Stream, Error, Info};
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

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Mode {
    Normal,
    Css
}

#[derive(Clone)]
pub struct Tokenizer<'a> {
    data: &'a str,
    iter: CodeIter<'a>,
    braces: Vec<char>,
    indents: Vec<usize>,
    mode: Mode,
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
            mode: Mode::Normal,
        };
    }

    fn next(&mut self)
        -> Result<(TokenType, &'a str, SourcePosition), Error<Token<'a>>>
    {
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
                                        // TODO(tailhook) how to report
                                        //                position?
                                        return Err(Error::Message(
                                            Info::Borrowed(
                                                "Wrong indentation level")));
                                    } else if nind == indent {
                                        self.iter = niter;  // commit if last
                                    }
                                    typ = TokenType::Dedent;
                                }
                                let pos = SourcePosition {
                                    line: line,
                                    column: col,
                                    };
                                return Ok((typ, "", pos));
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
                    return Ok((TokenType::Dedent, "", pos));
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
                                return Ok((TokenType::Newline, "\n", pos));
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
                            return Ok((typ, &self.data[off..off+1], pos));
                        }
                        ')'|'}'|']' => {
                            let (typ, val) = match ch {
                                ')' => (TokenType::CloseParen, '('),
                                '}' => (TokenType::CloseBrace, '{'),
                                ']' => (TokenType::CloseBracket, '['),
                                _ => unreachable!(),
                            };
                            let br = self.braces.pop();
                            let tok = (typ, &self.data[off..off+1], pos);
                            if Some(val) == br {
                                return Ok(tok);
                            } else {
                                return Err(Error::Unexpected(
                                    Token(tok.0, tok.1, tok.2)));
                            }
                        }
                        'a'...'z'|'A'...'Z'|'_'|'-'|'0'...'9'
                        if self.mode == Mode::Css => {
                            let mut offset = self.data.len();
                            loop {
                                match self.iter.peek() {
                                    Some((x, off, _, _)) => {
                                        match x {
                                            'a'...'z'|'A'...'Z'
                                            |'0'...'9'|'_'|'-'|'.'
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
                            match value {
                                "html" if column == 1 => {
                                    self.mode = Mode::Normal;
                                    return Ok((TokenType::Html, value, pos));
                                }
                                _ => {
                                    return Ok((TokenType::CssWord,
                                                 value, pos));
                                }
                            }
                        }
                        ':'|'.'|'='|','|'-'|'+'|'*'|'/'|'?'|'>'|'<'|'!' => {
                            let mut len = 1;
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
                                            len = 2;
                                            TokenType::Eq
                                        }
                                        _ => TokenType::Equals,
                                    }
                                }
                                '!' => {
                                    match self.iter.peek() {
                                        Some(('=', _, _, _)) => {
                                            self.iter.next();
                                            len = 2;
                                            TokenType::NotEq
                                        }
                                        _ => TokenType::Not, //TODO remove?
                                    }
                                }
                                '>' => {
                                    match self.iter.peek() {
                                        Some(('=', _, _, _)) => {
                                            self.iter.next();
                                            len = 2;
                                            TokenType::GreaterEq
                                        }
                                        _ => TokenType::Greater,
                                    }
                                }
                                '<' => {
                                    match self.iter.peek() {
                                        Some(('=', _, _, _)) => {
                                            self.iter.next();
                                            len = 2;
                                            TokenType::LessEq
                                        }
                                        _ => TokenType::Less,
                                    }
                                }
                                '-' => {
                                    match self.iter.peek() {
                                        Some(('>', _, _, _)) => {
                                            self.iter.next();
                                            len = 2;
                                            TokenType::ArrowRight
                                        }
                                        _ => TokenType::Dash,
                                    }
                                }
                                ',' => TokenType::Comma,
                                _ => unreachable!(),
                            };
                            return Ok((typ,
                                &self.data[off..off+len],
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
                                return Ok((TokenType::Newline, "\n", pos));
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
                            return Ok((TokenType::String, value, pos));
                        }
                        ' '|'\t' => {  // Skip whitespace
                            continue;
                        }
                        'a'...'z'|'A'...'Z'|'_'
                        if self.mode == Mode::Normal => {
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
                                "css" => {
                                    if column == 1 {
                                        self.mode = Mode::Css;
                                    }
                                    TokenType::Css
                                }
                                "html" => TokenType::Html,
                                "import" => TokenType::Import,
                                "from" => TokenType::From,
                                "if" => TokenType::If,
                                "elif" => TokenType::Elif,
                                "for" => TokenType::For,
                                "in" => TokenType::In,
                                "of" => TokenType::Of,
                                "as" => TokenType::As,
                                "key" => TokenType::Key,
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
                            return Ok((tok, value, pos));
                        }
                        '0'...'9' => {
                            let mut offset = self.data.len();
                            loop {
                                match self.iter.peek() {
                                    Some((x, off, _, _)) => {
                                        match x {
                                            '0'...'9'|'_'|'.' => {}
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
                            return Ok((TokenType::Number, value, pos));
                        }
                        _ => {
                            return Err(Error::Message(Info::Owned(
                                format!("unexpected character {:?} at {}",
                                &self.data[off..off+1], pos))));
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
                            return Ok((TokenType::Dedent, "", pos));
                        }
                        _ => {
                            return Ok((TokenType::Eof, "", pos));
                        }
                    }
                }
            }
        }
    }
}

impl<'a> Stream for Tokenizer<'a> {
    type Item = Token<'a>;
    fn uncons(mut self) -> Result<(Token<'a>, Tokenizer<'a>), Error<Token<'a>>>
    {
        match self.next() {
            Ok((t, v, p)) => Ok((Token(t, v, p), self)),
            Err(e) => Err(e),
        }
    }
}
