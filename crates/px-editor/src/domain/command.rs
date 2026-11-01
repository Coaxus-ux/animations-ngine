use kdl::KdlValue;
use px_core::Rgb;
use px_engine::Cel;
use px_format::edit::{NodePath, Source};
use px_format::{PaintNames, write_pixels};

use super::palette_model::ramp_kdl;

/// Every change to a document (D-015): each turns into node-level edits of its KDL text.
#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    /// Rewrites a `pixels` node's legend and cels.
    SetCels {
        art: NodePath,
        cels: Vec<Cel>,
    },
    /// A new pixel layer as child `child` of the scene node.
    AddLayer {
        scene: NodePath,
        child: usize,
        name: String,
    },
    Remove {
        path: NodePath,
    },
    /// Moves child `from` of the scene node to `to`.
    MoveChild {
        scene: NodePath,
        from: usize,
        to: usize,
    },
    /// Renames a layer (its first argument) and its pixels node.
    Rename {
        path: NodePath,
        art: Option<NodePath>,
        name: String,
    },
    SetProperty {
        path: NodePath,
        key: String,
        value: KdlValue,
    },
    /// Sets color `tone` of a palette ramp.
    SetColor {
        ramp: NodePath,
        tone: usize,
        color: Rgb,
    },
    /// Rewrites one node from its text as `Source::node_text` gives it (the code view).
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
    /// Sets the scene's size and rewrites the pixels of each art in `arts` (already shifted
    /// and cropped by `resize::shifted_cels`).
    ResizeCanvas {
        scene: NodePath,
        width: u16,
        height: u16,
        arts: Vec<(NodePath, Vec<Cel>)>,
    },
}

impl Command {
    /// What undo and the status bar call it.
    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            Self::SetCels { .. } => "paint",
            Self::AddLayer { .. } => "add layer",
            Self::Remove { .. } => "delete",
            Self::MoveChild { .. } => "move layer",
            Self::Rename { .. } => "rename",
            Self::SetProperty { .. } => "set property",
            Self::SetColor { .. } => "change color",
            Self::ReplaceNode { .. } => "edit code",
            Self::AddRamp { .. } => "add ramp",
            Self::SetRamp { .. } => "edit ramp",
            Self::RemoveRamp { .. } => "remove ramp",
            Self::ResizeCanvas { .. } => "canvas size",
        }
    }

    /// # Errors
    /// A readable reason when a node is gone or the edit wouldn't be valid KDL.
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
            Self::SetProperty { path, key, value } => {
                source.set_property(path, key, value).map_err(fail)
            }
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
}
