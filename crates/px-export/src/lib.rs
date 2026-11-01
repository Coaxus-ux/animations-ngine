//! Indexed canvas to PNG and GIF; RGBA sprite sheets with Aseprite JSON. Memory profile: allocation-conscious; buffers are reused across frames.
#![forbid(unsafe_code)]

mod color_slots;
mod error;
mod gif_frame;
mod gif_out;
mod gif_rect;
mod json;
mod png_out;
mod resolve;
mod rgba;
mod scale;
mod sheet;
mod sheet_meta;
mod trim;

pub use error::ExportError;
pub use gif_frame::{EncodedFrame, GifFrameEncoder, GifScratch};
pub use gif_out::GifWriter;
pub use json::JsonWriter;
pub use png_out::{write_png, write_png_colors};
pub use resolve::{indexed_len, resolve_rgb};
pub use rgba::{RgbaImage, write_png_rgba};
pub use scale::{MAX_SCALE, Scale};
pub use sheet::{CellRect, Padding, SheetLayout, compose, place};
pub use sheet_meta::{FrameTag, SheetMeta, Trim, aseprite_json};
pub use trim::{crop, trim_bounds};
