use px_core::Paint;
use px_format::Font;

use crate::canvas::UiCanvas;

/// Draws `text` with its top-left at `(x, y)`; returns the x just past it.
pub fn draw_text(
    canvas: &mut UiCanvas,
    font: &Font,
    x: i32,
    y: i32,
    text: &str,
    paint: Paint,
) -> i32 {
    let mut pen = x;
    let placeholder = font.placeholder();
    for character in text.chars() {
        let glyph = font.glyph(character).unwrap_or(&placeholder);
        for &(column, row) in glyph.ink() {
            canvas.pixel(pen + i32::from(column), y + i32::from(row), paint);
        }
        pen += i32::from(glyph.width()) + i32::from(font.spacing());
    }
    pen
}

/// The width `text` takes, without trailing spacing.
#[must_use]
pub fn text_width(font: &Font, text: &str) -> i32 {
    let placeholder = font.placeholder();
    let total: i32 = text
        .chars()
        .map(|character| {
            i32::from(font.glyph(character).unwrap_or(&placeholder).width())
                + i32::from(font.spacing())
        })
        .sum();
    (total - i32::from(font.spacing())).max(0)
}

/// `text` shortened with `…` (three dots) to fit `width`.
#[must_use]
pub fn fit_text(font: &Font, text: &str, width: i32) -> String {
    if text_width(font, text) <= width {
        return text.to_owned();
    }
    let mut fitted: String = text.to_owned();
    while !fitted.is_empty() && text_width(font, &format!("{fitted}...")) > width {
        fitted.pop();
    }
    format!("{fitted}...")
}
