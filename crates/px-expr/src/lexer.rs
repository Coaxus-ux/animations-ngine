use crate::error::{CompileError, CompileErrorKind};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Unit {
    None,
    Seconds,
    Millis,
    Frames,
    Degrees,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum TokenKind {
    Number(f64, Unit),
    Str,
    Ident,
    Param,
    Ref,
    Plus,
    Minus,
    Star,
    StarStar,
    Slash,
    Percent,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    AndAnd,
    OrOr,
    Bang,
    Question,
    Colon,
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
        let range = match token.kind {
            TokenKind::Str => token.start + 1..token.end - 1,
            TokenKind::Param | TokenKind::Ref => token.start + 1..token.end,
            _ => token.start..token.end,
        };
        self.source.get(range).unwrap_or("")
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
        let next = bytes.get(start + 1).copied();
        let (kind, len) = match (byte, next) {
            (b'*', Some(b'*')) => (TokenKind::StarStar, 2),
            (b'=', Some(b'=')) => (TokenKind::Eq, 2),
            (b'!', Some(b'=')) => (TokenKind::Ne, 2),
            (b'<', Some(b'=')) => (TokenKind::Le, 2),
            (b'>', Some(b'=')) => (TokenKind::Ge, 2),
            (b'&', Some(b'&')) => (TokenKind::AndAnd, 2),
            (b'|', Some(b'|')) => (TokenKind::OrOr, 2),
            (b'+', _) => (TokenKind::Plus, 1),
            (b'-', _) => (TokenKind::Minus, 1),
            (b'*', _) => (TokenKind::Star, 1),
            (b'/', _) => (TokenKind::Slash, 1),
            (b'%', _) => (TokenKind::Percent, 1),
            (b'<', _) => (TokenKind::Lt, 1),
            (b'>', _) => (TokenKind::Gt, 1),
            (b'!', _) => (TokenKind::Bang, 1),
            (b'?', _) => (TokenKind::Question, 1),
            (b':', _) => (TokenKind::Colon, 1),
            (b'(', _) => (TokenKind::LParen, 1),
            (b')', _) => (TokenKind::RParen, 1),
            (b',', _) => (TokenKind::Comma, 1),
            (b'0'..=b'9' | b'.', _) => return self.number(start),
            (b'\'', _) => return self.string(start),
            (b'a'..=b'z' | b'_', _) => return Ok(self.ident(start)),
            (b'$', Some(b'a'..=b'z')) => return Ok(self.sigil(start, TokenKind::Param, false)),
            (b'@', Some(b'a'..=b'z')) => return Ok(self.sigil(start, TokenKind::Ref, true)),
            _ => return Err(error(CompileErrorKind::UnexpectedChar, start, start + 1)),
        };
        self.pos += len;
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
        let digits_end = self.pos;
        while bytes.get(self.pos).is_some_and(u8::is_ascii_lowercase) {
            self.pos += 1;
        }
        let invalid = || error(CompileErrorKind::InvalidNumber, start, self.pos);
        let digits = self.source.get(start..digits_end).ok_or_else(invalid)?;
        let value: f64 = digits.parse().map_err(|_| invalid())?;
        let unit = match self.source.get(digits_end..self.pos).unwrap_or("") {
            "" => Unit::None,
            "s" => Unit::Seconds,
            "ms" => Unit::Millis,
            "f" => Unit::Frames,
            "deg" => Unit::Degrees,
            _ => return Err(error(CompileErrorKind::InvalidUnit, digits_end, self.pos)),
        };
        Ok(Token {
            kind: TokenKind::Number(value, unit),
            start,
            end: self.pos,
        })
    }

    fn string(&mut self, start: usize) -> Result<Token, CompileError> {
        let bytes = self.source.as_bytes();
        self.pos += 1;
        while bytes.get(self.pos).is_some_and(|b| *b != b'\'') {
            self.pos += 1;
        }
        if self.pos >= bytes.len() {
            return Err(error(CompileErrorKind::UnterminatedString, start, self.pos));
        }
        self.pos += 1;
        Ok(Token {
            kind: TokenKind::Str,
            start,
            end: self.pos,
        })
    }

    fn sigil(&mut self, start: usize, kind: TokenKind, dotted: bool) -> Token {
        let bytes = self.source.as_bytes();
        self.pos += 1;
        loop {
            self.ident(self.pos);
            let continues = dotted
                && bytes.get(self.pos) == Some(&b'.')
                && bytes.get(self.pos + 1).is_some_and(u8::is_ascii_lowercase);
            if !continues {
                break;
            }
            self.pos += 1;
        }
        Token {
            kind,
            start,
            end: self.pos,
        }
    }

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

pub(crate) fn time_literal(text: &str) -> Option<(f64, Unit)> {
    let mut lexer = Lexer::new(text);
    let token = lexer.next_token().ok()?;
    let end = lexer.next_token().ok()?;
    match (token.kind, end.kind) {
        (TokenKind::Number(value, unit), TokenKind::End)
            if matches!(unit, Unit::Seconds | Unit::Millis | Unit::Frames) =>
        {
            Some((value, unit))
        }
        _ => None,
    }
}
