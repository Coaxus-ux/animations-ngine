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
mod track;

pub use ease::Ease;
pub use error::{CompileError, CompileErrorKind, EvalError, ResolveError};
pub use names::Var;
pub use parser::MAX_NESTING;
pub use program::{Blend, Env, MAX_OPS, MAX_SCOPES, MAX_STACK, Program, Wind};
pub use track::{Key, Track, TrackLoop};

pub struct Reference {
    pub program: Program,
    pub seed: u64,
}

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

#[must_use]
pub fn value_noise(seed: u64, x: f64, y: f64) -> f64 {
    random::value_noise(seed, x, y)
}

#[must_use]
pub fn wind_at(field: Wind, time: f64, x: f64, loop_period: Option<f64>) -> f64 {
    random::wind(field, time, x, loop_period)
}

pub fn compile(source: &str) -> Result<Program, CompileError> {
    compile_with(source, &mut NoNames)
}

pub fn compile_with(source: &str, resolver: &mut dyn Resolver) -> Result<Program, CompileError> {
    parser::Parser::new(source, resolver).parse()
}
