use kdl::KdlValue;
use px_core::Rgb;
use px_engine::Cel;
use px_format::edit::{NodePath, Source};
use px_format::{PaintNames, write_pixels};

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
        }
    }

    /// # Errors
    /// A readable reason when a node is gone or the edit wouldn't be valid KDL.
    pub fn apply(&self, source: &mut Source, names: &PaintNames) -> Result<(), String> {
        let fail = |error: px_format::edit::EditError| error.to_string();
        match self {
            Self::SetCels { art, cels } => {
                let children = write_pixels(cels, names)
                    .ok_or("a pixel uses a paint the palette has no name for")?;
                source.set_children(art, &children).map_err(fail)
            }
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
            Self::SetColor { ramp, tone, color } => {
                let hex = format!("#{:02x}{:02x}{:02x}", color.r, color.g, color.b);
                source
                    .set_argument(ramp, tone + 1, &KdlValue::String(hex))
                    .map_err(fail)
            }
        }
    }
}
