//! Indexed canvas to PNG and GIF. Memory profile: allocation-conscious; buffers are reused across frames.
#![forbid(unsafe_code)]

mod color_slots;
mod error;
mod gif_frame;
mod gif_out;
mod png_out;
mod resolve;
mod scale;

pub use error::ExportError;
pub use gif_frame::{EncodedFrame, GifFrameEncoder, GifScratch};
pub use gif_out::GifWriter;
pub use png_out::{write_png, write_png_colors};
pub use resolve::{indexed_len, resolve_rgb};
pub use scale::{MAX_SCALE, Scale};
