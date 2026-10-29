use super::param::Param;

/// One ink pixel of laid-out text; `x` and `y` are its top-left in glyph pixels from the
/// text's anchor.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TextCell {
    pub x: i16,
    pub y: i16,
    /// Position of its character in the text, compared with `reveal`.
    pub glyph: u16,
    /// Its side in glyph pixels, for text that mixes sizes; 1 for most text.
    pub scale: u8,
}

/// Pixel-font text, laid out when the scene loads.
#[derive(Clone, Debug, PartialEq)]
pub struct Text {
    pub x: Param,
    pub y: Param,
    /// Canvas pixels per glyph pixel.
    pub size: Param,
    /// How many characters show, from the first.
    pub reveal: Param,
    /// How far each glyph pixel is pushed out from its place, in random directions.
    pub scatter: Param,
    pub cells: Vec<TextCell>,
}
