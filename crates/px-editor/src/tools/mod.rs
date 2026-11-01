mod paint;
mod select;
mod shapes;

use px_core::Paint;
use px_ui::Key;

use crate::domain::{Raster, Selection};

pub struct Work<'w> {
    pub raster: &'w mut Raster,
    pub base: &'w Raster,
    pub selection: &'w Selection,
    pub paint: Option<Paint>,
    pub filled: bool,
    pub constrain: bool,
    pub global: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Effect {
    pub repaint: bool,
    pub commit: bool,
    pub selection: Option<Selection>,
    pub picked_paint: Option<Option<Paint>>,
    pub picked_object: Option<(i32, i32)>,
}

pub trait Tool {
    fn press(&mut self, work: &mut Work<'_>, at: (i32, i32)) -> Effect;
    fn drag(&mut self, work: &mut Work<'_>, at: (i32, i32)) -> Effect;
    fn release(&mut self, work: &mut Work<'_>, at: (i32, i32)) -> Effect;
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
