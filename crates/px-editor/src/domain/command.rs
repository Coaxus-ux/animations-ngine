use kdl::KdlValue;
use px_core::Rgb;
use px_engine::Cel;
use px_format::edit::{NodePath, Source};
use px_format::{PaintNames, write_pixels};

use super::palette_model::ramp_kdl;

#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    SetCels {
        art: NodePath,
        cels: Vec<Cel>,
    },
    AddLayer {
        scene: NodePath,
        child: usize,
        name: String,
    },
    Remove {
        path: NodePath,
    },
    MoveChild {
        scene: NodePath,
        from: usize,
        to: usize,
    },
    Rename {
        path: NodePath,
        art: Option<NodePath>,
        name: String,
    },
    SetArgument {
        path: NodePath,
        index: usize,
        value: KdlValue,
    },
    SetProperty {
        path: NodePath,
        key: String,
        value: KdlValue,
    },
    SetProperties {
        path: NodePath,
        values: Vec<(String, KdlValue)>,
    },
    SetColor {
        ramp: NodePath,
        tone: usize,
        color: Rgb,
    },
    ReplaceNode {
        path: NodePath,
        text: String,
    },
    AddRamp {
        palette: NodePath,
        name: String,
        colors: Vec<Rgb>,
    },
    SetRamp {
        ramp: NodePath,
        name: String,
        colors: Vec<Rgb>,
    },
    RemoveRamp {
        ramp: NodePath,
    },
    ResizeCanvas {
        scene: NodePath,
        width: u16,
        height: u16,
        arts: Vec<(NodePath, Vec<Cel>)>,
    },
}

impl Command {
    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            Self::SetCels { .. } => "paint",
            Self::AddLayer { .. } => "add layer",
            Self::Remove { .. } => "delete",
            Self::MoveChild { .. } => "move layer",
            Self::Rename { .. } => "rename",
            Self::SetArgument { .. } => "set value",
            Self::SetProperty { .. } => "set property",
            Self::SetProperties { .. } => "set properties",
            Self::SetColor { .. } => "change color",
            Self::ReplaceNode { .. } => "edit code",
            Self::AddRamp { .. } => "add ramp",
            Self::SetRamp { .. } => "edit ramp",
            Self::RemoveRamp { .. } => "remove ramp",
            Self::ResizeCanvas { .. } => "canvas size",
        }
    }

    pub fn apply(&self, source: &mut Source, names: &PaintNames) -> Result<(), String> {
        let fail = |error: px_format::edit::EditError| error.to_string();
        match self {
            Self::SetCels { art, cels } => set_cels(source, names, art, cels),
            Self::AddLayer { scene, child, name } => {
                let text = format!(
                    "layer \"{name}\" {{\n    pixels \"{name}-art\" x=0 y=0 {{\n        legend {{ }}\n    }}\n}}"
                );
                source.insert(scene, *child, &text).map_err(fail)
            }
            Self::Remove { path } => source.remove(path).map_err(fail),
            Self::MoveChild { scene, from, to } => {
                source.move_child(scene, *from, *to).map_err(fail)
            }
            Self::Rename { path, art, name } => {
                if let Some(art) = art {
                    let value = KdlValue::String(format!("{name}-art"));
                    source.set_argument(art, 0, &value).map_err(fail)?;
                }
                source
                    .set_argument(path, 0, &KdlValue::String(name.clone()))
                    .map_err(fail)
            }
            Self::SetArgument { path, index, value } => {
                source.set_argument(path, *index, value).map_err(fail)
            }
            Self::SetProperty { path, key, value } => {
                source.set_property(path, key, value).map_err(fail)
            }
            Self::SetProperties { path, values } => values
                .iter()
                .try_for_each(|(key, value)| source.set_property(path, key, value).map_err(fail)),
            Self::SetColor { ramp, tone, color } => source
                .set_argument(ramp, tone + 1, &KdlValue::String(color.to_string()))
                .map_err(fail),
            Self::ReplaceNode { path, text } => {
                let indent = source.indent(path);
                source
                    .replace(path, &Source::reindent(text, &indent))
                    .map_err(fail)
            }
            Self::AddRamp {
                palette,
                name,
                colors,
            } => source
                .insert(palette, usize::MAX, &ramp_kdl(name, colors))
                .map_err(fail),
            Self::SetRamp { ramp, name, colors } => {
                source.replace(ramp, &ramp_kdl(name, colors)).map_err(fail)
            }
            Self::RemoveRamp { ramp } => source.remove(ramp).map_err(fail),
            Self::ResizeCanvas {
                scene,
                width,
                height,
                arts,
            } => {
                for (key, side) in [("width", *width), ("height", *height)] {
                    source
                        .set_property(scene, key, &KdlValue::Integer(side.into()))
                        .map_err(fail)?;
                }
                arts.iter()
                    .try_for_each(|(art, cels)| set_cels(source, names, art, cels))
            }
        }
    }
}

fn set_cels(
    source: &mut Source,
    names: &PaintNames,
    art: &NodePath,
    cels: &[Cel],
) -> Result<(), String> {
    let children =
        write_pixels(cels, names).ok_or("a pixel uses a paint the palette has no name for")?;
    source
        .set_children(art, &children)
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "tests fail loudly")]

    use super::*;

    #[test]
    fn set_color_writes_the_tone_as_a_lower_case_hex_string() {
        let mut source =
            Source::parse("palette \"p\" {\n    ramp \"r\" \"#ffffff\" \"#000000\"\n}\n").unwrap();
        let ramp = NodePath::root()
            .child("palette", Some("p"), 0)
            .child("ramp", Some("r"), 0);
        let command = Command::SetColor {
            ramp,
            tone: 1,
            color: Rgb::new(0x0a, 0xb0, 0x01),
        };
        command.apply(&mut source, &PaintNames::default()).unwrap();
        assert_eq!(
            source.text(),
            "palette \"p\" {\n    ramp \"r\" \"#ffffff\" \"#0ab001\"\n}\n"
        );
    }

    #[test]
    fn set_properties_writes_every_value_or_none() {
        let text = "scene \"s\" fps=12 duration=\"18f\"\n";
        let mut source = Source::parse(text).unwrap();
        let path = NodePath::root().child("scene", Some("s"), 0);
        let values = |path: &NodePath| Command::SetProperties {
            path: path.clone(),
            values: vec![
                ("fps".to_owned(), KdlValue::Integer(24)),
                ("loop".to_owned(), KdlValue::Bool(false)),
            ],
        };
        values(&path)
            .apply(&mut source, &PaintNames::default())
            .unwrap();
        assert_eq!(
            source.text(),
            "scene \"s\" fps=24 duration=\"18f\" loop=#false\n"
        );

        let mut untouched = Source::parse(text).unwrap();
        let missing = NodePath::root().child("scene", Some("gone"), 0);
        assert!(
            values(&missing)
                .apply(&mut untouched, &PaintNames::default())
                .is_err()
        );
    }
}
