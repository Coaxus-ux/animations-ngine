use alloc::vec::Vec;

use crate::ease::Ease;
use crate::error::{CompileError, CompileErrorKind};
use crate::funcs::Func;
use crate::lexer::{Lexer, Token, TokenKind, Unit, error, time_literal};
use crate::names::{Name, Var, lookup};
use crate::program::{BinaryOp, MAX_SCOPES, MAX_STACK, Op, Program, UnaryOp, required_stack};

pub const MAX_NESTING: usize = 32;

pub(crate) struct Parser<'a> {
    lexer: Lexer<'a>,
    current: Token,
    ops: Vec<Op>,
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
            nesting: 0,
        }
    }

    pub(crate) fn parse(mut self) -> Result<Program, CompileError> {
        self.advance()?;
        if self.current.kind == TokenKind::End {
            return Err(error(CompileErrorKind::Empty, 0, 0));
        }
        self.ternary()?;
        if self.current.kind != TokenKind::End {
            return Err(self.unexpected());
        }
        let too_complex = error(CompileErrorKind::TooComplex, 0, self.current.end);
        let (stack, scopes) = required_stack(&self.ops).ok_or(too_complex)?;
        if stack > MAX_STACK || scopes > MAX_SCOPES {
            return Err(too_complex);
        }
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

    fn expect(&mut self, kind: TokenKind) -> Result<Token, CompileError> {
        if self.current.kind == kind {
            self.advance()
        } else {
            Err(self.unexpected())
        }
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

    fn ternary(&mut self) -> Result<(), CompileError> {
        self.binary_level(0)?;
        if self.current.kind != TokenKind::Question {
            return Ok(());
        }
        let start = self.advance()?.start;
        self.nested(start, |parser| {
            parser.ternary()?;
            parser.expect(TokenKind::Colon)?;
            parser.ternary()?;
            parser.ops.push(Op::Select);
            Ok(())
        })
    }

    fn binary_level(&mut self, level: usize) -> Result<(), CompileError> {
        let Some(operators) = LEVELS.get(level) else {
            return self.power();
        };
        self.binary_level(level + 1)?;
        while let Some(op) = operators
            .iter()
            .find(|(kind, _)| *kind == self.current.kind)
            .map(|(_, op)| *op)
        {
            self.advance()?;
            self.binary_level(level + 1)?;
            self.ops.push(Op::Binary(op));
        }
        Ok(())
    }

    fn power(&mut self) -> Result<(), CompileError> {
        self.unary()?;
        if self.current.kind != TokenKind::StarStar {
            return Ok(());
        }
        let start = self.advance()?.start;
        self.nested(start, |parser| {
            parser.power()?;
            parser.ops.push(Op::Binary(BinaryOp::Pow));
            Ok(())
        })
    }

    fn unary(&mut self) -> Result<(), CompileError> {
        let op = match self.current.kind {
            TokenKind::Minus => UnaryOp::Neg,
            TokenKind::Bang => UnaryOp::Not,
            _ => return self.primary(),
        };
        let start = self.advance()?.start;
        self.nested(start, |parser| {
            parser.unary()?;
            parser.ops.push(Op::Unary(op));
            Ok(())
        })
    }

    fn primary(&mut self) -> Result<(), CompileError> {
        let token = self.current;
        match token.kind {
            TokenKind::Number(value, unit) => {
                self.advance()?;
                self.number(value, unit);
                Ok(())
            }
            TokenKind::Str => {
                let (value, unit) = time_literal(self.lexer.text(token)).ok_or_else(|| {
                    error(CompileErrorKind::StringNotAllowed, token.start, token.end)
                })?;
                self.advance()?;
                self.number(value, unit);
                Ok(())
            }
            TokenKind::Ident => self.name(token),
            TokenKind::LParen => {
                self.advance()?;
                self.nested(token.start, |parser| {
                    parser.ternary()?;
                    parser.expect(TokenKind::RParen).map(|_| ())
                })
            }
            _ => Err(self.unexpected()),
        }
    }

    fn number(&mut self, value: f64, unit: Unit) {
        match unit {
            Unit::None | Unit::Seconds | Unit::Degrees => self.ops.push(Op::Const(value)),
            Unit::Millis => self.ops.push(Op::Const(value / 1000.0)),
            Unit::Frames => self.ops.extend([
                Op::Const(value),
                Op::Var(Var::Fps),
                Op::Binary(BinaryOp::Div),
            ]),
        }
    }

    fn name(&mut self, token: Token) -> Result<(), CompileError> {
        let name = lookup(self.lexer.text(token))
            .ok_or_else(|| error(CompileErrorKind::UnknownName, token.start, token.end))?;
        self.advance()?;
        let is_call = self.current.kind == TokenKind::LParen;
        match name {
            Name::Func(func) if is_call => {
                self.nested(token.start, |parser| parser.call(func, token))
            }
            Name::Func(_) => Err(error(CompileErrorKind::MissingCall, token.start, token.end)),
            _ if is_call => Err(error(
                CompileErrorKind::NotAFunction,
                token.start,
                token.end,
            )),
            Name::Var(var) => {
                self.ops.push(Op::Var(var));
                Ok(())
            }
            Name::Const(value) => {
                self.ops.push(Op::Const(value));
                Ok(())
            }
        }
    }

    fn call(&mut self, func: Func, name: Token) -> Result<(), CompileError> {
        self.advance()?;
        let ease = if func == Func::Ease {
            Some(self.ease_name()?)
        } else {
            None
        };
        let first_arg = self.ops.len();
        let mut second_arg = None;
        let mut found = u8::from(ease.is_some());
        if self.current.kind != TokenKind::RParen {
            loop {
                if found == 1 && ease.is_none() {
                    second_arg = Some(self.ops.len());
                }
                self.ternary()?;
                found = found.saturating_add(1);
                if self.current.kind != TokenKind::Comma {
                    break;
                }
                self.advance()?;
            }
        }
        let close = self.expect(TokenKind::RParen)?.end;
        let (min, max) = func.arity();
        if found < min || found > max {
            return Err(error(
                CompileErrorKind::WrongArity { min, max, found },
                name.start,
                close,
            ));
        }
        if let Some(ease) = ease {
            self.ops.push(Op::Ease(ease));
        } else if let Some((scope, fixed)) = func.scope() {
            self.wrap_in_scope(first_arg, second_arg, scope, fixed);
        } else {
            let defaults = func.defaults();
            let missing = usize::from(max - found).min(defaults.len());
            self.ops.extend(
                defaults[defaults.len() - missing..]
                    .iter()
                    .map(|value| Op::Const(*value)),
            );
            let argc = found + u8::try_from(missing).unwrap_or(0);
            self.ops.push(Op::Call(func, argc));
        }
        Ok(())
    }

    fn ease_name(&mut self) -> Result<Ease, CompileError> {
        let token = self.expect(TokenKind::Str)?;
        let ease = Ease::parse(self.lexer.text(token))
            .ok_or_else(|| error(CompileErrorKind::UnknownEase, token.start, token.end))?;
        self.expect(TokenKind::Comma)?;
        Ok(ease)
    }

    // `loop(expr, period)` must evaluate `period` before entering the scope that `expr` runs in.
    fn wrap_in_scope(
        &mut self,
        first_arg: usize,
        second_arg: Option<usize>,
        scope: crate::funcs::Scope,
        fixed: Option<f64>,
    ) {
        let parameter = second_arg.map(|start| self.ops.split_off(start));
        let body = self.ops.split_off(first_arg);
        match (fixed, parameter) {
            (Some(value), _) => self.ops.push(Op::Const(value)),
            (None, Some(parameter)) => self.ops.extend(parameter),
            (None, None) => self.ops.push(Op::Const(0.0)),
        }
        self.ops.push(Op::Enter(scope));
        self.ops.extend(body);
        self.ops.push(Op::Leave);
    }
}

const LEVELS: [&[(TokenKind, BinaryOp)]; 5] = [
    &[(TokenKind::OrOr, BinaryOp::Or)],
    &[(TokenKind::AndAnd, BinaryOp::And)],
    &[
        (TokenKind::Eq, BinaryOp::Eq),
        (TokenKind::Ne, BinaryOp::Ne),
        (TokenKind::Lt, BinaryOp::Lt),
        (TokenKind::Le, BinaryOp::Le),
        (TokenKind::Gt, BinaryOp::Gt),
        (TokenKind::Ge, BinaryOp::Ge),
    ],
    &[
        (TokenKind::Plus, BinaryOp::Add),
        (TokenKind::Minus, BinaryOp::Sub),
    ],
    &[
        (TokenKind::Star, BinaryOp::Mul),
        (TokenKind::Slash, BinaryOp::Div),
        (TokenKind::Percent, BinaryOp::Rem),
    ],
];
