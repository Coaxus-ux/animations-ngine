//! Turns indexed canvases into image files: RGB resolve, nearest-neighbor integer
//! upscale, PNG stills and looping GIFs.
//!
//! # Allocation
//!
//! Allocation-conscious: output buffers are allocated once per export and reused for
//! every frame.
#![forbid(unsafe_code)]

mod error;
mod gif_out;
mod png_out;
mod resolve;
mod scale;

pub use error::ExportError;
pub use gif_out::GifWriter;
pub use png_out::write_png;
pub use resolve::{indexed_len, resolve_rgb};
pub use scale::{MAX_SCALE, Scale};
