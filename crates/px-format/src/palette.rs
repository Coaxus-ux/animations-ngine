use std::collections::BTreeMap;

use kdl::{KdlDocument, KdlNode};
use px_core::{Paint, Palette, RampId, Rgb, Tone, ToneName};

use crate::diagnostic::Diagnostics;
use crate::props::Props;
use crate::values;

/// A palette plus the names of its ramps.
#[derive(Clone, Debug, Default)]
pub(crate) struct NamedPalette {
    pub(crate) palette: Palette,
    ramps: BTreeMap<String, RampId>,
}

/// Parses `palette "name" { ramp "sky" "#112233" "#445566" … }`.
pub(crate) fn parse(node: &KdlNode, diags: &mut Diagnostics) -> Option<(String, NamedPalette)> {
    let props = Props::new(node);
    let name = props.name_arg(diags)?;
    let mut named = NamedPalette::default();
    for child in node.children().map(KdlDocument::nodes).unwrap_or_default() {
        if child.name().value() == "ramp" {
            named.add_ramp(child, diags);
        } else {
            diags.error(
                child.span().offset(),
                format!(
                    "unknown node `{}` in palette (expected `ramp`)",
                    child.name().value()
                ),
            );
        }
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

    /// Resolves `ramp`, `ramp.tone-name` or `ramp.index` (`docs/PALETTES.md` §1.2).
    pub(crate) fn paint(
        &self,
        text: &str,
        offset: usize,
        diags: &mut Diagnostics,
    ) -> Option<Paint> {
        let (ramp_name, tone_text) = text.split_once('.').unwrap_or((text, "base"));
        let Some(&ramp_id) = self.ramps.get(ramp_name) else {
            let known: Vec<&str> = self.ramps.keys().map(String::as_str).collect();
            diags.error(
                offset,
                format!("unknown ramp \"{ramp_name}\" (known: {})", known.join(", ")),
            );
            return None;
        };
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
}
