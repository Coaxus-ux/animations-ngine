use px_expr::{Env, EvalError, Program};

#[derive(Clone, Debug, PartialEq)]
pub enum Param {
    Fixed(f64),
    Expr(Program),
}

impl Param {
    pub fn value(&self, env: &Env) -> Result<f64, EvalError> {
        match self {
            Self::Fixed(value) => Ok(*value),
            Self::Expr(program) => program.eval(env),
        }
    }

    /// The same value in every frame (see `Program::is_timeless`).
    #[must_use]
    pub const fn is_timeless(&self) -> bool {
        match self {
            Self::Fixed(_) => true,
            Self::Expr(program) => program.is_timeless(),
        }
    }
}

impl Default for Param {
    fn default() -> Self {
        Self::Fixed(0.0)
    }
}
