use px_core::LightColor;
use px_engine::{
    Ambient, Bounds, Camera, Clip, Environment, Geometry, Layer, Light, LightKind, Outline, Param,
    Particles, Reflection, Segment, Shading, Shape, Style,
};
use px_expr::{Blend, Program};

use crate::extras::Extras;
use crate::objects::Tree;
use crate::schema::Kind;
use crate::style::Flags;

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

pub(crate) fn assemble(tree: Tree<'_>, params: Vec<Vec<Param>>) -> (Vec<Shape>, Environment) {
    let mut world = Environment {
        wind: tree.wind,
        ..Environment::default()
    };
    if !tree.layers.is_empty() {
        world.layers.push(Layer::default());
    }
    let mut clip_ids = vec![None; tree.drafts.len()];
    let mut shapes = Vec::new();
    for (index, (draft, params)) in tree.drafts.into_iter().zip(params).enumerate() {
        let mut next = Next(params.into_iter());
        match (draft.kind, draft.extras) {
            (
                _,
                Extras::Layer {
                    parallax,
                    fog_paint,
                },
            ) => world.layers.push(Layer {
                parallax,
                fog: next.param(),
                fog_paint,
            }),
            (_, Extras::Group { clip: true, round }) => {
                clip_ids[index] = Some(world.clips.len());
                world.clips.push(Clip {
                    bounds: next.bounds(),
                    round,
                    layer: draft.layer,
                });
            }
            (_, Extras::Group { .. }) => {}
            (
                _,
                Extras::Ambient {
                    occlusion,
                    key_light,
                },
            ) => {
                world.ambient = Ambient {
                    darkness: next.param(),
                    occlusion,
                    key_light,
                };
            }
            (Kind::Camera, _) => {
                world.camera = Camera {
                    x: next.param(),
                    y: next.param(),
                }
            }
            (kind, Extras::Light { color, shadows }) => {
                world.lights.push(light(kind, &mut next, color, shadows));
            }
            (_, Extras::Reflection { surface }) => world.reflections.push(Reflection {
                bounds: next.bounds(),
                axis: next.param(),
                ripple: next.param(),
                speed: next.param(),
                tone_shift: next.param(),
                opacity: next.param(),
                surface,
            }),
            (kind, extras) => {
                let geometry = geometry(kind, extras, &mut next);
                let clips = draft
                    .clips
                    .iter()
                    .filter_map(|group| clip_ids[*group])
                    .collect();
                let style = style(&draft.flags, &mut next, draft.layer, clips);
                shapes.extend(draft.paint.map(|paint| Shape {
                    geometry,
                    paint,
                    style,
                }));
            }
        }
    }
    (shapes, world)
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
            count,
            fade,
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
