use alloc::vec::Vec;

use crate::ease::Ease;
use crate::error::EvalError;
use crate::funcs::{Ctx, Func, Scope};
use crate::names::Var;
use crate::stack::Stack;
use crate::track::Track;

pub const MAX_STACK: usize = 32;
pub const MAX_SCOPES: usize = 16;
pub const MAX_OPS: usize = 4096;
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
    pub wind: Wind,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Wind {
    pub strength: f64,
    pub gust: f64,
    pub speed: f64,
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
            wind: Wind::default(),
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
    Seed(u64),
    Track(u16),
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
    seed: u64,
}

impl Clock {
    fn shifted(self, scope: Scope, value: f64, env: &Env) -> Self {
        let delay = |seconds: f64| Self {
            time: self.time - seconds,
            frame: self.frame - seconds * env.fps,
            ..self
        };
        match scope {
            Scope::Loop if value > 0.0 => {
                let time = self.time - value * px_core::floor(self.time / value);
                Self {
                    time,
                    frame: time * env.fps,
                    ..self
                }
            }
            Scope::Loop => self,
            Scope::Delay => delay(value),
            Scope::Stagger => delay(env.index * value),
            Scope::Hold => {
                let step = libm::round(value).max(1.0);
                let frame = px_core::floor(self.frame / step) * step;
                Self {
                    time: frame / env.fps,
                    frame,
                    ..self
                }
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Program {
    ops: Vec<Op>,
    tracks: Vec<Track>,
    timeless: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Blend {
    Replace,
    Add,
    Multiply,
}

impl Program {
    pub(crate) fn new(ops: Vec<Op>, tracks: Vec<Track>) -> Self {
        let timeless = ops.iter().all(|op| match op {
            Op::Var(var) => !matches!(var, Var::Time | Var::Frame),
            Op::Call(func, _) => !func.reads_time(),
            Op::Track(_) => false,
            Op::Const(_)
            | Op::Unary(_)
            | Op::Binary(_)
            | Op::Select
            | Op::Ease(_)
            | Op::Enter(_)
            | Op::Seed(_)
            | Op::Leave => true,
        });
        Self {
            ops,
            tracks,
            timeless,
        }
    }

    /// The same value in every frame for a given environment: no time, frame, keyframes or
    /// time-driven function. Scopes only shift time, so they don't count.
    #[must_use]
    pub const fn is_timeless(&self) -> bool {
        self.timeless
    }

    #[must_use]
    pub fn track(track: Track) -> Self {
        Self::new(alloc::vec![Op::Track(0)], alloc::vec![track])
    }

    /// Combines a base value with a keyframe track (FORMAT §8 `blend`).
    #[must_use]
    pub fn blend(base: &Self, track: &Self, blend: Blend) -> Self {
        let op = match blend {
            Blend::Replace => return track.clone(),
            Blend::Add => BinaryOp::Add,
            Blend::Multiply => BinaryOp::Mul,
        };
        let (mut ops, mut tracks) = (Vec::new(), Vec::new());
        append(&mut ops, &mut tracks, base);
        append(&mut ops, &mut tracks, track);
        ops.push(Op::Binary(op));
        Self::new(ops, tracks)
    }

    #[must_use]
    pub fn fixed(value: f64) -> Self {
        Self::new(alloc::vec![Op::Const(value)], Vec::new())
    }

    /// A value picked once per object seed: uniform in `low..high`, or an inclusive integer.
    #[must_use]
    pub fn random_range(low: f64, high: f64, integer: bool, key: f64) -> Self {
        let func = if integer { Func::Randi } else { Func::Range };
        Self::new(
            alloc::vec![
                Op::Const(low),
                Op::Const(high),
                Op::Const(key),
                Op::Call(func, 3)
            ],
            Vec::new(),
        )
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
        let mut stack = Stack::<f64, MAX_STACK>::new(0.0);
        let mut clock = Clock {
            time: env.time,
            frame: env.frame,
            seed: env.seed,
        };
        let mut saved = Stack::<Clock, MAX_SCOPES>::new(clock);
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
                        seed: clock.seed,
                        loop_period: env.loop_period,
                        wind: env.wind,
                    };
                    stack.push(func.apply(args, &ctx).ok_or(EvalError::Malformed)?)?;
                }
                Op::Ease(ease) => {
                    let k = stack.pop()?;
                    stack.push(ease.apply(k))?;
                }
                Op::Enter(scope) => {
                    let value = stack.pop()?;
                    saved.push(clock)?;
                    clock = clock.shifted(scope, value, env);
                }
                Op::Seed(seed) => {
                    saved.push(clock)?;
                    clock.seed = seed;
                }
                Op::Track(index) => {
                    let track = self
                        .tracks
                        .get(usize::from(index))
                        .ok_or(EvalError::Malformed)?;
                    stack.push(track.sample(clock.time, env.fps))?;
                }
                Op::Leave => clock = saved.pop()?,
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
        Var::Seed => f64::from(u32::try_from(clock.seed & u64::from(u32::MAX)).unwrap_or(0)),
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
            Op::Const(_) | Op::Var(_) | Op::Track(_) => (0, 1),
            Op::Unary(_) | Op::Ease(_) => (1, 1),
            Op::Binary(_) => (2, 1),
            Op::Select => (3, 1),
            Op::Call(_, argc) => (usize::from(*argc), 1),
            Op::Enter(_) | Op::Seed(_) => {
                scopes += 1;
                max_scopes = max_scopes.max(scopes);
                (usize::from(matches!(op, Op::Enter(_))), 0)
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

/// Appends `program` to `ops`/`tracks`, renumbering its track indices.
pub(crate) fn append(ops: &mut Vec<Op>, tracks: &mut Vec<Track>, program: &Program) {
    let offset = u16::try_from(tracks.len()).unwrap_or(u16::MAX);
    tracks.extend(program.tracks.iter().cloned());
    ops.extend(program.ops.iter().map(|op| match *op {
        Op::Track(index) => Op::Track(index.saturating_add(offset)),
        other => other,
    }));
}
