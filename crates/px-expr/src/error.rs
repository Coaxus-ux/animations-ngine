use core::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompileErrorKind {
    Empty,
    UnexpectedChar,
    InvalidNumber,
    InvalidUnit,
    UnterminatedString,
    StringNotAllowed,
    UnknownEase,
    UnexpectedToken,
    UnknownName,
    MissingCall,
    NotAFunction,
    WrongArity { min: u8, max: u8, found: u8 },
    TooDeep,
    TooComplex,
    Resolve(ResolveError),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResolveError {
    UnknownParam,
    NotNumeric,
    UnknownReference,
    Cycle,
    Failed,
    TooDeep,
    Unsupported,
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
            Self::InvalidUnit => f.write_str("unknown unit (use s, ms, f or deg)"),
            Self::UnterminatedString => f.write_str("string is missing its closing quote"),
            Self::StringNotAllowed => {
                f.write_str("strings are only allowed as ease names or time literals")
            }
            Self::UnknownEase => f.write_str(
                "unknown ease (step, linear, smooth, ease-in, ease-out, ease-in-out, bounce, elastic)",
            ),
            Self::UnexpectedToken => f.write_str("unexpected token"),
            Self::UnknownName => f.write_str("unknown name (write `a - b` with spaces to subtract)"),
            Self::MissingCall => f.write_str("function needs arguments in parentheses"),
            Self::NotAFunction => f.write_str("not a function"),
            Self::WrongArity { min, max, found } if min == max => {
                write!(f, "function takes {min} argument(s), found {found}")
            }
            Self::WrongArity { min, max, found } => {
                write!(f, "function takes {min} to {max} arguments, found {found}")
            }
            Self::TooDeep => f.write_str("expression is nested too deeply"),
            Self::TooComplex => f.write_str("expression is too complex"),
            Self::Resolve(error) => write!(f, "{error}"),
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

impl fmt::Display for ResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::UnknownParam => "unknown param",
            Self::NotNumeric => "value is not a number",
            Self::UnknownReference => "unknown reference",
            Self::Cycle => "reference cycle",
            Self::Failed => "referenced property has errors",
            Self::TooDeep => "reference chain is too deep",
            Self::Unsupported => "this kind of reference is not available yet",
        })
    }
}
