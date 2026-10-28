//! Loads KDL scene files into the engine scene model. Memory profile: allocation-conscious authoring layer.
#![forbid(unsafe_code)]

mod assemble;
mod compiler;
mod diagnostic;
mod document;
mod extras;
mod keyframes;
mod objects;
mod palette;
mod params;
mod props;
mod scene;
mod schema;
mod style;
mod suggest;
mod units;
mod values;

pub use diagnostic::{Diagnostic, LoadError, Severity};
pub use document::{LoadOptions, Loaded, load_scene};
