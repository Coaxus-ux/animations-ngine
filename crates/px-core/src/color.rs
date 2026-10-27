use core::fmt;

/// An sRGB color with 8 bits per channel.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Rgb {
    /// Red channel.
    pub r: u8,
    /// Green channel.
    pub g: u8,
    /// Blue channel.
    pub b: u8,
}

/// Why a `#rrggbb` color could not be parsed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ColorParseError {
    /// The text does not start with `#`.
    MissingHash,
    /// The text after `#` is not exactly six digits long.
    WrongLength {
        /// Number of characters found after `#`.
        len: usize,
    },
    /// A character is not a hexadecimal digit.
    InvalidDigit {
        /// Position of the character after `#`.
        index: usize,
    },
}

impl Rgb {
    /// Creates a color from its channels.
    #[must_use]
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Parses `#rrggbb` (case-insensitive).
    ///
    /// # Errors
    ///
    /// Returns a [`ColorParseError`] describing the first problem found.
    pub fn parse_hex(text: &str) -> Result<Self, ColorParseError> {
        let digits = text.strip_prefix('#').ok_or(ColorParseError::MissingHash)?;
        let bytes = digits.as_bytes();
        if bytes.len() != 6 {
            return Err(ColorParseError::WrongLength { len: bytes.len() });
        }
        let channel = |index: usize| -> Result<u8, ColorParseError> {
            let high = hex_value(bytes[index], index)?;
            let low = hex_value(bytes[index + 1], index + 1)?;
            Ok((high << 4) | low)
        };
        Ok(Self::new(channel(0)?, channel(2)?, channel(4)?))
    }
}

fn hex_value(byte: u8, index: usize) -> Result<u8, ColorParseError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(ColorParseError::InvalidDigit { index }),
    }
}

impl fmt::Display for ColorParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingHash => f.write_str("color must start with '#'"),
            Self::WrongLength { len } => {
                write!(f, "color needs 6 hex digits after '#', found {len}")
            }
            Self::InvalidDigit { index } => write!(f, "invalid hex digit at position {index}"),
        }
    }
}
