use std::collections::BTreeMap;

use kdl::{KdlDocument, KdlNode};
use px_core::{LightColor, MAX_LIGHT_COLORS, Paint, Palette, RampId, Rgb, Tone, ToneName};

use crate::diagnostic::Diagnostics;
use crate::props::Props;
use crate::suggest;
use crate::values;

#[derive(Clone, Debug, Default)]
pub(crate) struct NamedPalette {
    pub(crate) palette: Palette,
    ramps: BTreeMap<String, RampId>,
    light_colors: BTreeMap<String, LightColor>,
}

pub(crate) fn parse(node: &KdlNode, diags: &mut Diagnostics) -> Option<(String, NamedPalette)> {
    let props = Props::new(node);
    let name = props.name_arg(diags)?;
    let mut named = NamedPalette::default();
    let children = node.children().map(KdlDocument::nodes).unwrap_or_default();
    for child in children {
        match child.name().value() {
            "ramp" => named.add_ramp(child, diags),
            "twin" => {}
            other => diags.error(
                child.span().offset(),
                format!(
                    "unknown node `{other}` in palette (expected `ramp` or `twin`).{}",
                    suggest::did_you_mean(other, ["ramp", "twin"])
                ),
            ),
        }
    }
    for child in children
        .iter()
        .filter(|child| child.name().value() == "twin")
    {
        named.add_twins(child, diags);
    }
    props.finish(diags);
    if named.ramps.is_empty() {
        diags.error(
            node.span().offset(),
            format!("palette \"{name}\" has no ramps"),
        );
    }
    Some((name.to_owned(), named))
}

impl NamedPalette {
    fn add_ramp(&mut self, node: &KdlNode, diags: &mut Diagnostics) {
        let props = Props::new(node);
        let mut args = props.args();
        let Some(name) = args.next().and_then(|entry| values::string(entry, diags)) else {
            return;
        };
        let colors: Vec<Rgb> = args
            .filter_map(|entry| {
                let text = values::string(entry, diags)?;
                Rgb::parse_hex(text)
                    .map_err(|error| {
                        diags.error(entry.span().offset(), format!("color \"{text}\": {error}"));
                    })
                    .ok()
            })
            .collect();
        props.finish(diags);
        if self.ramps.contains_key(name) {
            diags.error(
                node.span().offset(),
                format!("ramp \"{name}\" is defined twice"),
            );
            return;
        }
        match self.palette.push_ramp(&colors) {
            Ok(id) => {
                self.ramps.insert(name.to_owned(), id);
            }
            Err(error) => diags.error(node.span().offset(), format!("ramp \"{name}\": {error}")),
        }
    }

    pub(crate) fn paint(
        &self,
        text: &str,
        offset: usize,
        diags: &mut Diagnostics,
    ) -> Option<Paint> {
        let (ramp_name, tone_text) = text.split_once('.').unwrap_or((text, "base"));
        let ramp_id = self.ramp(ramp_name, offset, diags)?;
        let ramp = self.palette.ramp(ramp_id).ok()?;
        let tone = ToneName::parse(tone_text)
            .and_then(|name| ramp.named(name))
            .or_else(|| tone_text.parse::<u8>().ok().map(Tone::new));
        let paint = tone.and_then(|tone| self.palette.paint(ramp_id, tone).ok());
        if paint.is_none() {
            diags.error(
                offset,
                format!("invalid tone \"{tone_text}\" for ramp \"{ramp_name}\" ({} tones; use deep, shade, base, lit, hi or 0..{})", ramp.len(), ramp.len().saturating_sub(1)),
            );
        }
        paint
    }

    fn add_twins(&mut self, node: &KdlNode, diags: &mut Diagnostics) {
        let offset = node.span().offset();
        let Some(base) = node
            .entries()
            .first()
            .and_then(|entry| entry.value().as_string())
        else {
            diags.error(
                offset,
                "`twin` needs the base ramp: twin \"wall\" warm=\"wall-warm\"",
            );
            return;
        };
        let Some(ramp) = self.ramp(base, offset, diags) else {
            return;
        };
        for entry in node.entries().iter().skip(1) {
            let (Some(color), Some(target)) = (
                entry.name().map(kdl::KdlIdentifier::value),
                entry.value().as_string(),
            ) else {
                diags.error(
                    entry.span().offset(),
                    "twins are written as light-color=\"ramp\"",
                );
                continue;
            };
            let color = self.register_color(color, offset, diags);
            let twin = self.ramp(target, offset, diags);
            let (Some(color), Some(twin)) = (color, twin) else {
                continue;
            };
            if let Err(error) = self.palette.set_twin(ramp, color, twin) {
                diags.error(entry.span().offset(), format!("twin \"{target}\": {error}"));
            }
        }
    }

    fn register_color(
        &mut self,
        name: &str,
        offset: usize,
        diags: &mut Diagnostics,
    ) -> Option<LightColor> {
        if let Some(color) = self.light_colors.get(name) {
            return Some(*color);
        }
        let Some(color) = u8::try_from(self.light_colors.len())
            .ok()
            .and_then(LightColor::new)
        else {
            diags.error(
                offset,
                format!("a palette can have at most {MAX_LIGHT_COLORS} light colors"),
            );
            return None;
        };
        self.light_colors.insert(name.to_owned(), color);
        Some(color)
    }

    pub(crate) fn light_color(
        &self,
        name: &str,
        offset: usize,
        diags: &mut Diagnostics,
    ) -> Option<LightColor> {
        let found = self.light_colors.get(name).copied();
        if found.is_none() {
            let hint = suggest::hint(name, self.light_colors.keys().map(String::as_str));
            diags.error(
                offset,
                format!(
                    "unknown light color \"{name}\"; declare it with `twin` in the palette.{hint}"
                ),
            );
        }
        found
    }

    pub(crate) fn ramp(
        &self,
        name: &str,
        offset: usize,
        diags: &mut Diagnostics,
    ) -> Option<RampId> {
        let found = self.ramps.get(name).copied();
        if found.is_none() {
            let hint = suggest::hint(name, self.ramps.keys().map(String::as_str));
            diags.error(offset, format!("unknown ramp \"{name}\".{hint}"));
        }
        found
    }

    pub(crate) fn darkest(&self) -> Option<Paint> {
        let luminance =
            |rgb: Rgb| u32::from(rgb.r) * 299 + u32::from(rgb.g) * 587 + u32::from(rgb.b) * 114;
        self.ramps
            .values()
            .filter_map(|ramp| {
                self.palette
                    .ramp(*ramp)
                    .ok()
                    .map(|colors| (*ramp, colors.colors()[0]))
            })
            .min_by_key(|(_, rgb)| luminance(*rgb))
            .map(|(ramp, _)| Paint {
                ramp,
                tone: Tone::new(0),
            })
    }
}
