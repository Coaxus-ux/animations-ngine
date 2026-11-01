use std::collections::BTreeMap;

use kdl::{KdlDocument, KdlEntry, KdlNode};

use crate::diagnostic::Diagnostics;
use crate::props::Props;
use crate::suggest;
use crate::values;

pub(crate) const DEFAULT_FONT: &str = "5x7";
const DEFAULT_SOURCE: &str = include_str!("../fonts/5x7.kdl");

const MAX_GLYPH_SIDE: u8 = 64;
const INK: char = '#';
const EMPTY: char = '.';

pub(crate) type Fonts = BTreeMap<String, Font>;

#[derive(Clone, Debug)]
pub struct Font {
    pub(crate) height: u8,
    pub(crate) spacing: u8,
    pub(crate) line_height: u8,
    glyphs: BTreeMap<char, Glyph>,
}

#[derive(Clone, Debug)]
pub struct Glyph {
    pub(crate) width: u8,
    pub(crate) ink: Vec<(u8, u8)>,
}

impl Font {
    #[must_use]
    pub fn builtin() -> Option<Self> {
        default_font()
    }

    #[must_use]
    pub const fn height(&self) -> u8 {
        self.height
    }

    #[must_use]
    pub const fn spacing(&self) -> u8 {
        self.spacing
    }

    #[must_use]
    pub const fn line_height(&self) -> u8 {
        self.line_height
    }

    #[must_use]
    pub fn glyph(&self, character: char) -> Option<&Glyph> {
        self.glyphs.get(&character)
    }

    #[must_use]
    pub fn placeholder(&self) -> Glyph {
        let width = (self.height / 2 + 1).max(3);
        let (right, bottom) = (width - 1, self.height - 1);
        let ink = (0..=bottom)
            .flat_map(|y| (0..=right).map(move |x| (x, y)))
            .filter(|&(x, y)| x == 0 || y == 0 || x == right || y == bottom)
            .collect();
        Glyph { width, ink }
    }
}

impl Glyph {
    #[must_use]
    pub const fn width(&self) -> u8 {
        self.width
    }

    #[must_use]
    pub fn ink(&self) -> &[(u8, u8)] {
        &self.ink
    }
}

pub(crate) fn default_font() -> Option<Font> {
    let document = KdlDocument::parse_v2(DEFAULT_SOURCE).ok()?;
    let node = document.nodes().first()?;
    let mut diags = Diagnostics::default();
    let (_, font) = parse(node, &mut diags)?;
    (!diags.has_errors()).then_some(font)
}

pub(crate) fn parse(node: &KdlNode, diags: &mut Diagnostics) -> Option<(String, Font)> {
    let mut props = Props::new(node);
    let name = props.name_arg(diags)?;
    let height = props
        .require("height", diags)
        .and_then(|entry| values::bounded(entry, 1, MAX_GLYPH_SIDE, diags))?;
    let spacing = props
        .get("spacing")
        .and_then(|entry| values::bounded(entry, 0, MAX_GLYPH_SIDE, diags))
        .unwrap_or(1);
    let line_height = props
        .get("line-height")
        .and_then(|entry| values::bounded(entry, 1, u8::MAX, diags))
        .unwrap_or(height.saturating_add(2));
    props.finish(diags);
    let mut font = Font {
        height,
        spacing,
        line_height,
        glyphs: BTreeMap::new(),
    };
    for child in node.children().map(KdlDocument::nodes).unwrap_or_default() {
        match child.name().value() {
            "glyph" => font.add_glyph(child, diags),
            other => diags.error(
                child.span().offset(),
                format!(
                    "unknown node `{other}` in font (expected `glyph`).{}",
                    suggest::did_you_mean(other, ["glyph"])
                ),
            ),
        }
    }
    if font.glyphs.is_empty() {
        diags.error(
            node.span().offset(),
            format!("font \"{name}\" has no glyphs"),
        );
    }
    Some((name.to_owned(), font))
}

impl Font {
    fn add_glyph(&mut self, node: &KdlNode, diags: &mut Diagnostics) {
        let mut props = Props::new(node);
        let args: Vec<&KdlEntry> = props.args().collect();
        let width = props
            .get("width")
            .and_then(|entry| values::bounded(entry, 1, MAX_GLYPH_SIDE, diags));
        props.finish(diags);
        let offset = node.span().offset();
        let mut characters = args
            .first()
            .and_then(|entry| values::string(entry, diags))
            .unwrap_or_default()
            .chars();
        let (Some(character), None) = (characters.next(), characters.next()) else {
            diags.error(
                offset,
                "`glyph` needs one character: glyph \"A\" \"\"\"…\"\"\"",
            );
            return;
        };
        let glyph = match (args.get(1), width) {
            (Some(entry), _) => values::string(entry, diags)
                .and_then(|grid| self.grid(grid, entry.span().offset(), diags)),
            (None, Some(width)) => Some(Glyph {
                width,
                ink: Vec::new(),
            }),
            (None, None) => {
                diags.error(
                    offset,
                    format!("glyph \"{character}\" needs a grid or a width"),
                );
                None
            }
        };
        let Some(glyph) = glyph else { return };
        if self.glyphs.insert(character, glyph).is_some() {
            diags.error(offset, format!("glyph \"{character}\" is defined twice"));
        }
    }

    fn grid(&self, grid: &str, offset: usize, diags: &mut Diagnostics) -> Option<Glyph> {
        let rows: Vec<&str> = grid.split_whitespace().collect();
        let width = rows.first().map_or(0, |row| row.chars().count());
        let mut problem = None;
        if rows.len() != usize::from(self.height) {
            problem = Some(format!(
                "a glyph needs {} rows (the font's height), found {}",
                self.height,
                rows.len()
            ));
        } else if let Some(row) = rows.iter().find(|row| row.chars().count() != width) {
            problem = Some(format!(
                "every row of a glyph needs the same width; `{row}` differs"
            ));
        } else if let Some(other) = grid
            .chars()
            .find(|c| ![INK, EMPTY].contains(c) && !c.is_whitespace())
        {
            problem = Some(format!(
                "glyphs use `{INK}` for ink and `{EMPTY}` for empty, found `{other}`"
            ));
        }
        let width = u8::try_from(width)
            .ok()
            .filter(|width| (1..=MAX_GLYPH_SIDE).contains(width));
        let (None, Some(width)) = (&problem, width) else {
            diags.error(
                offset,
                problem.unwrap_or_else(|| format!("glyphs are 1..={MAX_GLYPH_SIDE} pixels wide")),
            );
            return None;
        };
        let ink = (0u8..)
            .zip(&rows)
            .flat_map(|(y, row)| {
                (0u8..)
                    .zip(row.chars())
                    .filter(|(_, c)| *c == INK)
                    .map(move |(x, _)| (x, y))
            })
            .collect();
        Some(Glyph { width, ink })
    }
}
