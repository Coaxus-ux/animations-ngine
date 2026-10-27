use crate::funcs::Func;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Var {
    Time,
    Frame,
    Fps,
    Duration,
    Seed,
    Index,
    Count,
    Position,
    X,
    Y,
    Width,
    Height,
}

pub(crate) enum Name {
    Var(Var),
    Const(f64),
    Func(Func),
}

pub(crate) fn lookup(name: &str) -> Option<Name> {
    let var = match name {
        "t" => Var::Time,
        "f" => Var::Frame,
        "fps" => Var::Fps,
        "dur" => Var::Duration,
        "seed" => Var::Seed,
        "i" => Var::Index,
        "n" => Var::Count,
        "u" => Var::Position,
        "x" => Var::X,
        "y" => Var::Y,
        "w" => Var::Width,
        "h" => Var::Height,
        "pi" => return Some(Name::Const(core::f64::consts::PI)),
        "tau" => return Some(Name::Const(core::f64::consts::TAU)),
        "true" => return Some(Name::Const(1.0)),
        "false" => return Some(Name::Const(0.0)),
        _ => return Func::parse(name).map(Name::Func),
    };
    Some(Name::Var(var))
}
