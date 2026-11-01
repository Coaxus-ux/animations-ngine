use std::path::PathBuf;

use px_core::{MAX_TONES, Rgb};

use super::Context;
use super::menus::{Entry, check, entry, separator};
use super::palette_edit::selected_ramp;
use crate::app::Intent;
use crate::assets::palettes;
use crate::domain::Document;
use crate::domain::palette_model::{PaletteModel, RampModel, palette_path};

/// Hue shift in degrees, contrast and desaturation (fractions) the menu's generator uses;
/// the palette editor (F4) has fields for them.
const GENERATOR: (f64, f64, f64) = (15.0, 0.4, 0.2);

pub(super) fn palette<'a>(context: &Context<'_>) -> Vec<Entry<'a>> {
    let open = |intent: Intent| context.document.map(|_| intent);
    let mut entries = vec![
        check(
            "Edit palette",
            "F4",
            context.palette_edit,
            open(Intent::TogglePaletteEdit),
        ),
        entry(
            "Add ramp",
            "",
            open(Intent::AddRamp {
                name: "ramp".to_owned(),
                colors: Vec::new(),
            }),
        ),
    ];
    entries.extend(ramp_entries(context));
    entries.extend([
        separator(),
        entry(
            "Import .hex/.gpl",
            "",
            open(Intent::ImportPalette(PathBuf::new())),
        ),
        entry(
            "Save palette as",
            "",
            open(Intent::SavePalette(PathBuf::new())),
        ),
        separator(),
        entry("Apply a preset:", "", None),
    ]);
    entries.extend(
        palettes::names().map(|name| entry(name, "", open(Intent::ApplyPreset(name.to_owned())))),
    );
    entries
}

/// Entries acting on the selected ramp; disabled without one.
fn ramp_entries<'a>(context: &Context<'_>) -> Vec<Entry<'a>> {
    let target = context
        .document
        .and_then(|document| selected(context, document));
    let ramp = target.as_ref().map(|(ramp, _)| ramp);
    let with_tones = |change: fn(&mut Vec<Rgb>)| {
        let ramp = ramp?;
        let mut colors = ramp.colors.clone();
        change(&mut colors);
        (colors.len() != ramp.colors.len()).then(|| Intent::SetRamp {
            ramp: ramp.name.clone(),
            name: ramp.name.clone(),
            colors,
        })
    };
    vec![
        entry(
            "Remove ramp",
            "",
            ramp.map(|ramp| Intent::RemoveRamp(ramp.name.clone())),
        ),
        entry(
            "Generate ramp",
            "",
            target.as_ref().map(|(ramp, base)| Intent::GenerateRamp {
                ramp: ramp.name.clone(),
                base: *base,
                steps: u8::try_from(ramp.colors.len()).unwrap_or(1),
                hue_shift: GENERATOR.0,
                contrast: GENERATOR.1,
                saturation: GENERATOR.2,
            }),
        ),
        entry(
            "Add a tone",
            "",
            with_tones(|colors| {
                if colors.len() < MAX_TONES {
                    colors.extend(colors.last().copied());
                }
            }),
        ),
        entry(
            "Drop the last tone",
            "",
            with_tones(|colors| {
                if colors.len() > 1 {
                    colors.pop();
                }
            }),
        ),
    ]
}

fn selected(context: &Context<'_>, document: &Document) -> Option<(RampModel, Rgb)> {
    let (name, base) = selected_ramp(context, document)?;
    let path = palette_path(document).ok()?;
    let model = PaletteModel::from_source(document.source(), &path)?;
    let ramp = model.ramps.into_iter().find(|ramp| ramp.name == name)?;
    Some((ramp, base))
}
