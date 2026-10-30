//! Tools turn pointer events on the canvas into changes (the strategy pattern): each kind
//! is its own type behind `Tool`, so adding one changes nothing else (open/closed).

mod paint;
mod select;
mod shapes;

use px_core::Paint;
use px_ui::Key;

use crate::domain::{Raster, Selection};

/// What a stroke works on: the active cel as it was when the stroke began (`base`) and as
/// it is now (`raster`), the selection, and the paint (`None` erases).
pub struct Work<'w> {
    pub raster: &'w mut Raster,
    pub base: &'w Raster,
    pub selection: &'w Selection,
    pub paint: Option<Paint>,
    /// Shapes are filled instead of outlined.
    pub filled: bool,
    /// Shift: lines snap to 45°, boxes become squares and circles.
    pub constrain: bool,
    /// The fill bucket fills every matching pixel, not just the touching ones.
    pub global: bool,
}

/// What one pointer event did.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Effect {
    /// `raster` changed: show it.
    pub repaint: bool,
    /// The stroke is finished: keep `raster` as the cel.
    pub commit: bool,
    pub selection: Option<Selection>,
    /// The paint under the pointer (`Some(None)` for an empty pixel).
    pub picked_paint: Option<Option<Paint>>,
    /// A canvas pixel whose object should be selected.
    pub picked_object: Option<(i32, i32)>,
}

pub trait Tool {
    fn press(&mut self, work: &mut Work<'_>, at: (i32, i32)) -> Effect;
    fn drag(&mut self, work: &mut Work<'_>, at: (i32, i32)) -> Effect;
    fn release(&mut self, work: &mut Work<'_>, at: (i32, i32)) -> Effect;
    /// Canvas pixels to outline while the tool is in use (a lasso's path).
    fn guide(&self) -> &[(i32, i32)] {
        &[]
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ToolKind {
    #[default]
    Pencil,
    Eraser,
    Fill,
    Line,
    Rect,
    Ellipse,
    Eyedropper,
    Select,
    Lasso,
    Move,
    Pick,
}

impl ToolKind {
    pub const ALL: [Self; 11] = [
        Self::Pencil,
        Self::Eraser,
        Self::Fill,
        Self::Line,
        Self::Rect,
        Self::Ellipse,
        Self::Eyedropper,
        Self::Select,
        Self::Lasso,
        Self::Move,
        Self::Pick,
    ];

    #[must_use]
    pub const fn icon(self) -> &'static str {
        match self {
            Self::Pencil => "pencil",
            Self::Eraser => "eraser",
            Self::Fill => "fill",
            Self::Line => "line",
            Self::Rect => "rect",
            Self::Ellipse => "ellipse",
            Self::Eyedropper => "eyedropper",
            Self::Select => "select",
            Self::Lasso => "lasso",
            Self::Move => "move",
            Self::Pick => "pick",
        }
    }

    /// The tooltip: the tool's name and its shortcut.
    #[must_use]
    pub const fn tip(self) -> &'static str {
        match self {
            Self::Pencil => "Pencil (B)",
            Self::Eraser => "Eraser (E)",
            Self::Fill => "Fill (G, Shift: every match)",
            Self::Line => "Line (L)",
            Self::Rect => "Rectangle (U)",
            Self::Ellipse => "Ellipse (Shift+U)",
            Self::Eyedropper => "Eyedropper (I)",
            Self::Select => "Select box (M)",
            Self::Lasso => "Lasso (Q)",
            Self::Move => "Move selection (V)",
            Self::Pick => "Pick object to recolor (O)",
        }
    }

    /// The tool a key selects (Aseprite's keys where it has them).
    #[must_use]
    pub fn for_key(key: Key, shift: bool) -> Option<Self> {
        Some(match key {
            Key::Letter('b') => Self::Pencil,
            Key::Letter('e') => Self::Eraser,
            Key::Letter('g') => Self::Fill,
            Key::Letter('l') => Self::Line,
            Key::Letter('u') if shift => Self::Ellipse,
            Key::Letter('u') => Self::Rect,
            Key::Letter('i') => Self::Eyedropper,
            Key::Letter('m') => Self::Select,
            Key::Letter('q') => Self::Lasso,
            Key::Letter('v') => Self::Move,
            Key::Letter('o') => Self::Pick,
            _ => return None,
        })
    }

    /// Whether the tool changes pixels (and so needs a paintable layer).
    #[must_use]
    pub const fn paints(self) -> bool {
        !matches!(
            self,
            Self::Eyedropper | Self::Select | Self::Lasso | Self::Pick
        )
    }

    #[must_use]
    pub fn make(self) -> Box<dyn Tool> {
        match self {
            Self::Pencil | Self::Eraser => Box::new(paint::Pencil::default()),
            Self::Fill => Box::new(paint::Fill),
            Self::Eyedropper => Box::new(paint::Eyedropper),
            Self::Line => Box::new(shapes::ShapeTool::new(shapes::Kind::Line)),
            Self::Rect => Box::new(shapes::ShapeTool::new(shapes::Kind::Rect)),
            Self::Ellipse => Box::new(shapes::ShapeTool::new(shapes::Kind::Ellipse)),
            Self::Select => Box::new(select::BoxSelect::default()),
            Self::Lasso => Box::new(select::Lasso::default()),
            Self::Move => Box::new(select::Move::default()),
            Self::Pick => Box::new(select::Pick),
        }
    }
}

/// The pixels of a line from `a` to `b`, endpoints included (Bresenham).
#[must_use]
pub fn line_points(a: (i32, i32), b: (i32, i32)) -> Vec<(i32, i32)> {
    let (dx, dy) = ((b.0 - a.0).abs(), -(b.1 - a.1).abs());
    let (sx, sy) = ((b.0 - a.0).signum(), (b.1 - a.1).signum());
    let (mut x, mut y, mut error) = (a.0, a.1, dx + dy);
    let mut points = vec![(x, y)];
    while (x, y) != b {
        let double = 2 * error;
        if double >= dy {
            error += dy;
            x += sx;
        }
        if double <= dx {
            error += dx;
            y += sy;
        }
        points.push((x, y));
    }
    points
}
