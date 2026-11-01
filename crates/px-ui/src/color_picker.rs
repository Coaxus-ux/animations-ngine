use px_core::{Hsv, Rgb};

use crate::area::{Rect, rect};
use crate::ui::{Id, Ui};

pub fn hue_bar(ui: &mut Ui<'_>, id: Id, area: Rect, color: &mut Hsv) -> bool {
    let before = *color;
    let response = ui.interact(id, area);
    if response.held
        && let Some((x, _)) = ui.input.pointer
    {
        *color = Hsv::new(fraction(x, area.x, area.width) * 360.0, color.s, color.v);
    }
    let pixels: Vec<Rgb> = (0..area.height)
        .flat_map(|_| 0..area.width)
        .map(|x| Hsv::new(fraction(x, 0, area.width) * 360.0, 1.0, 1.0).to_rgb())
        .collect();
    ui.canvas.rgb_image(area, &pixels);
    let marker_x = area.x + position(color.h / 360.0, area.width);
    ui.canvas
        .rgb_box(rect(marker_x, area.y, 1, area.height), Rgb::new(0, 0, 0));
    *color != before
}

pub fn sv_square(ui: &mut Ui<'_>, id: Id, area: Rect, color: &mut Hsv) -> bool {
    let before = *color;
    let response = ui.interact(id, area);
    if response.held
        && let Some((x, y)) = ui.input.pointer
    {
        *color = Hsv::new(
            color.h,
            fraction(x, area.x, area.width),
            1.0 - fraction(y, area.y, area.height),
        );
    }
    let pixels: Vec<Rgb> = (0..area.height)
        .flat_map(|y| (0..area.width).map(move |x| (x, y)))
        .map(|(x, y)| {
            Hsv::new(
                color.h,
                fraction(x, 0, area.width),
                1.0 - fraction(y, 0, area.height),
            )
            .to_rgb()
        })
        .collect();
    ui.canvas.rgb_image(area, &pixels);
    let marker = rect(
        area.x + position(color.s, area.width),
        area.y + position(1.0 - color.v, area.height),
        1,
        1,
    );
    let contrast = if color.v > 0.5 { 0 } else { 255 };
    ui.canvas
        .rgb_box(marker, Rgb::new(contrast, contrast, contrast));
    *color != before
}

fn fraction(x: i32, start: i32, length: i32) -> f64 {
    (f64::from(x - start) / f64::from((length - 1).max(1))).clamp(0.0, 1.0)
}

fn position(t: f64, length: i32) -> i32 {
    #[expect(clippy::cast_possible_truncation, reason = "clamped to the length")]
    let pixel = (t.clamp(0.0, 1.0) * f64::from((length - 1).max(0))).round() as i32;
    pixel
}
