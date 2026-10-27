use alloc::vec::Vec;

use crate::error::{CompileError, CompileErrorKind};
use crate::lexer::{Lexer, Token, TokenKind, error};
use crate::names::{Func, Name, lookup};
use crate::program::{BinaryOp, MAX_STACK, Op, Program};

/// Maximum nesting of parentheses, unary minus and calls. Bounds parser recursion.
pub const MAX_NESTING: usize = 32;

/// Recursive-descent parser that emits postfix ops directly.
pub(crate) struct Parser<'a> {
    lexer: Lexer<'a>,
    current: Token,
    ops: Vec<Op>,
    stack_depth: usize,
    max_stack_depth: usize,
    nesting: usize,
}

impl<'a> Parser<'a> {
    pub(crate) fn new(source: &'a str) -> Self {
        let end = Token {
            kind: TokenKind::End,
            start: 0,
            end: 0,
        };
        Self {
            lexer: Lexer::new(source),
            current: end,
            ops: Vec::new(),
            stack_depth: 0,
            max_stack_depth: 0,
            nesting: 0,
        }
    }

    pub(crate) fn parse(mut self) -> Result<Program, CompileError> {
        self.advance()?;
        if self.current.kind == TokenKind::End {
            return Err(error(CompileErrorKind::Empty, 0, 0));
        }
        self.expression()?;
        if self.current.kind != TokenKind::End {
            return Err(self.unexpected());
        }
        if self.max_stack_depth > MAX_STACK {
            return Err(error(CompileErrorKind::TooComplex, 0, self.current.end));
        }
        debug_assert_eq!(
            self.stack_depth, 1,
            "a complete expression leaves one value"
        );
        Ok(Program::new(self.ops))
    }

    fn advance(&mut self) -> Result<Token, CompileError> {
        let previous = self.current;
        self.current = self.lexer.next_token()?;
        Ok(previous)
    }

    fn unexpected(&self) -> CompileError {
        error(
            CompileErrorKind::UnexpectedToken,
            self.current.start,
            self.current.end,
        )
    }

    fn expect(&mut self, kind: TokenKind) -> Result<(), CompileError> {
        if self.current.kind == kind {
            self.advance()?;
            Ok(())
        } else {
            Err(self.unexpected())
        }
    }

    fn emit(&mut self, op: Op) {
        match op {
            Op::Const(_) | Op::Var(_) => self.stack_depth += 1,
            Op::Neg => {}
            Op::Binary(_) => self.stack_depth = self.stack_depth.saturating_sub(1),
            Op::Call(func) => {
                let consumed = usize::from(func.arity()).saturating_sub(1);
                self.stack_depth = self.stack_depth.saturating_sub(consumed);
            }
        }
        self.max_stack_depth = self.max_stack_depth.max(self.stack_depth);
        self.ops.push(op);
    }

    fn nested(
        &mut self,
        start: usize,
        parse: impl FnOnce(&mut Self) -> Result<(), CompileError>,
    ) -> Result<(), CompileError> {
        if self.nesting >= MAX_NESTING {
            return Err(error(CompileErrorKind::TooDeep, start, self.current.end));
        }
        self.nesting += 1;
        let result = parse(self);
        self.nesting -= 1;
        result
    }

    fn expression(&mut self) -> Result<(), CompileError> {
        self.term()?;
        loop {
            let op = match self.current.kind {
                TokenKind::Plus => BinaryOp::Add,
                TokenKind::Minus => BinaryOp::Sub,
                _ => return Ok(()),
            };
            self.advance()?;
            self.term()?;
            self.emit(Op::Binary(op));
        }
    }

    fn term(&mut self) -> Result<(), CompileError> {
        self.unary()?;
        loop {
            let op = match self.current.kind {
                TokenKind::Star => BinaryOp::Mul,
                TokenKind::Slash => BinaryOp::Div,
                TokenKind::Percent => BinaryOp::Rem,
                _ => return Ok(()),
            };
            self.advance()?;
            self.unary()?;
            self.emit(Op::Binary(op));
        }
    }

    fn unary(&mut self) -> Result<(), CompileError> {
        if self.current.kind != TokenKind::Minus {
            return self.primary();
        }
        let start = self.advance()?.start;
        self.nested(start, |parser| {
            parser.unary()?;
            parser.emit(Op::Neg);
            Ok(())
        })
    }

    fn primary(&mut self) -> Result<(), CompileError> {
        let token = self.current;
        match token.kind {
            TokenKind::Number(value) => {
                self.advance()?;
                self.emit(Op::Const(value));
                Ok(())
            }
            TokenKind::Ident => self.name(token),
            TokenKind::LParen => {
                self.advance()?;
                self.nested(token.start, |parser| {
                    parser.expression()?;
                    parser.expect(TokenKind::RParen)
                })
            }
            _ => Err(self.unexpected()),
        }
    }

    fn name(&mut self, token: Token) -> Result<(), CompileError> {
        let name = lookup(self.lexer.text(token))
            .ok_or_else(|| error(CompileErrorKind::UnknownName, token.start, token.end))?;
        self.advance()?;
        let is_call = self.current.kind == TokenKind::LParen;
        match name {
            Name::Func(func) if is_call => self.call(func, token),
            Name::Func(_) => Err(error(CompileErrorKind::MissingCall, token.start, token.end)),
            _ if is_call => Err(error(
                CompileErrorKind::NotAFunction,
                token.start,
                token.end,
            )),
            Name::Var(var) => {
                self.emit(Op::Var(var));
                Ok(())
            }
            Name::Const(value) => {
                self.emit(Op::Const(value));
                Ok(())
            }
        }
    }

    fn call(&mut self, func: Func, name: Token) -> Result<(), CompileError> {
        self.advance()?;
        self.nested(name.start, |parser| {
            let mut found: u8 = 0;
            if parser.current.kind != TokenKind::RParen {
                loop {
                    parser.expression()?;
                    found = found.saturating_add(1);
                    if parser.current.kind != TokenKind::Comma {
                        break;
                    }
                    parser.advance()?;
                }
            }
            let close = parser.current.end;
            parser.expect(TokenKind::RParen)?;
            let expected = func.arity();
            if found != expected {
                let kind = CompileErrorKind::WrongArity { expected, found };
                return Err(error(kind, name.start, close));
            }
            parser.emit(Op::Call(func));
            Ok(())
        })
    }
}
