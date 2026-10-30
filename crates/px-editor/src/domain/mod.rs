//! The editor's model: documents (KDL text as the source of truth), layers, commands,
//! history, and the images and selections tools work on. No UI, window or file system.

mod command;
mod document;
mod history;
mod layers;
mod raster;
mod selection;

pub use command::Command;
pub use document::Document;
pub use history::History;
pub use layers::{Art, Layer, scene_path};
pub use raster::{Brush, Raster};
pub use selection::Selection;
