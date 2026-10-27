//! The single table of names the language understands.

/// A variable provided by the evaluation environment.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Var {
    /// `t`: scene time in seconds.
    Time,
    /// `f`: frame index.
    Frame,
    /// `fps`: frames per second.
    Fps,
    /// `dur`: scene duration in seconds.
    Duration,
}

/// A built-in function.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Func {
    Sin,
    Cos,
    Abs,
    Floor,
    Min,
    Max,
}

/// What an identifier refers to.
pub(crate) enum Name {
    Var(Var),
    Const(f64),
    Func(Func),
}

pub(crate) fn lookup(name: &str) -> Option<Name> {
    Some(match name {
        "t" => Name::Var(Var::Time),
        "f" => Name::Var(Var::Frame),
        "fps" => Name::Var(Var::Fps),
        "dur" => Name::Var(Var::Duration),
        "pi" => Name::Const(core::f64::consts::PI),
        "tau" => Name::Const(core::f64::consts::TAU),
        "sin" => Name::Func(Func::Sin),
        "cos" => Name::Func(Func::Cos),
        "abs" => Name::Func(Func::Abs),
        "floor" => Name::Func(Func::Floor),
        "min" => Name::Func(Func::Min),
        "max" => Name::Func(Func::Max),
        _ => return None,
    })
}

impl Func {
    pub(crate) const fn arity(self) -> u8 {
        match self {
            Self::Sin | Self::Cos | Self::Abs | Self::Floor => 1,
            Self::Min | Self::Max => 2,
        }
    }

    /// Applies the function; `None` when `args` does not match the arity.
    pub(crate) fn apply(self, args: &[f64]) -> Option<f64> {
        Some(match (self, args) {
            (Self::Sin, [x]) => libm::sin(*x),
            (Self::Cos, [x]) => libm::cos(*x),
            (Self::Abs, [x]) => libm::fabs(*x),
            (Self::Floor, [x]) => libm::floor(*x),
            (Self::Min, [a, b]) => libm::fmin(*a, *b),
            (Self::Max, [a, b]) => libm::fmax(*a, *b),
            _ => return None,
        })
    }
}
