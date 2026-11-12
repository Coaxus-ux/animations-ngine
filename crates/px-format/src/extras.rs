use px_core::{LightColor, Paint, RampId};
use px_engine::{Cel, MAX_PARTICLES, TextCell};

use crate::diagnostic::Diagnostics;
use crate::font::DEFAULT_FONT;
use crate::markup::{self, Style};
use crate::names::Names;
use crate::params::{ParamValue, value_of};
use crate::props::Props;
use crate::schema::Kind;
use crate::suggest;
use crate::text::{Align, layout};
use crate::values;

#[derive(Clone, Debug, Default)]
pub(crate) enum Extras {
    #[default]
    None,
    Points(Vec<[f64; 2]>),
    Circles(Vec<[f64; 3]>),
    Text(Vec<TextCell>),
    Pixels(Vec<Cel>),
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
    palette: &Names<'_>,
    diags: &mut Diagnostics,
) -> Option<Paint> {
    let entry = props.get(key)?;
    let text = values::string(entry, diags)?;
    palette.paint(text, entry.span().offset(), diags)
}

pub(crate) fn parse(
    props: &mut Props<'_>,
    kind: Kind,
    palette: &Names<'_>,
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
        Kind::Text => text(props, palette, diags),
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
        Kind::UseScene => placement(props, palette, diags),
        _ => Extras::None,
    }
}

fn text(props: &mut Props<'_>, names: &Names<'_>, diags: &mut Diagnostics) -> Extras {
    let font = props
        .get("font")
        .map_or(Some(DEFAULT_FONT), |entry| values::string(entry, diags));
    let font = font.and_then(|name| {
        let font = names.fonts.get(name);
        if font.is_none() {
            let hint = suggest::did_you_mean(name, names.fonts.keys().map(String::as_str));
            diags.error(props.offset(), format!("unknown font \"{name}\".{hint}"));
        }
        font.map(|font| (name, font))
    });
    let words = props.args().next();
    if words.is_none() {
        diags.error(
            props.offset(),
            "`text` needs its words first: text \"Hello\" x=0 y=0 paint=\"ink\"",
        );
    }
    let content = words.and_then(|entry| {
        let text = values::string(entry, diags)?;
        let Some(param) = text.strip_prefix('$') else {
            return Some(text);
        };
        names
            .params
            .text(param)
            .map_err(|message| diags.error(entry.span().offset(), message))
            .ok()
    });
    let mut flag = |key| {
        props
            .get(key)
            .and_then(|entry| values::boolean(entry, diags))
            .unwrap_or(false)
    };
    let style = Style {
        bold: flag("bold"),
        italic: flag("italic"),
        ..Style::default()
    };
    let align = props.get("align").map_or(Some(Align::Left), |entry| {
        let name = values::string(entry, diags)?;
        let align = Align::parse(name);
        if align.is_none() {
            let hint = suggest::did_you_mean(name, Align::NAMES);
            diags.error(
                entry.span().offset(),
                format!("`align` is left, center or right.{hint}"),
            );
        }
        align
    });
    let (Some((name, font)), Some(content), Some(align)) = (font, content, align) else {
        return Extras::Text(Vec::new());
    };
    let laid = layout(font, &markup::styled(content, style), align);
    if !laid.missing.is_empty() {
        let missing: String = laid.missing.iter().collect();
        diags.warning(
            props.offset(),
            format!("font \"{name}\" has no glyph for \"{missing}\"; drawn as boxes"),
        );
    }
    Extras::Text(laid.cells)
}

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

pub(crate) fn overrides(props: &mut Props<'_>, diags: &mut Diagnostics) -> Vec<Override> {
    props
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
        .collect()
}

fn forwarded(item: Override, names: &Names<'_>, diags: &mut Diagnostics) -> Option<Override> {
    let ParamValue::Text(text) = &item.value else {
        return Some(item);
    };
    let Some(name) = text.strip_prefix('$') else {
        return Some(item);
    };
    match names.params.value(name) {
        Ok(value) => Some(Override {
            value: value.clone(),
            ..item
        }),
        Err(message) => {
            diags.error(item.offset, message);
            None
        }
    }
}

fn placement(props: &mut Props<'_>, names: &Names<'_>, diags: &mut Diagnostics) -> Extras {
    let scene = props.name_arg(diags).unwrap_or_default().to_owned();
    props.get("id");
    let looping = props
        .get("loop")
        .and_then(|entry| values::boolean(entry, diags));
    let overrides = overrides(props, diags)
        .into_iter()
        .filter_map(|item| forwarded(item, names, diags))
        .collect();
    Extras::Instance(Box::new(Placement {
        scene,
        looping,
        overrides,
    }))
}
