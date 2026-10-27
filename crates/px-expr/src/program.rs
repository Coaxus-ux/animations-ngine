use alloc::vec::Vec;

use crate::error::EvalError;
use crate::names::{Func, Var};

pub const MAX_STACK: usize = 32;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Env {
    pub time: f64,
    pub frame: f64,
    pub fps: f64,
    pub duration: f64,
}

impl Env {
    const fn get(&self, var: Var) -> f64 {
        match var {
            Var::Time => self.time,
            Var::Frame => self.frame,
            Var::Fps => self.fps,
            Var::Duration => self.duration,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Op {
    Const(f64),
    Var(Var),
    Neg,
    Binary(BinaryOp),
    Call(Func),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
}

impl BinaryOp {
    const fn apply(self, a: f64, b: f64) -> f64 {
        match self {
            Self::Add => a + b,
            Self::Sub => a - b,
            Self::Mul => a * b,
            Self::Div => a / b,
            Self::Rem => a % b,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Program {
    ops: Vec<Op>,
    constant: Option<f64>,
}

impl Program {
    pub(crate) fn new(ops: Vec<Op>) -> Self {
        let constant = match ops.as_slice() {
            [Op::Const(value)] => Some(*value),
            _ => None,
        };
        Self { ops, constant }
    }

    #[must_use]
    pub const fn constant(&self) -> Option<f64> {
        self.constant
    }

    #[must_use]
    pub fn uses(&self, var: Var) -> bool {
        self.ops.contains(&Op::Var(var))
    }

    pub fn eval(&self, env: &Env) -> Result<f64, EvalError> {
        let mut stack = Stack::new();
        for op in &self.ops {
            match *op {
                Op::Const(value) => stack.push(value)?,
                Op::Var(var) => stack.push(env.get(var))?,
                Op::Neg => {
                    let value = stack.pop()?;
                    stack.push(-value)?;
                }
                Op::Binary(op) => {
                    let b = stack.pop()?;
                    let a = stack.pop()?;
                    stack.push(op.apply(a, b))?;
                }
                Op::Call(func) => {
                    let args = stack.pop_n(usize::from(func.arity()))?;
                    let value = func
                        .apply(&args.values[..args.len])
                        .ok_or(EvalError::Malformed)?;
                    stack.push(value)?;
                }
            }
        }
        let result = stack.finish()?;
        if result.is_finite() {
            Ok(result)
        } else {
            Err(EvalError::NonFinite)
        }
    }
}

struct Args {
    values: [f64; 2],
    len: usize,
}

struct Stack {
    values: [f64; MAX_STACK],
    len: usize,
}

impl Stack {
    const fn new() -> Self {
        Self {
            values: [0.0; MAX_STACK],
            len: 0,
        }
    }

    fn push(&mut self, value: f64) -> Result<(), EvalError> {
        let slot = self.values.get_mut(self.len).ok_or(EvalError::Malformed)?;
        *slot = value;
        self.len += 1;
        Ok(())
    }

    fn pop(&mut self) -> Result<f64, EvalError> {
        self.len = self.len.checked_sub(1).ok_or(EvalError::Malformed)?;
        Ok(self.values[self.len])
    }

    fn pop_n(&mut self, count: usize) -> Result<Args, EvalError> {
        let mut args = Args {
            values: [0.0; 2],
            len: count,
        };
        let dest = args.values.get_mut(..count).ok_or(EvalError::Malformed)?;
        let start = self.len.checked_sub(count).ok_or(EvalError::Malformed)?;
        dest.copy_from_slice(&self.values[start..self.len]);
        self.len = start;
        Ok(args)
    }

    fn finish(&self) -> Result<f64, EvalError> {
        match self.values.get(..self.len) {
            Some([value]) => Ok(*value),
            _ => Err(EvalError::Malformed),
        }
    }
}
