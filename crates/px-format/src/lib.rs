#![forbid(unsafe_code)]

mod assemble;
mod attach;
mod compiler;
mod diagnostic;
mod document;
pub mod edit;
mod expand;
mod extras;
mod font;
mod geometry;
mod instances;
mod keyframes;
mod library;
mod markup;
mod names;
mod objects;
mod palette;
mod palettes;
mod params;
mod pixels;
mod props;
mod scene;
mod schema;
mod sequence;
mod source_map;
mod sources;
mod style;
mod suggest;
mod text;
mod timeline;
mod units;
mod values;

pub use diagnostic::{Diagnostic, LoadError, Severity};
pub use document::{
    LoadOptions, Loaded, LoadedSequence, Work, load, load_file, load_scene, load_work,
    load_work_file,
};
pub use expand::MotifSignature;
pub use font::{Font, Glyph};
pub use pixels::write_pixels;
pub use source_map::{Origin, PaintNames};
pub use sources::{FsSources, MemorySources, SourceFile, Sources};
pub use timeline::{Keyed, Marker, Timeline, Track};
