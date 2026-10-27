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
mod stack;

pub use ease::Ease;
pub use error::{CompileError, CompileErrorKind, EvalError, ResolveError};
pub use names::Var;
pub use parser::MAX_NESTING;
pub use program::{Env, MAX_OPS, MAX_SCOPES, MAX_STACK, Program};

pub struct Reference {
    pub program: Program,
    pub seed: u64,
}

/// Supplies `$param` values and `@id.prop` programs while compiling.
pub trait Resolver {
    fn param(&mut self, name: &str) -> Result<f64, ResolveError>;
    fn reference(&mut self, path: &str) -> Result<Reference, ResolveError>;
}

struct NoNames;

impl Resolver for NoNames {
    fn param(&mut self, _: &str) -> Result<f64, ResolveError> {
        Err(ResolveError::UnknownParam)
    }

    fn reference(&mut self, _: &str) -> Result<Reference, ResolveError> {
        Err(ResolveError::UnknownReference)
    }
}

pub fn compile(source: &str) -> Result<Program, CompileError> {
    compile_with(source, &mut NoNames)
}

pub fn compile_with(source: &str, resolver: &mut dyn Resolver) -> Result<Program, CompileError> {
    parser::Parser::new(source, resolver).parse()
}
