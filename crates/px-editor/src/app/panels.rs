use std::collections::BTreeSet;
use std::fmt::Write as _;

use kdl::KdlNode;

use super::window::PanelKind;

pub const MIN_COLUMN: i32 = 64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Side {
    Left,
    Right,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Panels {
    pub left: Option<i32>,
    pub right: Option<i32>,
    pub collapsed: BTreeSet<PanelKind>,
    pub palette_right: bool,
    pub every_element: bool,
}

impl Panels {
    #[must_use]
    pub fn width(&self, side: Side) -> Option<i32> {
        match side {
            Side::Left => self.left,
            Side::Right => self.right,
        }
    }

    pub fn set_width(&mut self, side: Side, width: i32) {
        let width = Some(width.max(MIN_COLUMN));
        match side {
            Side::Left => self.left = width,
            Side::Right => self.right = width,
        }
    }

    #[must_use]
    pub fn is_collapsed(&self, kind: PanelKind) -> bool {
        self.collapsed.contains(&kind)
    }

    pub fn toggle_collapsed(&mut self, kind: PanelKind) {
        if !self.collapsed.remove(&kind) {
            self.collapsed.insert(kind);
        }
    }

    #[must_use]
    pub fn to_kdl(&self) -> String {
        if *self == Self::default() {
            return String::new();
        }
        let mut text = "    panels".to_owned();
        for (key, width) in [("left", self.left), ("right", self.right)] {
            if let Some(width) = width {
                let _ = write!(text, " {key}={width}");
            }
        }
        if self.palette_right {
            text.push_str(" palette-right=#true");
        }
        if self.every_element {
            text.push_str(" every-element=#true");
        }
        for kind in &self.collapsed {
            let _ = write!(text, " {:?}", kind.title());
        }
        text.push('\n');
        text
    }

    #[must_use]
    pub fn from_node(node: &KdlNode) -> Self {
        let width = |key: &str| {
            node.entry(key)
                .and_then(|entry| entry.value().as_integer())
                .and_then(|width| i32::try_from(width).ok())
                .map(|width| width.max(MIN_COLUMN))
        };
        let flag = |key: &str| {
            node.entry(key)
                .and_then(|entry| entry.value().as_bool())
                .unwrap_or(false)
        };
        Self {
            left: width("left"),
            right: width("right"),
            collapsed: node
                .entries()
                .iter()
                .filter(|entry| entry.name().is_none())
                .filter_map(|entry| entry.value().as_string())
                .filter_map(PanelKind::from_title)
                .collect(),
            palette_right: flag("palette-right"),
            every_element: flag("every-element"),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "tests fail loudly")]

    use kdl::KdlDocument;

    use super::*;

    #[test]
    fn panels_survive_a_round_trip_through_kdl() {
        let mut panels = Panels {
            left: Some(130),
            right: None,
            palette_right: true,
            every_element: true,
            ..Panels::default()
        };
        panels.toggle_collapsed(PanelKind::Inspector);
        let document = KdlDocument::parse_v2(&panels.to_kdl()).unwrap();
        assert_eq!(Panels::from_node(&document.nodes()[0]), panels);
    }

    #[test]
    fn default_panels_write_nothing() {
        assert_eq!(Panels::default().to_kdl(), "");
    }
}
