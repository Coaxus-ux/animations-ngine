use px_engine::TextCell;

use crate::font::{Font, Glyph};
use crate::markup::Style;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Align {
    Left,
    Center,
    Right,
}

impl Align {
    pub(crate) const NAMES: [&str; 3] = ["left", "center", "right"];

    pub(crate) fn parse(name: &str) -> Option<Self> {
        match name {
            "left" => Some(Self::Left),
            "center" => Some(Self::Center),
            "right" => Some(Self::Right),
            _ => None,
        }
    }

    fn start(self, width: i32) -> i32 {
        match self {
            Self::Left => 0,
            Self::Center => -(width / 2),
            Self::Right => -width,
        }
    }
}

/// Laid-out text: its ink pixels, and the characters the font has no glyph for (drawn
/// as boxes).
#[derive(Debug, Default)]
pub(crate) struct Layout {
    pub(crate) cells: Vec<TextCell>,
    pub(crate) missing: Vec<char>,
}

/// Italic leans one pixel right for every this many rows above the font's bottom row.
const ITALIC_RISE: u8 = 5;

/// Lines stack down from the anchor; each is placed on x by `align`. Within a line,
/// bigger characters sit on the same bottom as smaller ones. Every character but a line
/// break counts toward `reveal`.
pub(crate) fn layout(font: &Font, text: &[(char, Style)], align: Align) -> Layout {
    let mut out = Layout::default();
    let mut glyph: u16 = 0;
    let mut top = 0;
    for line in text.split(|&(character, _)| character == '\n') {
        let tall = line.iter().map(|(_, style)| style.scale).max().unwrap_or(1);
        let start = out.cells.len();
        let (mut x, mut right) = (0, 0);
        for &(character, style) in line {
            let placeholder;
            let found = if let Some(found) = font.glyph(character) {
                found
            } else {
                if !out.missing.contains(&character) {
                    out.missing.push(character);
                }
                placeholder = font.placeholder();
                &placeholder
            };
            let scale = i32::from(style.scale);
            let bottom = top + i32::from(font.height) * i32::from(tall);
            let glyph_top = bottom - i32::from(font.height) * scale;
            for (gx, gy) in pixels(font, found, style) {
                push(
                    &mut out.cells,
                    [x + gx * scale, glyph_top + gy * scale],
                    glyph,
                    style.scale,
                );
            }
            let lean = if style.italic { lean(font, 0) } else { 0 };
            let width = i32::from(found.width) + i32::from(style.bold);
            right = x + (width + lean) * scale;
            x += (width + i32::from(font.spacing)) * scale;
            glyph = glyph.saturating_add(1);
        }
        let shift = align.start(right);
        for cell in &mut out.cells[start..] {
            cell.x = cell.x.saturating_add(i16::try_from(shift).unwrap_or(0));
        }
        top += i32::from(font.line_height) * i32::from(tall);
    }
    out
}

/// A glyph's ink in font pixels, leaned for italic and thickened one pixel right for bold.
fn pixels(font: &Font, glyph: &Glyph, style: Style) -> Vec<(i32, i32)> {
    let mut pixels: Vec<(i32, i32)> = glyph
        .ink
        .iter()
        .flat_map(|&(gx, gy)| {
            let x = i32::from(gx) + if style.italic { lean(font, gy) } else { 0 };
            let thick = i32::from(style.bold);
            (x..=x + thick).map(move |x| (x, i32::from(gy)))
        })
        .collect();
    pixels.sort_unstable_by_key(|&(x, y)| (y, x));
    pixels.dedup();
    pixels
}

fn lean(font: &Font, row: u8) -> i32 {
    i32::from(font.height.saturating_sub(1).saturating_sub(row) / ITALIC_RISE)
}

fn push(cells: &mut Vec<TextCell>, [x, y]: [i32; 2], glyph: u16, scale: u8) {
    let (Ok(x), Ok(y)) = (i16::try_from(x), i16::try_from(y)) else {
        return;
    };
    cells.push(TextCell { x, y, glyph, scale });
}
