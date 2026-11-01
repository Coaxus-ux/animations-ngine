use px_core::{Paint, Palette, Size, Tone, dither_keep, dither_steps};

use crate::sequence::{Edge, TransitionKind};

const WIPE_BAND: f64 = 6.0;

#[derive(Clone, Copy)]
pub(crate) struct Side<'a> {
    pub(crate) shot: u8,
    pub(crate) cells: &'a [Paint],
    pub(crate) palette: &'a Palette,
}

pub(crate) struct Target<'a> {
    pub(crate) paints: &'a mut [Paint],
    pub(crate) shots: &'a mut [u8],
}

pub(crate) fn compose(
    kind: TransitionKind,
    progress: f64,
    size: Size,
    [from, to]: [Side<'_>; 2],
    Target { paints, shots }: Target<'_>,
) {
    let width = usize::from(size.width()).max(1);
    let fade = Fade::new(kind, progress, if progress < 0.5 { from } else { to });
    for (index, (paint, shot)) in paints.iter_mut().zip(shots.iter_mut()).enumerate() {
        let (x, y) = (coordinate(index % width), coordinate(index / width));
        let chosen = match kind {
            TransitionKind::Cut => pick(progress >= 0.5, from, to),
            TransitionKind::Dither => pick(dither_keep(progress, x, y), from, to),
            TransitionKind::Wipe(edge) => pick(wiped(edge, progress, size, x, y), from, to),
            TransitionKind::Black | TransitionKind::White => fade.side,
        };
        let source = chosen.cells[index];
        *shot = chosen.shot;
        *paint = match kind {
            TransitionKind::Black | TransitionKind::White => fade.apply(source, x, y),
            _ => source,
        };
    }
}

const fn pick<'a>(second: bool, from: Side<'a>, to: Side<'a>) -> Side<'a> {
    if second { to } else { from }
}

fn coordinate(value: usize) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

fn wiped(edge: Edge, progress: f64, size: Size, x: i32, y: i32) -> bool {
    let (width, height) = (i32::from(size.width()), i32::from(size.height()));
    let (along, extent) = match edge {
        Edge::Left => (x, width),
        Edge::Right => (width - 1 - x, width),
        Edge::Top => (y, height),
        Edge::Bottom => (height - 1 - y, height),
    };
    let front = progress * (f64::from(extent) + WIPE_BAND) - WIPE_BAND / 2.0;
    dither_keep((front - f64::from(along)) / WIPE_BAND + 0.5, x, y)
}

struct Fade<'a> {
    side: Side<'a>,
    toward: Option<Paint>,
    darker: bool,
    steps: f64,
}

impl<'a> Fade<'a> {
    fn new(kind: TransitionKind, progress: f64, side: Side<'a>) -> Self {
        let darker = kind != TransitionKind::White;
        let longest = side
            .palette
            .ramps()
            .iter()
            .map(px_core::Ramp::len)
            .max()
            .unwrap_or(1);
        let depth = 1.0 - (2.0 * progress - 1.0).abs();
        Self {
            side,
            toward: if darker {
                side.palette.darkest()
            } else {
                side.palette.lightest()
            },
            darker,
            steps: depth * (f64::from(longest) + 1.0),
        }
    }

    fn apply(&self, paint: Paint, x: i32, y: i32) -> Paint {
        let steps = dither_steps(self.steps, x, y);
        let tone = i32::from(paint.tone.index());
        let shifted = if self.darker {
            tone - steps
        } else {
            tone + steps
        };
        let len = self
            .side
            .palette
            .ramp(paint.ramp)
            .map_or(0, |ramp| i32::from(ramp.len()));
        match (u8::try_from(shifted), self.toward) {
            (Ok(tone), _) if i32::from(tone) < len => Paint {
                ramp: paint.ramp,
                tone: Tone::new(tone),
            },
            (_, Some(end)) => end,
            (_, None) => paint,
        }
    }
}
