use std::collections::BTreeSet;

use kdl::{KdlDocument, KdlEntry, KdlNode};
use px_core::Paint;
use px_expr::Wind;

use crate::diagnostic::Diagnostics;
use crate::extras::{self, Extras};
use crate::keyframes::Anim;
use crate::names::Names;
use crate::pixels;
use crate::props::Props;
use crate::schema::Kind;
use crate::style::{self, Flags};
use crate::suggest;
use crate::values;

const STRUCTURE: [&str; 3] = ["param", "wind", "timeline"];
const MAX_CLIP_NESTING: usize = 8;

pub(crate) struct Draft<'a> {
    pub(crate) node: &'a KdlNode,
    pub(crate) id: Option<&'a str>,
    pub(crate) kind: Kind,
    pub(crate) entries: Vec<Option<&'a KdlEntry>>,
    pub(crate) anims: Vec<Option<Anim<'a>>>,
    pub(crate) paint: Option<Paint>,
    pub(crate) flags: Flags,
    pub(crate) extras: Extras,
    pub(crate) layer: usize,
    pub(crate) clips: Vec<usize>,
    /// `attach="id"` or `attach="placed.id"`: what the object is positioned from.
    pub(crate) attach: Option<&'a KdlEntry>,
}

#[derive(Default)]
pub(crate) struct Tree<'a> {
    pub(crate) drafts: Vec<Draft<'a>>,
    pub(crate) wind: Wind,
    pub(crate) layers: Vec<usize>,
}

#[derive(Clone, Default)]
struct Context {
    layer: usize,
    clips: Vec<usize>,
}

pub(crate) fn collect<'a>(
    scene: &'a KdlNode,
    palette: &Names<'_>,
    diags: &mut Diagnostics,
) -> Tree<'a> {
    let mut tree = Tree::default();
    walk(scene, &Context::default(), palette, &mut tree, diags);
    let mut seen = BTreeSet::new();
    for draft in &tree.drafts {
        if let Some(id) = draft.id.filter(|id| !seen.insert(*id)) {
            diags.error(
                draft.node.span().offset(),
                format!("id \"{id}\" is used twice in this scene"),
            );
        }
    }
    tree
}

fn children(node: &KdlNode) -> &[KdlNode] {
    node.children().map(KdlDocument::nodes).unwrap_or_default()
}

fn walk<'a>(
    parent: &'a KdlNode,
    context: &Context,
    palette: &Names<'_>,
    tree: &mut Tree<'a>,
    diags: &mut Diagnostics,
) {
    for node in children(parent) {
        let name = node.name().value();
        if name == "wind" {
            tree.wind = wind(node, diags);
            continue;
        }
        // A group's or layer's own keyframes were read with its draft.
        if STRUCTURE.contains(&name) || (name == "anim" && parent.name().value() != "scene") {
            continue;
        }
        let Some(kind) = Kind::parse(name) else {
            let known = Kind::all_names().chain(STRUCTURE);
            diags.error(
                node.span().offset(),
                format!(
                    "unknown node `{name}`.{}",
                    suggest::did_you_mean(name, known)
                ),
            );
            continue;
        };
        let index = tree.drafts.len();
        let draft = draft(node, kind, context, palette, diags);
        let clip = matches!(draft.extras, Extras::Group { clip: true, .. });
        tree.drafts.push(draft);
        match kind {
            Kind::Layer if context.layer != 0 => {
                diags.error(node.span().offset(), "layers cannot be nested");
            }
            Kind::Layer => {
                tree.layers.push(index);
                walk(
                    node,
                    &Context {
                        layer: tree.layers.len(),
                        ..context.clone()
                    },
                    palette,
                    tree,
                    diags,
                );
            }
            Kind::Group => {
                let mut inner = context.clone();
                if clip {
                    inner.clips.push(index);
                }
                if inner.clips.len() > MAX_CLIP_NESTING {
                    diags.error(
                        node.span().offset(),
                        format!("clip groups can be nested at most {MAX_CLIP_NESTING} deep"),
                    );
                }
                walk(node, &inner, palette, tree, diags);
            }
            _ => {}
        }
    }
}

