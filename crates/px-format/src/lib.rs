//! Loads KDL scene files into the engine scene model. Memory profile: allocation-conscious authoring layer.
#![forbid(unsafe_code)]

mod assemble;
mod compiler;
mod diagnostic;
mod document;
mod extras;
mod instances;
mod keyframes;
mod library;
mod objects;
mod paints;
mod palette;
mod params;
mod props;
mod scene;
mod schema;
mod sources;
mod style;
mod suggest;
mod timeline;
mod units;
mod values;

pub use diagnostic::{Diagnostic, LoadError, Severity};
pub use document::{LoadOptions, Loaded, load, load_file, load_scene};
pub use sources::{FsSources, MemorySources, SourceFile, Sources};
pub use timeline::{Marker, Timeline, Track};
