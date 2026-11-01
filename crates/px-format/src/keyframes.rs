use kdl::{KdlEntry, KdlNode, KdlValue};
use px_expr::{Blend, Ease, Key, Program, Track, TrackLoop};

use crate::diagnostic::Diagnostics;
use crate::props::Props;
use crate::units::{Measure, Quantity};
use crate::values;

const INTERPOLATIONS: [&str; 10] = [
    "step",
    "linear",
    "smooth",
    "ease-in",
    "ease-out",
    "ease-in-out",
    "bounce",
    "elastic",
    "hold-2",
    "hold-3",
];

#[derive(Clone, Copy)]
pub(crate) struct Anim<'a> {
    pub(crate) node: &'a KdlNode,
    pub(crate) multi: Option<&'static str>,
}

pub(crate) struct Animated {
    pub(crate) track: Program,
    pub(crate) blend: Blend,
    pub(crate) times: Vec<f64>,
}

fn choice<T: Copy>(
    props: &mut Props<'_>,
    key: &'static str,
    options: &[(&str, T)],
    default: T,
    diags: &mut Diagnostics,
) -> T {
    let Some(entry) = props.get(key) else {
        return default;
    };
    let text = values::string(entry, diags).unwrap_or_default();
    options.iter().find(|(name, _)| *name == text).map_or_else(
        || {
            let names: Vec<&str> = options.iter().map(|(name, _)| *name).collect();
            diags.error(
                entry.span().offset(),
                format!("`{key}` must be one of: {}", names.join(", ")),
            );
            default
        },
        |(_, value)| *value,
    )
}

pub(crate) fn parse(
    anim: Anim<'_>,
    quantity: Quantity,
    measure: Measure<'_>,
    diags: &mut Diagnostics,
) -> Option<Animated> {
    let (looping, blend) = if anim.multi.is_some() {
        (TrackLoop::None, Blend::Replace)
    } else {
        let mut props = Props::new(anim.node);
        let looping = choice(
            &mut props,
            "loop",
            &[
                ("none", TrackLoop::None),
                ("repeat", TrackLoop::Repeat),
                ("pingpong", TrackLoop::PingPong),
            ],
            TrackLoop::None,
            diags,
        );
        let blend = choice(
            &mut props,
            "blend",
            &[
                ("replace", Blend::Replace),
                ("add", Blend::Add),
                ("multiply", Blend::Multiply),
            ],
            Blend::Replace,
            diags,
        );
        props.finish(diags);
        (looping, blend)
    };
    let mut keys = Vec::new();
    let mut previous = 0.0;
    let nodes = anim
        .node
        .children()
        .map(kdl::KdlDocument::nodes)
        .unwrap_or_default();
    for node in nodes.iter().filter(|node| node.name().value() == "k") {
        if let Some(key) = key(node, anim.multi, previous, quantity, measure, diags) {
            previous = key.time;
            keys.push(key);
        }
    }
    if anim.multi.is_none() {
        for node in nodes.iter().filter(|node| node.name().value() != "k") {
            diags.error(
                node.span().offset(),
                format!(
                    "`anim` blocks only contain `k` keys, found `{}`",
                    node.name().value()
                ),
            );
        }
    }
    let times = keys.iter().map(|key| key.time).collect();
    let track = Track::new(keys, looping);
    if track.is_none() && anim.multi.is_none() {
        diags.error(
            anim.node.span().offset(),
            "`anim` needs at least one valid key: k \"0s\" 10",
        );
    }
    Some(Animated {
        track: Program::track(track?),
        blend,
        times,
    })
}

fn key(
    node: &KdlNode,
    multi: Option<&'static str>,
    previous: f64,
    quantity: Quantity,
    measure: Measure<'_>,
    diags: &mut Diagnostics,
) -> Option<Key> {
    let mut props = Props::new(node);
    let mut args = props.args();
    let time_entry = args.next();
    let value_entry = match multi {
        Some(name) => props.get(name),
        None => args.next(),
    };
    let interp = props.get("interp");
    if multi.is_none() {
        props.finish(diags);
    }
    let Some(value_entry) = value_entry else {
        if multi.is_none() {
            diags.error(
                node.span().offset(),
                "a key needs a time and a value: k \"1s\" 10",
            );
        }
        return None;
    };
    let Some(time_entry) = time_entry else {
        diags.error(node.span().offset(), "a key needs a time: k \"1s\" …");
        return None;
    };
    let time = time(time_entry, previous, measure, diags)?;
    let value = value(value_entry, quantity, measure, diags)?;
    let (ease, hold) =
        interp.map_or(Some((Ease::Linear, 0)), |entry| interpolation(entry, diags))?;
    Some(Key {
        time,
        value,
        ease,
        hold,
    })
}

fn time(
    entry: &KdlEntry,
    previous: f64,
    measure: Measure<'_>,
    diags: &mut Diagnostics,
) -> Option<f64> {
    let text = values::string(entry, diags)?;
    let (relative, text) = text
        .strip_prefix('+')
        .map_or((false, text), |rest| (true, rest));
    let seconds = measure
        .literal(text)
        .ok_or_else(|| format!("invalid key time \"{text}\""))
        .and_then(|literal| literal.convert(Quantity::Time, measure));
    match seconds {
        Ok(seconds) => Some(if relative {
            previous + seconds
        } else {
            seconds
        }),
        Err(message) => {
            diags.error(
                entry.span().offset(),
                format!("{message}; use \"0.5s\", \"6f\" or \"+1s\""),
            );
            None
        }
    }
}

fn value(
    entry: &KdlEntry,
    quantity: Quantity,
    measure: Measure<'_>,
    diags: &mut Diagnostics,
) -> Option<f64> {
    let KdlValue::String(text) = entry.value() else {
        return values::number(entry, diags);
    };
    let converted = measure
        .literal(text)
        .ok_or_else(|| format!("expected a number like 10 or \"50%\", found \"{text}\""))
        .and_then(|literal| literal.convert(quantity, measure));
    converted
        .map_err(|message| diags.error(entry.span().offset(), message))
        .ok()
}

fn interpolation(entry: &KdlEntry, diags: &mut Diagnostics) -> Option<(Ease, u8)> {
    let text = values::string(entry, diags)?;
    let parsed = match text {
        "hold-2" => Some((Ease::Linear, 2)),
        "hold-3" => Some((Ease::Linear, 3)),
        other => Ease::parse(other).map(|ease| (ease, 0)),
    };
    if parsed.is_none() {
        let hint = crate::suggest::did_you_mean(text, INTERPOLATIONS);
        diags.error(
            entry.span().offset(),
            format!("unknown interp \"{text}\".{hint}"),
        );
    }
    parsed
}