fn draft<'a>(
    node: &'a KdlNode,
    kind: Kind,
    context: &Context,
    palette: &Names<'_>,
    diags: &mut Diagnostics,
) -> Draft<'a> {
    let mut props = Props::new(node);
    let id = match kind {
        Kind::UseScene | Kind::Text => props.get("id"),
        _ => props.args().next(),
    }
    .and_then(|entry| values::string(entry, diags));
    let entries: Vec<Option<&KdlEntry>> = kind.props().map(|prop| props.get(prop.name)).collect();
    let anims = anims(node, kind, diags);
    let paint = match kind {
        Kind::Shade => palette.darkest(),
        // Pixels bring their own paints; `paint` only colors outlines and an empty layer.
        Kind::Pixels => match props.get("paint") {
            Some(entry) => values::string(entry, diags)
                .and_then(|text| palette.paint(text, entry.span().offset(), diags)),
            None => palette.darkest(),
        },
        _ if kind.is_shape() => props.require("paint", diags).and_then(|entry| {
            let text = values::string(entry, diags)?;
            palette.paint(text, entry.span().offset(), diags)
        }),
        _ => None,
    };
    let flags = if kind.is_shape() {
        style::parse(&mut props, palette, diags)
    } else {
        Flags::default()
    };
    let attach = if kind.can_attach() {
        props.get("attach")
    } else {
        None
    };
    let extras = match kind {
        Kind::Pixels => Extras::Pixels(pixels::parse(node, palette, diags)),
        _ => extras::parse(&mut props, kind, palette, diags),
    };
    props.finish(diags);
    Draft {
        node,
        id,
        kind,
        entries,
        anims,
        paint,
        flags,
        extras,
        layer: context.layer,
        clips: context.clips.clone(),
        attach,
    }
}

fn anims<'a>(node: &'a KdlNode, kind: Kind, diags: &mut Diagnostics) -> Vec<Option<Anim<'a>>> {
    let mut anims = vec![None; kind.prop_count()];
    let name = node.name().value();
    for child in children(node) {
        match child.name().value() {
            "anim" => {
                let property = child
                    .entries()
                    .first()
                    .and_then(|entry| entry.value().as_string())
                    .unwrap_or("");
                if let Some(index) = kind.prop_index(property) {
                    anims[index] = Some(Anim {
                        node: child,
                        multi: None,
                    });
                } else {
                    let hint = suggest::did_you_mean(property, kind.props().map(|prop| prop.name));
                    diags.error(
                        child.span().offset(),
                        format!("`{name}` has no animatable property `{property}`.{hint}"),
                    );
                }
            }
            "k" if kind == Kind::Camera => multi_key(node, child, kind, &mut anims, diags),
            _ if matches!(kind, Kind::Layer | Kind::Group) => {}
            "legend" | "cel" if kind == Kind::Pixels => {}
            other => diags.error(
                child.span().offset(),
                format!("`{name}` can only contain `anim` blocks, found `{other}`"),
            ),
        }
    }
    anims
}

fn multi_key<'a>(
    node: &'a KdlNode,
    key: &KdlNode,
    kind: Kind,
    anims: &mut [Option<Anim<'a>>],
    diags: &mut Diagnostics,
) {
    for name in key
        .entries()
        .iter()
        .filter_map(|entry| entry.name())
        .map(kdl::KdlIdentifier::value)
    {
        match kind.props().enumerate().find(|(_, prop)| prop.name == name) {
            Some((index, prop)) => {
                anims[index] = Some(Anim {
                    node,
                    multi: Some(prop.name),
                });
            }
            None if name == "interp" => {}
            None => {
                let hint = suggest::did_you_mean(name, kind.props().map(|prop| prop.name));
                diags.error(
                    key.span().offset(),
                    format!("`{}` keys cannot set `{name}`.{hint}", node.name().value()),
                );
            }
        }
    }
}

fn wind(node: &KdlNode, diags: &mut Diagnostics) -> Wind {
    let mut props = Props::new(node);
    let wind = Wind {
        strength: extras::number(&mut props, "strength", 1.0, diags),
        gust: extras::number(&mut props, "gust", 0.5, diags),
        speed: extras::number(&mut props, "speed", 16.0, diags),
    };
    props.finish(diags);
    wind
}
