use px_core::LightColor;
use px_engine::object_seed;
use px_engine::{
    Ambient, Anchor, Bounds, Camera, Clip, Environment, Geometry, Layer, Light, LightKind, Outline,
    Param, Particles, Reflection, Segment, Shading, Shape, Style, Text,
};
use px_expr::{Blend, Program};

use crate::attach::Targets;
use crate::diagnostic::Diagnostics;
use crate::extras::{Extras, Placement};
use crate::objects::Tree;
use crate::schema::Kind;
use crate::style::Flags;
use crate::timeline::Track;
use crate::values;

/// Hands out a draft's compiled properties in schema order.
struct Next(std::vec::IntoIter<Param>);

impl Next {
    fn param(&mut self) -> Param {
        self.0.next().unwrap_or_default()
    }

    fn bounds(&mut self) -> Bounds {
        Bounds {
            x: self.param(),
            y: self.param(),
            width: self.param(),
            height: self.param(),
        }
    }

    fn segment(&mut self) -> Segment {
        Segment {
            x1: self.param(),
            y1: self.param(),
            x2: self.param(),
            y2: self.param(),
        }
    }
}

/// Everything a scene and the scenes it places add up to.
#[derive(Debug, Default)]
pub(crate) struct Output {
    pub(crate) shapes: Vec<Shape>,
    pub(crate) world: Environment,
    pub(crate) tracks: Vec<Track>,
    pub(crate) targets: Targets,
}

/// Where one scene's objects land: the instance that owns them (none for the root scene),
/// the root of their seeds, and the host layer and clips they inherit.
#[derive(Clone, Debug, Default)]
pub(crate) struct Scope {
    pub(crate) instance: Option<usize>,
    pub(crate) seed: u64,
    pub(crate) layer: usize,
    pub(crate) clips: Vec<usize>,
    /// The scene's loop length in seconds, when it loops.
    pub(crate) loop_period: Option<f64>,
}

/// A `use-scene` met while assembling, with its compiled properties in schema order.
pub(crate) struct Request {
    pub(crate) placement: Box<Placement>,
    pub(crate) params: Vec<Param>,
    pub(crate) offset: usize,
    /// Whether `seed` was written; otherwise each copy derives one.
    pub(crate) seeded: bool,
    /// Position of the `use-scene` among the scene's objects, for derived seeds.
    pub(crate) order: usize,
    pub(crate) id: Option<String>,
    pub(crate) parent: Scope,
    pub(crate) anchor: Option<Anchor>,
}

