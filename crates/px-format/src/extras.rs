use px_core::{LightColor, Paint, RampId};
use px_engine::MAX_PARTICLES;

use crate::diagnostic::Diagnostics;
use crate::paints::Paints;
use crate::params::{ParamValue, value_of};
use crate::props::Props;
use crate::schema::Kind;
use crate::values;

/// Non-animatable settings that only some kinds have.
#[derive(Clone, Debug, Default)]
pub(crate) enum Extras {
    #[default]
    None,
    Points(Vec<[f64; 2]>),
    Circles(Vec<[f64; 3]>),
    Particles {
        count: u16,
        fade: bool,
    },
    Shade {
        round: bool,
    },
    Light {
        color: Option<LightColor>,
        shadows: bool,
    },
    Group {
        clip: bool,
        round: bool,
    },
    Layer {
        parallax: f64,
        fog_paint: Option<Paint>,
    },
    Ambient {
        occlusion: f64,
        key_light: f64,
    },
    Reflection {
        surface: Option<RampId>,
    },
    Instance(Box<Placement>),
}

/// What a `use-scene` places: the scene, whether it loops, and its param overrides.
#[derive(Clone, Debug)]
pub(crate) struct Placement {
    pub(crate) scene: String,
    pub(crate) looping: Option<bool>,
    pub(crate) overrides: Vec<Override>,
}

#[derive(Clone, Debug)]
pub(crate) struct Override {
    pub(crate) name: String,
    pub(crate) value: ParamValue,
    pub(crate) offset: usize,
}

pub(crate) fn number(
    props: &mut Props<'_>,
    key: &'static str,
    default: f64,
    diags: &mut Diagnostics,
) -> f64 {
    props
        .get(key)
        .and_then(|entry| values::number(entry, diags))
        .unwrap_or(default)
}

pub(crate) fn flag(props: &mut Props<'_>, key: &'static str, diags: &mut Diagnostics) -> bool {
    props
        .get(key)
        .and_then(|entry| values::boolean(entry, diags))
        .unwrap_or(false)
}

fn paint(
    props: &mut Props<'_>,
    key: &'static str,
    palette: &Paints<'_>,
    diags: &mut Diagnostics,
) -> Option<Paint> {
    let entry = props.get(key)?;
    let text = values::string(entry, diags)?;
    palette.paint(text, entry.span().offset(), diags)
}

pub(crate) fn parse(
    props: &mut Props<'_>,
    kind: Kind,
    palette: &Paints<'_>,
    diags: &mut Diagnostics,
) -> Extras {
    match kind {
        Kind::Polygon => Extras::Points(tuples(props, "points", 3, "\"0,0 8,0 4,6\"", diags)),
        Kind::Blob => Extras::Circles(tuples(props, "circles", 1, "\"10,10,4 16,12,5\"", diags)),
        Kind::Particles => Extras::Particles {
            count: props
                .get("count")
                .and_then(|entry| values::bounded(entry, 0, MAX_PARTICLES, diags))
                .unwrap_or(20),
            fade: flag(props, "fade", diags),
        },
        Kind::Shade => Extras::Shade {
            round: flag(props, "round", diags),
        },
        Kind::LightPoint | Kind::LightCone | Kind::LightRect => Extras::Light {
            shadows: flag(props, "shadows", diags),
            color: props.get("color").and_then(|entry| {
                let name = values::string(entry, diags)?;
                palette.light_color(name, entry.span().offset(), diags)
            }),
        },
        Kind::Group => Extras::Group {
            clip: flag(props, "clip", diags),
            round: flag(props, "round", diags),
        },
        Kind::Layer => Extras::Layer {
            parallax: number(props, "parallax", 1.0, diags),
            fog_paint: paint(props, "fog-paint", palette, diags),
        },
        Kind::Ambient => Extras::Ambient {
            occlusion: number(props, "occlusion", 0.0, diags).clamp(0.0, 4.0),
            key_light: number(props, "key-light", 225.0, diags),
        },
        Kind::Reflection => Extras::Reflection {
            surface: props.get("on").and_then(|entry| {
                let name = values::string(entry, diags)?;
                palette.ramp(name, entry.span().offset(), diags)
            }),
        },
        Kind::UseScene => placement(props, diags),
        _ => Extras::None,
    }
}

/// Parses `"a,b a,b …"` into groups of `N` numbers.
fn tuples<const N: usize>(
    props: &mut Props<'_>,
    key: &'static str,
    minimum: usize,
    example: &str,
    diags: &mut Diagnostics,
) -> Vec<[f64; N]> {
    let Some(entry) = props.require(key, diags) else {
        return Vec::new();
    };
    let Some(text) = values::string(entry, diags) else {
        return Vec::new();
    };
    let parsed: Option<Vec<[f64; N]>> = text
        .split_whitespace()
        .map(|group| {
            let numbers: Option<Vec<f64>> = group
                .split(',')
                .map(|value| value.trim().parse::<f64>().ok().filter(|v| v.is_finite()))
                .collect();
            numbers?.try_into().ok()
        })
        .collect();
    match parsed {
        Some(groups) if groups.len() >= minimum => groups,
        _ => {
            diags.error(
                entry.span().offset(),
                format!("`{key}` needs at least {minimum} groups of {N} numbers, like {example}"),
            );
            Vec::new()
        }
    }
}

fn placement(props: &mut Props<'_>, diags: &mut Diagnostics) -> Extras {
    let scene = props.name_arg(diags).unwrap_or_default().to_owned();
    props.get("id");
    let looping = props
        .get("loop")
        .and_then(|entry| values::boolean(entry, diags));
    let overrides = props
        .rest()
        .into_iter()
        .filter_map(|entry| {
            let name = entry.name()?.value().to_owned();
            let value = value_of(entry.value());
            if value.is_none() {
                diags.error(
                    entry.span().offset(),
                    format!("param `{name}` needs a number, a bool or text"),
                );
            }
            Some(Override {
                name,
                value: value?,
                offset: entry.span().offset(),
            })
        })
        .collect();
    Extras::Instance(Box::new(Placement {
        scene,
        looping,
        overrides,
    }))
}
