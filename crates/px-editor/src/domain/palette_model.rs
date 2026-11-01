mod uses;

use std::fmt::Write as _;

use px_core::Rgb;
use px_format::edit::{NodePath, Source};

use super::Document;
use super::layers::scene_path;

use super::tree::{children_of, id_of};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RampModel {
    pub name: String,
    pub colors: Vec<Rgb>,
    pub path: NodePath,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaletteModel {
    pub name: String,
    pub path: NodePath,
    pub ramps: Vec<RampModel>,
}

impl PaletteModel {
    #[must_use]
    pub fn from_source(source: &Source, palette: &NodePath) -> Option<Self> {
        let node = source.node(palette)?;
        let children = children_of(node);
        let ramps = children
            .iter()
            .filter(|child| child.name().value() == "ramp")
            .filter_map(|child| {
                let name = id_of(child)?;
                let colors = child
                    .entries()
                    .iter()
                    .filter(|entry| entry.name().is_none())
                    .skip(1)
                    .filter_map(|entry| entry.value().as_string())
                    .filter_map(|hex| Rgb::parse_hex(hex).ok())
                    .collect();
                Some(RampModel {
                    name: name.to_owned(),
                    colors,
                    path: Source::path_of(palette, children, child),
                })
            })
            .collect();
        Some(Self {
            name: id_of(node).unwrap_or_default().to_owned(),
            path: palette.clone(),
            ramps,
        })
    }

    #[must_use]
    pub fn to_kdl(&self) -> String {
        let mut text = format!("palette {:?} {{\n", self.name);
        for ramp in &self.ramps {
            let _ = writeln!(text, "    {}", ramp_kdl(&ramp.name, &ramp.colors));
        }
        text.push_str("}\n");
        text
    }
}

#[must_use]
pub fn ramp_kdl(name: &str, colors: &[Rgb]) -> String {
    let mut text = format!("ramp {name:?}");
    for color in colors {
        let _ = write!(text, " \"{color}\"");
    }
    text
}

pub fn palette_path(document: &Document) -> Result<NodePath, String> {
    let loaded = document
        .loaded()
        .ok_or("Fix the scene's errors before editing its palette")?;
    let source = document.source();
    let name = source
        .node(&scene_path(&loaded.scene))
        .and_then(|scene| scene.entry("palette"))
        .and_then(|entry| entry.value().as_string())
        .ok_or("The scene names no palette")?;
    let path = NodePath::root().child("palette", Some(name), 0);
    if source.node(&path).is_none() {
        return Err(format!("Palette \"{name}\" lives in another file"));
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "tests fail loudly")]

    use super::*;

    const FILE: &str = "palette \"p\" {\n    ramp \"ink\" \"#000000\" \"#FFFFFF\"\n    ramp \"sky\" \"#102030\"\n    ramp \"sky-warm\" \"#302010\"\n    twin \"sky\" warm=\"sky-warm\"\n}\n\nscene \"s\" palette=\"p\" width=4 height=4 background=\"sky.0\" {\n    // paint=\"ink.1\" in a comment does not count\n    layer \"l\" {\n        rect \"floor\" x=0 paint=\"ink.1\" outline-paint=\"ink.hi\"\n        pixels \"art\" {\n            legend { a \"sky.0\"; b \"ink\" }\n            cel \"\"\"\n            ab\n            \"\"\"\n        }\n    }\n}\n";

    fn palette() -> NodePath {
        NodePath::root().child("palette", Some("p"), 0)
    }

    #[test]
    fn a_palette_is_read_with_its_ramps_and_written_back_lowercase() {
        let source = Source::parse(FILE).unwrap();
        let model = PaletteModel::from_source(&source, &palette()).unwrap();
        assert_eq!(model.name, "p");
        assert_eq!(model.ramps.len(), 3);
        assert_eq!(model.ramps[0].name, "ink");
        assert_eq!(
            model.ramps[0].colors,
            vec![Rgb::new(0, 0, 0), Rgb::new(255, 255, 255)]
        );
        assert_eq!(model.ramps[1].path, palette().child("ramp", Some("sky"), 0));
        assert!(source.node(&model.ramps[1].path).is_some());
        assert_eq!(
            model.to_kdl(),
            "palette \"p\" {\n    ramp \"ink\" \"#000000\" \"#ffffff\"\n    ramp \"sky\" \"#102030\"\n    ramp \"sky-warm\" \"#302010\"\n}\n"
        );
    }

    #[test]
    fn ramp_kdl_is_one_line_of_quoted_lowercase_hex() {
        assert_eq!(
            ramp_kdl("fur", &[Rgb::new(0xAB, 1, 2)]),
            "ramp \"fur\" \"#ab0102\""
        );
    }

    #[test]
    fn uses_of_finds_the_first_twin_legend_code_or_paint_property_in_file_order() {
        let source = Source::parse(FILE).unwrap();
        assert_eq!(
            PaletteModel::uses_of(&source, "sky").unwrap(),
            "twin \"sky\""
        );
        assert_eq!(
            PaletteModel::uses_of(&source, "sky-warm").unwrap(),
            "light color \"warm\" of twin \"sky\""
        );
        let no_twin = FILE.replace("    twin \"sky\" warm=\"sky-warm\"\n", "");
        let source = Source::parse(no_twin).unwrap();
        assert_eq!(
            PaletteModel::uses_of(&source, "sky").unwrap(),
            "background of scene \"s\""
        );
        assert_eq!(PaletteModel::uses_of(&source, "sky-warm"), None);
        assert_eq!(
            PaletteModel::uses_of(&source, "ink").unwrap(),
            "paint of rect \"floor\""
        );
        let legend_only = FILE.replace("paint=\"ink.1\" outline-paint=\"ink.hi\"\n", "\n");
        let source = Source::parse(legend_only).unwrap();
        assert_eq!(
            PaletteModel::uses_of(&source, "ink").unwrap(),
            "legend code \"b\" in pixels \"art\""
        );
        assert_eq!(PaletteModel::uses_of(&source, "in"), None);
    }

    #[test]
    fn every_paint_property_counts_as_a_use() {
        let outline_only = FILE
            .replace("paint=\"ink.1\" ", "")
            .replace("b \"ink\"", "b \"sky.0\"");
        let source = Source::parse(outline_only.as_str()).unwrap();
        assert_eq!(
            PaletteModel::uses_of(&source, "ink").unwrap(),
            "outline-paint of rect \"floor\""
        );
        let fog = outline_only.replace("outline-paint=\"ink.hi\"", "fog-paint=\"ink\"");
        let source = Source::parse(fog).unwrap();
        assert_eq!(
            PaletteModel::uses_of(&source, "ink").unwrap(),
            "fog-paint of rect \"floor\""
        );
    }

    #[test]
    fn dropped_numeric_tones_are_found_but_named_tones_fit_any_length() {
        let source = Source::parse(FILE).unwrap();
        assert_eq!(
            PaletteModel::uses_of_tones(&source, "ink", 1).unwrap(),
            (1, "paint of rect \"floor\"".to_owned())
        );
        assert_eq!(PaletteModel::uses_of_tones(&source, "ink", 2), None);
        assert_eq!(PaletteModel::uses_of_tones(&source, "sky", 1), None);
        assert_eq!(
            PaletteModel::uses_of_tones(&source, "sky", 0).unwrap(),
            (0, "background of scene \"s\"".to_owned())
        );
    }

    #[test]
    fn twin_partners_link_a_base_with_its_twins_both_ways() {
        let source = Source::parse(FILE).unwrap();
        assert_eq!(
            PaletteModel::twin_partners(&source, "sky"),
            vec!["sky-warm"]
        );
        assert_eq!(
            PaletteModel::twin_partners(&source, "sky-warm"),
            vec!["sky"]
        );
        assert_eq!(
            PaletteModel::twin_partners(&source, "ink"),
            Vec::<String>::new()
        );
    }
}
