use super::param::Param;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TextCell {
    pub x: i16,
    pub y: i16,
    pub glyph: u16,
    pub scale: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Text {
    pub x: Param,
    pub y: Param,
    pub size: Param,
    pub reveal: Param,
    pub scatter: Param,
    pub cells: Vec<TextCell>,
}
