use px_format::edit::{NodePath, Source};

use super::text::with_ramp_lines;
use crate::domain::palette_model::{PaletteModel, ramp_kdl};

pub(super) fn parse(name: &str, text: &str) -> Result<PaletteModel, String> {
    let source = Source::parse(text).map_err(|error| error.to_string())?;
    PaletteModel::from_source(&source, &NodePath::root().child("palette", Some(name), 0))
        .ok_or_else(|| format!("Preset \"{name}\" has no palette node"))
}

pub(super) fn node_text(
    source: &Source,
    model: &PaletteModel,
    preset: &PaletteModel,
) -> Result<String, String> {
    let kept: Vec<&str> = preset.ramps.iter().map(|ramp| ramp.name.as_str()).collect();
    for ramp in &model.ramps {
        if !kept.contains(&ramp.name.as_str())
            && let Some(user) = PaletteModel::uses_of(source, &ramp.name)
        {
            return Err(format!(
                "Ramp \"{}\" is used by {user} and preset \"{}\" has no such ramp",
                ramp.name, preset.name
            ));
        }
    }
    let node = source
        .node_text(&model.path)
        .ok_or("The palette node vanished")?;
    let mut draft = Source::parse(node).map_err(|error| error.to_string())?;
    let at_root = NodePath::root().child("palette", Some(&model.name), 0);
    for ramp in &model.ramps {
        draft
            .remove(&at_root.child("ramp", Some(&ramp.name), 0))
            .map_err(|error| error.to_string())?;
    }
    let lines = preset.ramps.iter().fold(String::new(), |mut lines, ramp| {
        lines.push_str("    ");
        lines.push_str(&ramp_kdl(&ramp.name, &ramp.colors));
        lines.push('\n');
        lines
    });
    let stripped = draft
        .node_text(&at_root)
        .ok_or("The palette node vanished")?;
    Ok(with_ramp_lines(&stripped, &lines))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "tests fail loudly")]

    use super::*;

    #[test]
    fn ramp_lines_are_swapped_and_the_rest_of_the_node_stays() {
        let file = "palette \"p\" {\n    // mood\n    ramp \"a\" \"#000000\"\n    ramp \"a-warm\" \"#101010\"\n    twin \"a\" warm=\"a-warm\"\n}\n";
        let source = Source::parse(file).unwrap();
        let path = NodePath::root().child("palette", Some("p"), 0);
        let model = PaletteModel::from_source(&source, &path).unwrap();
        let preset = parse(
            "q",
            "palette \"q\" {\n    ramp \"a\" \"#ffffff\"\n    ramp \"a-warm\" \"#eeeeee\"\n    ramp \"b\" \"#222222\"\n}\n",
        )
        .unwrap();
        assert_eq!(
            node_text(&source, &model, &preset).unwrap(),
            "palette \"p\" {\n    // mood\n    twin \"a\" warm=\"a-warm\"\n    ramp \"a\" \"#ffffff\"\n    ramp \"a-warm\" \"#eeeeee\"\n    ramp \"b\" \"#222222\"\n}"
        );
        let without_twin = parse("q", "palette \"q\" {\n    ramp \"a\" \"#ffffff\"\n}\n").unwrap();
        let refused = node_text(&source, &model, &without_twin).unwrap_err();
        assert!(
            refused.contains("\"a-warm\" is used by light color"),
            "{refused}"
        );
    }
}
