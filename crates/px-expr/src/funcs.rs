use core::f64::consts::{PI, TAU};

use crate::program::Wind;
use crate::random;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Func {
    Abs,
    Sign,
    Floor,
    Ceil,
    Round,
    Fract,
    Min,
    Max,
    Clamp,
    Lerp,
    Map,
    Step,
    Smoothstep,
    Mod,
    Sqrt,
    Exp,
    Log,
    Pow,
    Sin,
    Cos,
    Tan,
    Atan2,
    Deg,
    Rad,
    Dist,
    Len,
    Snap,
    Wave,
    Tri,
    Saw,
    Square,
    Cycle,
    Osc,
    Pulse,
    Between,
    Loop,
    Delay,
    Stagger,
    Hold,
    On2,
    On3,
    Ease,
    Rand,
    Range,
    Randi,
    Chance,
    Pick,
    Noise,
    Fbm,
    Cells,
    Wiggle,
    Wind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Scope {
    Loop,
    Delay,
    Stagger,
    Hold,
}

pub(crate) struct Ctx {
    pub(crate) time: f64,
    pub(crate) seed: u64,
    pub(crate) loop_period: Option<f64>,
    pub(crate) wind: Wind,
}

const NAMES: [(&str, Func); 52] = [
    ("wind", Func::Wind),
    ("abs", Func::Abs),
    ("sign", Func::Sign),
    ("floor", Func::Floor),
    ("ceil", Func::Ceil),
    ("round", Func::Round),
    ("fract", Func::Fract),
    ("min", Func::Min),
    ("max", Func::Max),
    ("clamp", Func::Clamp),
    ("lerp", Func::Lerp),
    ("map", Func::Map),
    ("step", Func::Step),
    ("smoothstep", Func::Smoothstep),
    ("mod", Func::Mod),
    ("sqrt", Func::Sqrt),
    ("exp", Func::Exp),
    ("log", Func::Log),
    ("pow", Func::Pow),
    ("sin", Func::Sin),
    ("cos", Func::Cos),
    ("tan", Func::Tan),
    ("atan2", Func::Atan2),
    ("deg", Func::Deg),
    ("rad", Func::Rad),
    ("dist", Func::Dist),
    ("len", Func::Len),
    ("snap", Func::Snap),
    ("wave", Func::Wave),
    ("tri", Func::Tri),
    ("saw", Func::Saw),
    ("square", Func::Square),
    ("cycle", Func::Cycle),
    ("osc", Func::Osc),
    ("pulse", Func::Pulse),
    ("between", Func::Between),
    ("loop", Func::Loop),
    ("delay", Func::Delay),
    ("stagger", Func::Stagger),
    ("hold", Func::Hold),
    ("on2", Func::On2),
    ("on3", Func::On3),
    ("ease", Func::Ease),
    ("rand", Func::Rand),
    ("range", Func::Range),
    ("randi", Func::Randi),
    ("chance", Func::Chance),
    ("pick", Func::Pick),
    ("noise", Func::Noise),
    ("fbm", Func::Fbm),
    ("cells", Func::Cells),
    ("wiggle", Func::Wiggle),
];

impl Func {
    pub(crate) fn parse(name: &str) -> Option<Self> {
        NAMES
            .iter()
            .find(|(known, _)| *known == name)
            .map(|(_, func)| *func)
    }

    pub(crate) const fn arity(self) -> (u8, u8) {
        match self {
            Self::Rand => (0, 1),
            Self::Abs
            | Self::Sign
            | Self::Floor
            | Self::Ceil
            | Self::Round
            | Self::Fract
            | Self::Sqrt
            | Self::Exp
            | Self::Log
            | Self::Sin
            | Self::Cos
            | Self::Tan
            | Self::Deg
            | Self::Rad
            | Self::Wave
            | Self::Tri
            | Self::Saw
            | Self::Cycle
            | Self::On2
            | Self::On3
            | Self::Wind => (1, 1),
            Self::Snap | Self::Square | Self::Pulse | Self::Chance => (1, 2),
            Self::Osc | Self::Noise => (1, 3),
            Self::Step
            | Self::Mod
            | Self::Pow
            | Self::Atan2
            | Self::Len
            | Self::Between
            | Self::Loop
            | Self::Delay
            | Self::Stagger
            | Self::Hold
            | Self::Ease
            | Self::Cells
            | Self::Wiggle => (2, 2),
            Self::Range | Self::Randi | Self::Fbm => (2, 3),
            Self::Min | Self::Max => (2, 8),
            Self::Pick => (2, 9),
            Self::Clamp | Self::Lerp | Self::Smoothstep => (3, 3),
            Self::Dist => (4, 4),
            Self::Map => (5, 5),
        }
    }

    pub(crate) const fn defaults(self) -> &'static [f64] {
        match self {
            Self::Snap => &[1.0],
            Self::Square => &[0.5],
            Self::Pulse => &[0.1],
            Self::Osc => &[-1.0, 1.0],
            Self::Rand | Self::Chance | Self::Range | Self::Randi => &[0.0],
            Self::Fbm => &[4.0],
            _ => &[],
        }
    }

    pub(crate) const fn scope(self) -> Option<(Scope, Option<f64>)> {
        match self {
            Self::Loop => Some((Scope::Loop, None)),
            Self::Delay => Some((Scope::Delay, None)),
            Self::Stagger => Some((Scope::Stagger, None)),
            Self::Hold => Some((Scope::Hold, None)),
            Self::On2 => Some((Scope::Hold, Some(2.0))),
            Self::On3 => Some((Scope::Hold, Some(3.0))),
            _ => None,
        }
    }

    pub(crate) fn apply(self, args: &[f64], ctx: &Ctx) -> Option<f64> {
        Some(match (self, args) {
            (Self::Min, [first, rest @ ..]) => rest.iter().fold(*first, |a, b| libm::fmin(a, *b)),
            (Self::Max, [first, rest @ ..]) => rest.iter().fold(*first, |a, b| libm::fmax(a, *b)),
            (Self::Pick, [key, options @ ..]) => pick(ctx.seed, *key, options)?,
            (_, [x]) => self.unary(*x, ctx)?,
            (_, [a, b]) => self.binary(*a, *b, ctx)?,
            (_, [a, b, c]) => self.ternary(*a, *b, *c, ctx)?,
            (Self::Dist, [x1, y1, x2, y2]) => libm::hypot(x2 - x1, y2 - y1),
            (Self::Map, [value, from, until, to, end]) => {
                to + (value - from) * (end - to) / (until - from)
            }
            _ => return None,
        })
    }

    fn unary(self, x: f64, ctx: &Ctx) -> Option<f64> {
        Some(match self {
            Self::Abs => libm::fabs(x),
            Self::Sign => {
                if x > 0.0 {
                    1.0
                } else if x < 0.0 {
                    -1.0
                } else {
                    0.0
                }
            }
            Self::Floor => libm::floor(x),
            Self::Ceil => libm::ceil(x),
            Self::Round => libm::round(x),
            Self::Fract | Self::Saw => fract(x),
            Self::Sqrt => libm::sqrt(x),
            Self::Exp => libm::exp(x),
            Self::Log => libm::log(x),
            Self::Sin => libm::sin(x),
            Self::Cos => libm::cos(x),
            Self::Tan => libm::tan(x),
            Self::Deg => x * 180.0 / PI,
            Self::Rad => x * PI / 180.0,
            Self::Wave => 0.5 - 0.5 * libm::cos(TAU * x),
            Self::Tri => 1.0 - libm::fabs(2.0 * fract(x) - 1.0),
            Self::Cycle => fract(ctx.time / x),
            Self::Rand => random::rand(ctx.seed, x),
            Self::Noise => random::noise(ctx.seed, [x, 0.0, 0.0]),
            Self::Wind => random::wind(ctx.wind, ctx.time, x, ctx.loop_period),
            _ => return None,
        })
    }

    fn binary(self, a: f64, b: f64, ctx: &Ctx) -> Option<f64> {
        Some(match self {
            Self::Step => truth(b >= a),
            Self::Mod => a - b * libm::floor(a / b),
            Self::Pow => libm::pow(a, b),
            Self::Atan2 => libm::atan2(a, b),
            Self::Len => libm::hypot(a, b),
            Self::Snap => libm::round(a / b) * b,
            Self::Square => truth(fract(a) < b),
            Self::Pulse => truth(fract(ctx.time / a) < b),
            Self::Between => truth(a <= ctx.time && ctx.time < b),
            Self::Chance => truth(random::rand(ctx.seed, b) < a),
            Self::Noise => random::noise(ctx.seed, [a, b, 0.0]),
            Self::Cells => random::cells(ctx.seed, a, b),
            Self::Wiggle => random::wiggle(ctx.seed, ctx.time, a, b, ctx.loop_period),
            _ => return None,
        })
    }

    fn ternary(self, a: f64, b: f64, c: f64, ctx: &Ctx) -> Option<f64> {
        Some(match self {
            Self::Clamp => libm::fmin(libm::fmax(a, b), c),
            Self::Lerp => a + (b - a) * c,
            Self::Smoothstep => {
                let k = ((c - a) / (b - a)).clamp(0.0, 1.0);
                k * k * (3.0 - 2.0 * k)
            }
            Self::Osc => b + (c - b) * (0.5 + 0.5 * libm::sin(TAU * ctx.time / a)),
            Self::Noise => random::noise(ctx.seed, [a, b, c]),
            Self::Range => a + (b - a) * random::rand(ctx.seed, c),
            Self::Randi => libm::fmin(
                libm::floor(a + (b - a + 1.0) * random::rand(ctx.seed, c)),
                b,
            ),
            Self::Fbm => random::fbm(ctx.seed, a, b, c),
            _ => return None,
        })
    }
}

fn fract(x: f64) -> f64 {
    x - libm::floor(x)
}

const fn truth(condition: bool) -> f64 {
    if condition { 1.0 } else { 0.0 }
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "index is floored and bounded by the option count (≤ 8)"
)]
fn pick(seed: u64, key: f64, options: &[f64]) -> Option<f64> {
    let index = (random::rand(seed, key) * options.len() as f64) as usize;
    options
        .get(index.min(options.len().checked_sub(1)?))
        .copied()
}
