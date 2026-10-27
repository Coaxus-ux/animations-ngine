//! The expression language of scene files (`docs/FORMAT.md` §9).
//!
//! M0 subset: numbers, `+ - * / %`, unary `-`, parentheses, the variables `t`, `f`, `fps`,
//! `dur`, the constants `pi`, `tau` and the functions `sin`, `cos`, `abs`, `floor`, `min`,
//! `max`. Math uses the pure-Rust `libm`, so results are identical on every platform.
//!
//! # Allocation
//!
//! [`compile`] allocates the compiled program once (allocation-conscious).
//! [`Program::eval`] performs no heap allocation: it uses a fixed stack of [`MAX_STACK`]
//! values, a bound checked at compile time.
#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

mod error;
mod lexer;
mod names;
mod parser;
mod program;

pub use error::{CompileError, CompileErrorKind, EvalError};
pub use names::Var;
pub use parser::MAX_NESTING;
pub use program::{Env, MAX_STACK, Program};

pub fn compile(source: &str) -> Result<Program, CompileError> {
    parser::Parser::new(source).parse()
}
