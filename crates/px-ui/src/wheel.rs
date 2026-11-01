use crate::ui::{Id, Ui};
use crate::widgets::ROW;

pub const WHEEL_NOTCH: i32 = ROW;

impl Ui<'_> {
    pub fn wheel_steps(&mut self, id: Id, unit: i32) -> (i32, i32) {
        let (x, y) = self.input.wheel;
        let rest = self.state.wheel_rest.entry(id).or_default();
        let (steps_x, rest_x) = whole_steps(rest.0 + x * notch(), unit);
        let (steps_y, rest_y) = whole_steps(rest.1 + y * notch(), unit);
        *rest = (rest_x, rest_y);
        (-steps_x, -steps_y)
    }
}

#[expect(clippy::cast_precision_loss, reason = "a notch is a few pixels")]
const fn notch() -> f32 {
    WHEEL_NOTCH as f32
}

#[expect(clippy::cast_precision_loss, reason = "units are a few pixels")]
#[expect(clippy::cast_possible_truncation, reason = "a few steps per frame")]
fn whole_steps(pixels: f32, unit: i32) -> (i32, f32) {
    let unit = unit.max(1) as f32;
    let steps = (pixels / unit).trunc();
    (steps as i32, pixels - steps * unit)
}
