use crate::error::{CompileError, CompileErrorKind};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum TokenKind {
    Number(f64),
    Ident,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    LParen,
    RParen,
    Comma,
    End,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Token {
    pub(crate) kind: TokenKind,
    pub(crate) start: usize,
    pub(crate) end: usize,
}

pub(crate) struct Lexer<'a> {
    source: &'a str,
    pos: usize,
}

impl<'a> Lexer<'a> {
    pub(crate) const fn new(source: &'a str) -> Self {
        Self { source, pos: 0 }
    }

    pub(crate) fn text(&self, token: Token) -> &'a str {
        self.source.get(token.start..token.end).unwrap_or("")
    }

    pub(crate) fn next_token(&mut self) -> Result<Token, CompileError> {
        let bytes = self.source.as_bytes();
        while bytes.get(self.pos).is_some_and(u8::is_ascii_whitespace) {
            self.pos += 1;
        }
        let start = self.pos;
        let Some(&byte) = bytes.get(start) else {
            return Ok(Token {
                kind: TokenKind::End,
                start,
                end: start,
            });
        };
        let kind = match byte {
            b'+' => TokenKind::Plus,
            b'-' => TokenKind::Minus,
            b'*' => TokenKind::Star,
            b'/' => TokenKind::Slash,
            b'%' => TokenKind::Percent,
            b'(' => TokenKind::LParen,
            b')' => TokenKind::RParen,
            b',' => TokenKind::Comma,
            b'0'..=b'9' | b'.' => return self.number(start),
            b'a'..=b'z' | b'_' => return Ok(self.ident(start)),
            _ => return Err(error(CompileErrorKind::UnexpectedChar, start, start + 1)),
        };
        self.pos += 1;
        Ok(Token {
            kind,
            start,
            end: self.pos,
        })
    }

    fn number(&mut self, start: usize) -> Result<Token, CompileError> {
        let bytes = self.source.as_bytes();
        while bytes
            .get(self.pos)
            .is_some_and(|b| b.is_ascii_digit() || *b == b'.')
        {
            self.pos += 1;
        }
        let text = self.source.get(start..self.pos).unwrap_or("");
        let value = text
            .parse::<f64>()
            .map_err(|_| error(CompileErrorKind::InvalidNumber, start, self.pos))?;
        Ok(Token {
            kind: TokenKind::Number(value),
            start,
            end: self.pos,
        })
    }

    /// Identifiers are kebab-case: a hyphen followed by a letter belongs to the name.
    fn ident(&mut self, start: usize) -> Token {
        let bytes = self.source.as_bytes();
        loop {
            match bytes.get(self.pos) {
                Some(b) if b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'_' => {
                    self.pos += 1;
                }
                Some(b'-') if bytes.get(self.pos + 1).is_some_and(u8::is_ascii_lowercase) => {
                    self.pos += 1;
                }
                _ => break,
            }
        }
        Token {
            kind: TokenKind::Ident,
            start,
            end: self.pos,
        }
    }
}

pub(crate) const fn error(kind: CompileErrorKind, start: usize, end: usize) -> CompileError {
    CompileError { kind, start, end }
}
