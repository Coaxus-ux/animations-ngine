//! Loads KDL scene files into the engine scene model. Memory profile: allocation-conscious authoring layer.
#![forbid(unsafe_code)]

mod compiler;
mod diagnostic;
mod document;
mod palette;
mod params;
mod props;
mod scene;
mod shapes;
mod units;
mod values;

pub use diagnostic::{Diagnostic, LoadError, Severity};
pub use document::{LoadOptions, Loaded, load_scene};
