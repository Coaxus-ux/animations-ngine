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
}

impl Default for Param {
    fn default() -> Self {
        Self::Fixed(0.0)
    }
}
