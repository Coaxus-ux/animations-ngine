use core::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompileErrorKind {
    Empty,
    UnexpectedChar,
    InvalidNumber,
    UnexpectedToken,
    UnknownName,
    MissingCall,
    NotAFunction,
    WrongArity { expected: u8, found: u8 },
    TooDeep,
    TooComplex,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompileError {
    pub kind: CompileErrorKind,
    pub start: usize,
    pub end: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvalError {
    NonFinite,
    Malformed,
}

impl fmt::Display for CompileErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("empty expression"),
            Self::UnexpectedChar => f.write_str("unexpected character"),
            Self::InvalidNumber => f.write_str("invalid number"),
            Self::UnexpectedToken => f.write_str("unexpected token"),
            Self::UnknownName => {
                f.write_str("unknown name (write `a - b` with spaces to subtract)")
            }
            Self::MissingCall => f.write_str("function needs arguments in parentheses"),
            Self::NotAFunction => f.write_str("not a function"),
            Self::WrongArity { expected, found } => {
                write!(f, "function takes {expected} argument(s), found {found}")
            }
            Self::TooDeep => f.write_str("expression is nested too deeply"),
            Self::TooComplex => f.write_str("expression is too complex"),
        }
    }
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at {}..{}", self.kind, self.start, self.end)
    }
}

impl fmt::Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFinite => f.write_str("expression result is not a finite number"),
            Self::Malformed => f.write_str("internal error: malformed expression program"),
        }
    }
}
