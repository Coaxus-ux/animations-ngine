//! The editor's model: documents (KDL text as the source of truth), layers, commands,
//! history, palettes, exports, the element tree and the images and selections tools work
//! on. No UI, window or file system.

mod command;
mod document;
pub mod export;
pub mod frames;
mod history;
mod layers;
pub mod palette_import;
pub mod palette_model;
mod raster;
mod recent;
pub mod resize;
mod selection;
pub mod template;
pub mod tree;
pub mod visibility;

pub use command::Command;
pub use document::Document;
pub use export::{ExportJob, ExportKind, FrameScope};
pub use history::History;
pub use layers::{Art, Layer, scene_path};
pub use raster::{Brush, Raster};
pub use recent::{MAX_RECENT, Recent};
pub use resize::Anchor;
pub use selection::Selection;
pub use template::NewSpec;
pub use tree::{Element, ElementKind};