pub(crate) fn assemble(
    tree: Tree<'_>,
    params: Vec<Vec<Param>>,
    scope: &Scope,
    out: &mut Output,
    diags: &mut Diagnostics,
    place: &mut dyn FnMut(Request, &mut Output, &mut Diagnostics),
) {
    if scope.instance.is_none() {
        out.world.wind = tree.wind;
    }
    let mut layers = vec![scope.layer];
    let mut clip_ids = vec![None; tree.drafts.len()];
    let mut shape_index = 0;
    for (index, (draft, params)) in tree.drafts.into_iter().zip(params).enumerate() {
        let own_layer = layers.get(draft.layer).copied().unwrap_or(scope.layer);
        let attached = draft.attach.and_then(|entry| {
            let path = values::string(entry, diags)?;
            out.targets
                .resolve(scope.instance, path)
                .map_err(|message| diags.error(entry.span().offset(), message))
                .ok()
        });
        let layer = attached.as_ref().map_or(own_layer, |(_, layer)| *layer);
        let anchor = attached.map(|(anchor, _)| anchor);
        if let Some(id) = draft.id.filter(|_| draft.kind != Kind::UseScene) {
            out.targets.point(
                scope.instance,
                id,
                draft.kind,
                &params,
                anchor.as_ref(),
                layer,
            );
        }
        let mut next = Next(params.into_iter());
        match (draft.kind, draft.extras) {
            (_, Extras::Instance(placement)) => {
                let request = Request {
                    placement,
                    params: next.0.collect(),
                    offset: draft.node.span().offset(),
                    seeded: Kind::UseScene
                        .prop_index("seed")
                        .and_then(|seed| draft.entries.get(seed))
                        .is_some_and(Option::is_some),
                    order: index,
                    id: draft.id.map(str::to_owned),
                    parent: Scope {
                        layer,
                        clips: clips(scope, &draft.clips, &clip_ids),
                        ..scope.clone()
                    },
                    anchor,
                };
                place(request, out, diags);
            }
            (kind, extras) if kind.is_shape() => {
                let geometry = geometry(kind, extras, &mut next);
                let clips = clips(scope, &draft.clips, &clip_ids);
                let style = Style {
                    instance: scope.instance,
                    anchor,
                    seed: Some(object_seed(scope.seed, shape_index)),
                    ..style(&draft.flags, &mut next, layer, clips)
                };
                shape_index += 1;
                out.shapes.extend(draft.paint.map(|paint| Shape {
                    geometry,
                    paint,
                    style,
                }));
            }
            (kind, extras) => {
                let node = SceneNode {
                    kind,
                    extras,
                    layer,
                    anchor,
                };
                match world_node(node, &mut next, scope, &mut out.world) {
                    Added::Layer(added) => layers.push(added),
                    Added::Clip(added) => clip_ids[index] = Some(added),
                    Added::Other => {}
                }
            }
        }
    }
}

/// A node that adds to the scene's environment rather than drawing.
struct SceneNode {
    kind: Kind,
    extras: Extras,
    layer: usize,
    anchor: Option<Anchor>,
}

enum Added {
    Layer(usize),
    Clip(usize),
    Other,
}

/// Camera, ambient light and wind belong to the root scene; a placed scene's own are only
/// for previewing it alone.
fn world_node(node: SceneNode, next: &mut Next, scope: &Scope, world: &mut Environment) -> Added {
    let root = scope.instance.is_none();
    match (node.kind, node.extras) {
        (
            _,
            Extras::Layer {
                parallax,
                fog_paint,
            },
        ) => {
            let layer = Layer {
                parallax,
                fog: next.param(),
                fog_paint,
            };
            return Added::Layer(push_layer(world, layer));
        }
        (_, Extras::Group { clip: true, round }) => {
            world.clips.push(Clip {
                bounds: next.bounds(),
                round,
                layer: node.layer,
                instance: scope.instance,
            });
            return Added::Clip(world.clips.len() - 1);
        }
        (
            _,
            Extras::Ambient {
                occlusion,
                key_light,
            },
        ) if root => {
            world.ambient = Ambient {
                darkness: next.param(),
                occlusion,
                key_light,
            };
        }
        (Kind::Camera, _) if root => {
            world.camera = Camera {
                x: next.param(),
                y: next.param(),
                zoom: next.param(),
            };
        }
        (kind, Extras::Light { color, shadows }) => {
            world.lights.push(Light {
                layer: node.layer,
                instance: scope.instance,
                anchor: node.anchor,
                ..light(kind, next, color, shadows)
            });
        }
        (_, Extras::Reflection { surface }) => world.reflections.push(Reflection {
            bounds: next.bounds(),
            axis: next.param(),
            ripple: next.param(),
            speed: next.param(),
            tone_shift: next.param(),
            opacity: next.param(),
            surface,
            instance: scope.instance,
        }),
        _ => {}
    }
    Added::Other
}

/// Adds a layer after the implicit root layer every layered scene starts with.
fn push_layer(world: &mut Environment, layer: Layer) -> usize {
    if world.layers.is_empty() {
        world.layers.push(Layer::default());
    }
    world.layers.push(layer);
    world.layers.len() - 1
}

