use px_engine::Cel;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Anchor {
    TopLeft,
    Top,
    TopRight,
    Left,
    #[default]
    Center,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
}

impl Anchor {
    pub const ALL: [Self; 9] = [
        Self::TopLeft,
        Self::Top,
        Self::TopRight,
        Self::Left,
        Self::Center,
        Self::Right,
        Self::BottomLeft,
        Self::Bottom,
        Self::BottomRight,
    ];

    #[must_use]
    pub fn default_index() -> usize {
        Self::ALL
            .iter()
            .position(|anchor| *anchor == Self::default())
            .unwrap_or(0)
    }

    #[must_use]
    pub const fn cell(self) -> (u8, u8) {
        match self {
            Self::TopLeft => (0, 0),
            Self::Top => (1, 0),
            Self::TopRight => (2, 0),
            Self::Left => (0, 1),
            Self::Center => (1, 1),
            Self::Right => (2, 1),
            Self::BottomLeft => (0, 2),
            Self::Bottom => (1, 2),
            Self::BottomRight => (2, 2),
        }
    }
}

#[must_use]
pub fn offset(anchor: Anchor, old: (u16, u16), new: (u16, u16)) -> (i32, i32) {
    let (column, row) = anchor.cell();
    let delta = |old: u16, new: u16, cell: u8| -> i32 {
        let difference = i32::from(new) - i32::from(old);
        match cell {
            0 => 0,
            1 => difference.div_euclid(2),
            _ => difference,
        }
    };
    (delta(old.0, new.0, column), delta(old.1, new.1, row))
}

#[must_use]
pub fn shifted_cels(cels: &[Cel], dx: i32, dy: i32, size: (u16, u16)) -> Vec<Cel> {
    cels.iter().map(|cel| shifted(cel, dx, dy, size)).collect()
}

fn shifted(cel: &Cel, dx: i32, dy: i32, size: (u16, u16)) -> Cel {
    let moved = (cel.x + dx, cel.y + dy);
    let left = moved.0.max(0);
    let top = moved.1.max(0);
    let right = (moved.0 + i32::from(cel.width)).min(i32::from(size.0));
    let bottom = (moved.1 + i32::from(cel.height)).min(i32::from(size.1));
    if right <= left || bottom <= top {
        return Cel::empty(cel.frame, 0, 0, 1, 1);
    }
    let mut cropped = Cel::empty(
        cel.frame,
        left,
        top,
        u16::try_from(right - left).unwrap_or(u16::MAX),
        u16::try_from(bottom - top).unwrap_or(u16::MAX),
    );
    for y in top..bottom {
        for x in left..right {
            if let (Some(from), Some(to)) = (cel.index(x - dx, y - dy), cropped.index(x, y)) {
                cropped.pixels[to] = cel.pixels[from];
            }
        }
    }
    cropped
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "tests fail loudly")]

    use px_core::{Paint, Palette, Rgb, Tone};

    use super::*;

    fn paint(tone: u8) -> Paint {
        let ramp = Palette::new().push_ramp(&[Rgb::new(0, 0, 0)]).unwrap();
        Paint {
            ramp,
            tone: Tone::new(tone),
        }
    }

    fn paints(tones: &[u8]) -> Vec<Option<Paint>> {
        tones.iter().map(|tone| Some(paint(*tone))).collect()
    }

    #[test]
    fn shifting_crops_pixels_outside_the_new_canvas() {
        let mut cel = Cel::empty(3, 0, 0, 3, 2);
        cel.pixels = paints(&[0, 1, 2, 3, 4, 5]);
        let moved = shifted_cels(&[cel.clone()], -1, 0, (2, 2));
        assert_eq!(moved.len(), 1);
        assert_eq!((moved[0].frame, moved[0].x, moved[0].y), (3, 0, 0));
        assert_eq!((moved[0].width, moved[0].height), (2, 2));
        assert_eq!(moved[0].pixels, paints(&[1, 2, 4, 5]));

        let gone = shifted_cels(&[cel], 10, 10, (4, 4));
        assert_eq!(gone[0], Cel::empty(3, 0, 0, 1, 1));
    }

    #[test]
    fn a_center_anchor_offset_is_half_the_difference() {
        assert_eq!(offset(Anchor::Center, (8, 8), (12, 10)), (2, 1));
        assert_eq!(offset(Anchor::TopLeft, (8, 8), (12, 10)), (0, 0));
        assert_eq!(offset(Anchor::BottomRight, (8, 8), (12, 10)), (4, 2));
        assert_eq!(offset(Anchor::Center, (8, 8), (5, 8)), (-2, 0));
    }
}
