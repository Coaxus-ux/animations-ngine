use core::fmt;

/// What went wrong while compiling.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompileErrorKind {
    /// The expression is empty.
    Empty,
    /// A character that the language does not use.
    UnexpectedChar,
    /// A malformed number.
    InvalidNumber,
    /// A token in a place where it cannot appear.
    UnexpectedToken,
    /// A name that is not a variable, constant or function.
    UnknownName,
    /// A function name used without `( … )`.
    MissingCall,
    /// A variable or constant followed by `( … )`.
    NotAFunction,
    /// A function called with the wrong number of arguments.
    WrongArity {
        /// Arguments the function takes.
        expected: u8,
        /// Arguments given.
        found: u8,
    },
    /// Parentheses or unary operators nested deeper than `MAX_NESTING`.
    TooDeep,
    /// Evaluation would need more than `MAX_STACK` stack slots.
    TooComplex,
}

/// A compile error and the byte range of the text that caused it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompileError {
    /// What went wrong.
    pub kind: CompileErrorKind,
    /// Start byte offset in the expression source.
    pub start: usize,
    /// End byte offset (exclusive).
    pub end: usize,
}

/// Why evaluating a compiled program failed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvalError {
    /// The result is infinite or NaN (for example a division by zero).
    NonFinite,
    /// The program violates the compiler's stack invariant; a defect, never user error.
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