/// The host's clips followed by the scene's own, as indices into the host's clip list.
fn clips(scope: &Scope, groups: &[usize], clip_ids: &[Option<usize>]) -> Vec<usize> {
    scope
        .clips
        .iter()
        .copied()
        .chain(groups.iter().filter_map(|group| clip_ids[*group]))
        .collect()
}

fn light(kind: Kind, next: &mut Next, color: Option<LightColor>, shadows: bool) -> Light {
    let rect = (kind == Kind::LightRect).then(|| next.bounds());
    let (x, y) = match &rect {
        Some(bounds) => (bounds.x.clone(), bounds.y.clone()),
        None => (next.param(), next.param()),
    };
    let (reach, intensity) = (next.param(), next.param());
    let kind = match (kind, rect) {
        (_, Some(bounds)) => LightKind::Rect {
            width: bounds.width,
            height: bounds.height,
        },
        (Kind::LightCone, None) => LightKind::Cone {
            angle: next.param(),
            spread: next.param(),
        },
        _ => LightKind::Point,
    };
    Light {
        kind,
        x,
        y,
        reach,
        intensity,
        color,
        shadows,
        layer: 0,
        instance: None,
        anchor: None,
    }
}

fn style(flags: &Flags, next: &mut Next, layer: usize, clips: Vec<usize>) -> Style {
    Style {
        opacity: next.param(),
        tone_shift: next.param(),
        visible: next.param(),
        contact_shadow: next.param(),
        emissive: flags.emissive,
        casts_shadow: flags.casts_shadow,
        cleanup: flags.cleanup,
        shading: flags.shading.unwrap_or(Shading::Flat),
        outline: flags.outline.unwrap_or(Outline::None),
        texture: flags.texture,
        gradient: flags.gradient,
        layer,
        clips,
        instance: None,
        anchor: None,
        seed: None,
    }
}

fn geometry(kind: Kind, extras: Extras, next: &mut Next) -> Geometry {
    match (kind, extras) {
        (Kind::Ellipse, _) => Geometry::Ellipse(next.bounds()),
        (Kind::RoundedRect, _) => Geometry::RoundedRect {
            bounds: next.bounds(),
            radius: next.param(),
        },
        (Kind::Line, _) => Geometry::Line {
            segment: next.segment(),
            width: next.param(),
        },
        (Kind::Capsule, _) => Geometry::Line {
            segment: next.segment(),
            width: doubled(next.param()),
        },
        (_, Extras::Points(points)) => Geometry::Polygon {
            x: next.param(),
            y: next.param(),
            points,
        },
        (_, Extras::Circles(circles)) => Geometry::Blob {
            x: next.param(),
            y: next.param(),
            circles,
            smooth: next.param(),
        },
        (Kind::Rope, _) => Geometry::Rope {
            segment: next.segment(),
            sag: next.param(),
            sway: next.param(),
            width: next.param(),
        },
        (_, Extras::Particles { count, fade }) => Geometry::Particles(Box::new(Particles {
            area: next.bounds(),
            velocity_x: next.param(),
            velocity_y: next.param(),
            gravity: next.param(),
            life: next.param(),
            size: next.param(),
            wind: next.param(),
            streak: next.param(),
            count,
            fade,
        })),
        (_, Extras::Text(cells)) => Geometry::Text(Box::new(Text {
            x: next.param(),
            y: next.param(),
            size: next.param(),
            reveal: next.param(),
            scatter: next.param(),
            cells,
        })),
        (_, Extras::Shade { round }) => Geometry::Shade {
            bounds: next.bounds(),
            steps: next.param(),
            round,
        },
        _ => Geometry::Rect(next.bounds()),
    }
}

fn doubled(radius: Param) -> Param {
    match radius {
        Param::Fixed(radius) => Param::Fixed(radius * 2.0),
        Param::Expr(program) => Param::Expr(Program::blend(
            &program,
            &Program::fixed(2.0),
            Blend::Multiply,
        )),
    }
}
