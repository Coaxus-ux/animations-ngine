//! Expression language (`docs/FORMAT.md` §9). Memory profile: `compile` allocates once; `Program::eval` never allocates.
#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

mod ease;
mod error;
mod funcs;
mod lexer;
mod names;
mod parser;
mod program;
mod random;

pub use ease::Ease;
pub use error::{CompileError, CompileErrorKind, EvalError};
pub use names::Var;
pub use parser::MAX_NESTING;
pub use program::{Env, MAX_SCOPES, MAX_STACK, Program};

pub fn compile(source: &str) -> Result<Program, CompileError> {
    parser::Parser::new(source).parse()
}
