//! Loads KDL scene files into the engine scene model. Memory profile: allocation-conscious authoring layer.
#![forbid(unsafe_code)]

mod diagnostic;
mod document;
mod palette;
mod props;
mod scene;
mod shapes;
mod values;

pub use diagnostic::{Diagnostic, LoadError, Severity};
pub use document::{Loaded, load_scene};
