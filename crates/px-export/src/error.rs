use std::{fmt, io};

use px_core::{CanvasError, PaletteError};

/// Why an export failed.
#[derive(Debug)]
pub enum ExportError {
    /// Reading or writing failed.
    Io(io::Error),
    /// The image encoder rejected the data.
    Encoding(String),
    /// The palette has more colors than the format supports (GIF: 256).
    TooManyColors {
        /// Colors in the palette.
        count: usize,
        /// The format's limit.
        max: usize,
    },
    /// A pixel's paint is not in the palette.
    Palette(PaletteError),
    /// The canvas buffer does not match its size.
    Canvas(CanvasError),
    /// The upscaled image would exceed the format's limits.
    TooLarge,
}

impl fmt::Display for ExportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "i/o error: {error}"),
            Self::Encoding(message) => write!(f, "encoding error: {message}"),
            Self::TooManyColors { count, max } => {
                write!(f, "palette has {count} colors, the format allows {max}")
            }
            Self::Palette(error) => write!(f, "palette error: {error}"),
            Self::Canvas(error) => write!(f, "canvas error: {error}"),
            Self::TooLarge => f.write_str("upscaled image is too large for the format"),
        }
    }
}

impl std::error::Error for ExportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<io::Error> for ExportError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<PaletteError> for ExportError {
    fn from(error: PaletteError) -> Self {
        Self::Palette(error)
    }
}
