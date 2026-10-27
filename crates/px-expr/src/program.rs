use alloc::vec::Vec;

use crate::ease::Ease;
use crate::error::EvalError;
use crate::funcs::{Ctx, Func, Scope};
use crate::names::Var;

pub const MAX_STACK: usize = 32;
pub const MAX_SCOPES: usize = 16;
const MAX_ARGS: usize = 9;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Env {
    pub time: f64,
    pub frame: f64,
    pub fps: f64,
    pub duration: f64,
    pub seed: u64,
    pub index: f64,
    pub count: f64,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub loop_period: Option<f64>,
}

impl Default for Env {
    fn default() -> Self {
        Self {
            time: 0.0,
            frame: 0.0,
            fps: 12.0,
            duration: 0.0,
            seed: 0,
            index: 0.0,
            count: 1.0,
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
            loop_period: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Op {
    Const(f64),
    Var(Var),
    Unary(UnaryOp),
    Binary(BinaryOp),
    Select,
    Call(Func, u8),
    Ease(Ease),
    Enter(Scope),
    Leave,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum UnaryOp {
    Neg,
    Not,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Pow,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

fn truth(condition: bool) -> f64 {
    if condition { 1.0 } else { 0.0 }
}

impl BinaryOp {
    #[expect(clippy::float_cmp, reason = "`==` in the language is exact comparison")]
    fn apply(self, a: f64, b: f64) -> f64 {
        match self {
            Self::Add => a + b,
            Self::Sub => a - b,
            Self::Mul => a * b,
            Self::Div => a / b,
            Self::Rem => a % b,
            Self::Pow => libm::pow(a, b),
            Self::Eq => truth(a == b),
            Self::Ne => truth(a != b),
            Self::Lt => truth(a < b),
            Self::Le => truth(a <= b),
            Self::Gt => truth(a > b),
            Self::Ge => truth(a >= b),
            Self::And => truth(a != 0.0 && b != 0.0),
            Self::Or => truth(a != 0.0 || b != 0.0),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Clock {
    time: f64,
    frame: f64,
}

impl Clock {
    fn shifted(self, scope: Scope, value: f64, env: &Env) -> Self {
        let delay = |seconds: f64| Self {
            time: self.time - seconds,
            frame: self.frame - seconds * env.fps,
        };
        match scope {
            Scope::Loop if value > 0.0 => {
                let time = self.time - value * libm::floor(self.time / value);
                Self {
                    time,
                    frame: time * env.fps,
                }
            }
            Scope::Loop => self,
            Scope::Delay => delay(value),
            Scope::Stagger => delay(env.index * value),
            Scope::Hold => {
                let step = libm::round(value).max(1.0);
                let frame = libm::floor(self.frame / step) * step;
                Self {
                    time: frame / env.fps,
                    frame,
                }
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Program {
    ops: Vec<Op>,
}

impl Program {
    pub(crate) fn new(ops: Vec<Op>) -> Self {
        Self { ops }
    }

    #[must_use]
    pub fn constant(&self) -> Option<f64> {
        match self.ops.as_slice() {
            [Op::Const(value)] => Some(*value),
            _ => None,
        }
    }

    #[must_use]
    pub fn uses(&self, var: Var) -> bool {
        self.ops.contains(&Op::Var(var))
    }

    pub fn eval(&self, env: &Env) -> Result<f64, EvalError> {
        let mut stack = Stack::<MAX_STACK>::new();
        let mut clocks = Stack::<MAX_SCOPES>::new();
        let mut clock = Clock {
            time: env.time,
            frame: env.frame,
        };
        let mut saved = [clock; MAX_SCOPES];
        for op in &self.ops {
            match *op {
                Op::Const(value) => stack.push(value)?,
                Op::Var(var) => stack.push(variable(var, clock, env))?,
                Op::Unary(UnaryOp::Neg) => {
                    let value = stack.pop()?;
                    stack.push(-value)?;
                }
                Op::Unary(UnaryOp::Not) => {
                    let value = stack.pop()?;
                    stack.push(truth(value == 0.0))?;
                }
                Op::Binary(op) => {
                    let b = stack.pop()?;
                    let a = stack.pop()?;
                    stack.push(op.apply(a, b))?;
                }
                Op::Select => {
                    let otherwise = stack.pop()?;
                    let then = stack.pop()?;
                    let condition = stack.pop()?;
                    stack.push(if condition == 0.0 { otherwise } else { then })?;
                }
                Op::Call(func, arg_count) => {
                    let mut buffer = [0.0; MAX_ARGS];
                    let args = stack.pop_into(usize::from(arg_count), &mut buffer)?;
                    let ctx = Ctx {
                        time: clock.time,
                        seed: env.seed,
                        loop_period: env.loop_period,
                    };
                    stack.push(func.apply(args, &ctx).ok_or(EvalError::Malformed)?)?;
                }
                Op::Ease(ease) => {
                    let k = stack.pop()?;
                    stack.push(ease.apply(k))?;
                }
                Op::Enter(scope) => {
                    let value = stack.pop()?;
                    *saved.get_mut(clocks.len).ok_or(EvalError::Malformed)? = clock;
                    clocks.push(0.0)?;
                    clock = clock.shifted(scope, value, env);
                }
                Op::Leave => {
                    clocks.pop()?;
                    clock = saved[clocks.len];
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

fn variable(var: Var, clock: Clock, env: &Env) -> f64 {
    match var {
        Var::Time => clock.time,
        Var::Frame => clock.frame,
        Var::Fps => env.fps,
        Var::Duration => env.duration,
        Var::Seed => f64::from(u32::try_from(env.seed & u64::from(u32::MAX)).unwrap_or(0)),
        Var::Index => env.index,
        Var::Count => env.count,
        Var::Position => env.index / (env.count - 1.0).max(1.0),
        Var::X => env.x,
        Var::Y => env.y,
        Var::Width => env.width,
        Var::Height => env.height,
    }
}

pub(crate) fn required_stack(ops: &[Op]) -> Option<(usize, usize)> {
    let (mut depth, mut max_depth, mut scopes, mut max_scopes) = (0usize, 0usize, 0usize, 0usize);
    for op in ops {
        let (pops, pushes) = match op {
            Op::Const(_) | Op::Var(_) => (0, 1),
            Op::Unary(_) | Op::Ease(_) => (1, 1),
            Op::Binary(_) => (2, 1),
            Op::Select => (3, 1),
            Op::Call(_, argc) => (usize::from(*argc), 1),
            Op::Enter(_) => {
                scopes += 1;
                max_scopes = max_scopes.max(scopes);
                (1, 0)
            }
            Op::Leave => {
                scopes = scopes.checked_sub(1)?;
                (0, 0)
            }
        };
        depth = depth.checked_sub(pops)? + pushes;
        max_depth = max_depth.max(depth);
    }
    (depth == 1 && scopes == 0).then_some((max_depth, max_scopes))
}

struct Stack<const N: usize> {
    values: [f64; N],
    len: usize,
}

impl<const N: usize> Stack<N> {
    const fn new() -> Self {
        Self {
            values: [0.0; N],
            len: 0,
        }
    }

    fn push(&mut self, value: f64) -> Result<(), EvalError> {
        *self.values.get_mut(self.len).ok_or(EvalError::Malformed)? = value;
        self.len += 1;
        Ok(())
    }

    fn pop(&mut self) -> Result<f64, EvalError> {
        self.len = self.len.checked_sub(1).ok_or(EvalError::Malformed)?;
        Ok(self.values[self.len])
    }

    fn pop_into<'a>(&mut self, count: usize, dest: &'a mut [f64]) -> Result<&'a [f64], EvalError> {
        let start = self.len.checked_sub(count).ok_or(EvalError::Malformed)?;
        let dest = dest.get_mut(..count).ok_or(EvalError::Malformed)?;
        dest.copy_from_slice(&self.values[start..self.len]);
        self.len = start;
        Ok(dest)
    }

    fn finish(&self) -> Result<f64, EvalError> {
        match self.values.get(..self.len) {
            Some([value]) => Ok(*value),
            _ => Err(EvalError::Malformed),
        }
    }
}
