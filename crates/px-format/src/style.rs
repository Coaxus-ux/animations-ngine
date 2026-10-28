use px_core::Paint;
use px_engine::{Gradient, GradientDirection, Outline, Shading, Texture};

use crate::diagnostic::Diagnostics;
use crate::paints::Paints;
use crate::props::Props;
use crate::values;

#[derive(Clone, Debug, Default)]
pub(crate) struct Flags {
    pub(crate) emissive: bool,
    pub(crate) casts_shadow: bool,
    pub(crate) cleanup: bool,
    pub(crate) shading: Option<Shading>,
    pub(crate) outline: Option<Outline>,
    pub(crate) texture: Option<Texture>,
    pub(crate) gradient: Option<Gradient>,
}

fn flag(props: &mut Props<'_>, key: &'static str, diags: &mut Diagnostics) -> bool {
    props
        .get(key)
        .and_then(|entry| values::boolean(entry, diags))
        .unwrap_or(false)
}

fn choice<T: Copy>(
    props: &mut Props<'_>,
    key: &'static str,
    options: &[(&str, T)],
    diags: &mut Diagnostics,
) -> Option<T> {
    let entry = props.get(key)?;
    let text = values::string(entry, diags)?;
    let found = options
        .iter()
        .find(|(name, _)| *name == text)
        .map(|(_, value)| *value);
    if found.is_none() {
        let names: Vec<&str> = options.iter().map(|(name, _)| *name).collect();
        diags.error(
            entry.span().offset(),
            format!("`{key}` must be one of: {}", names.join(", ")),
        );
    }
    found
}

fn steps(props: &mut Props<'_>, key: &'static str, default: i8, diags: &mut Diagnostics) -> i8 {
    props
        .get(key)
        .and_then(|entry| values::bounded(entry, -8, 8, diags))
        .unwrap_or(default)
}

pub(crate) fn parse(props: &mut Props<'_>, palette: &Paints<'_>, diags: &mut Diagnostics) -> Flags {
    let shading_steps = u8::try_from(steps(props, "shading-steps", 1, diags).max(1)).unwrap_or(1);
    let shading = choice(
        props,
        "shading",
        &[("flat", 0), ("dome", 1), ("soft", 2)],
        diags,
    )
    .map(|kind| match kind {
        0 => Shading::Flat,
        soft => Shading::Dome {
            steps: shading_steps,
            soft: soft == 2,
        },
    });
    let outline_paint = props.get("outline-paint").and_then(|entry| {
        let text = values::string(entry, diags)?;
        palette.paint(text, entry.span().offset(), diags)
    });
    let outline = choice(
        props,
        "outline",
        &[("none", 0), ("full", 1), ("selective", 2)],
        diags,
    )
    .and_then(|kind| {
        let paint: Paint = outline_paint.or_else(|| palette.darkest())?;
        Some(match kind {
            0 => Outline::None,
            1 => Outline::Full(paint),
            _ => Outline::Selective(paint),
        })
    });
    let texture = props
        .get("texture")
        .and_then(|entry| values::number(entry, diags))
        .map(|amount| Texture {
            amount: amount.clamp(0.0, 1.0),
            scale: props
                .get("texture-scale")
                .and_then(|entry| values::number(entry, diags))
                .unwrap_or(3.0),
        });
    let directions = [
        ("down", GradientDirection::Down),
        ("up", GradientDirection::Up),
        ("left", GradientDirection::Left),
        ("right", GradientDirection::Right),
        ("radial", GradientDirection::Radial),
    ];
    let gradient_steps = steps(props, "gradient-steps", 2, diags);
    let gradient = choice(props, "gradient", &directions, diags).map(|direction| Gradient {
        direction,
        steps: gradient_steps,
    });
    Flags {
        emissive: flag(props, "emissive", diags),
        casts_shadow: flag(props, "casts-shadow", diags),
        cleanup: flag(props, "cleanup", diags),
        shading,
        outline,
        texture,
        gradient,
    }
}
