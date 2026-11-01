mod preset;
mod text;

use std::path::Path;

use px_core::{MAX_TONES, Rgb, hsv_ramp};
use px_format::edit::Source;
use px_ui::Input;

use super::dialog::PickPurpose;
use super::intent::Intent;
use super::window::Request;
use super::{App, Targets, WindowId};
use crate::assets::palettes;
use crate::domain::Command;
use crate::domain::palette_import::ramps_by_hue;
use crate::domain::palette_model::palette_path;
use crate::domain::palette_model::{PaletteModel, RampModel, ramp_kdl};
use crate::ports::FileStore;
use text::{parse_palette_file, unique_name, with_ramp_lines};

const IMPORT_FILTERS: &[(&str, &[&str])] = &[("Palettes", &["hex", "gpl"])];
const KDL_FILTERS: &[(&str, &[&str])] = &[("KDL palette", &["kdl"])];
const NO_PRIMARY: Rgb = Rgb::new(0x80, 0x80, 0x80);

impl App {
    pub(super) fn apply_palette(
        &mut self,
        intent: Intent,
        targets: Targets,
        _input: &Input,
        store: &mut dyn FileStore,
    ) -> Option<Intent> {
        let window = targets.source;
        match intent {
            Intent::TogglePaletteEdit => {
                self.window_mut(window, |window| window.palette_edit = !window.palette_edit);
            }
            Intent::AddRamp { name, colors } => self.add_ramp(&name, colors, store),
            Intent::RemoveRamp(name) => self.remove_ramp(&name, store),
            Intent::SetRamp { ramp, name, colors } => self.set_ramp(&ramp, name, colors, store),
            Intent::GenerateRamp {
                ramp,
                base,
                steps,
                hue_shift,
                contrast,
                saturation,
            } => {
                let steps = steps.clamp(1, u8::try_from(MAX_TONES).unwrap_or(u8::MAX));
                let colors = hsv_ramp(base, steps, hue_shift, contrast, saturation)
                    [..usize::from(steps)]
                    .to_vec();
                self.set_ramp(&ramp, ramp.clone(), colors, store);
            }
            Intent::ImportPalette(path) if path.as_os_str().is_empty() => {
                self.pick_palette_file(window, PickPurpose::ImportPalette, None);
            }
            Intent::ImportPalette(path)
            | Intent::Picked {
                purpose: PickPurpose::ImportPalette,
                path: Some(path),
            } => self.import_palette(&path, store),
            Intent::SavePalette(path) if path.as_os_str().is_empty() => {
                match self
                    .palette()
                    .map(|(_, model)| format!("{}.kdl", model.name))
                {
                    Ok(name) => {
                        self.pick_palette_file(window, PickPurpose::SavePalette, Some(name));
                    }
                    Err(status) => self.editing.status = status,
                }
            }
            Intent::SavePalette(path)
            | Intent::Picked {
                purpose: PickPurpose::SavePalette,
                path: Some(path),
            } => self.save_palette(&path, store),
            Intent::ApplyPreset(name) => self.apply_preset(&name, store),
            other => return Some(other),
        }
        None
    }

    fn palette(&self) -> Result<(&Source, PaletteModel), String> {
        let document = self.document().ok_or("Open a scene first")?;
        let path = palette_path(document)?;
        let source = document.source();
        let model = PaletteModel::from_source(source, &path)
            .ok_or_else(|| format!("Palette node {path:?} could not be read"))?;
        Ok((source, model))
    }

    fn edit_palette(
        &mut self,
        store: &mut dyn FileStore,
        plan: impl FnOnce(&Source, &PaletteModel) -> Result<Command, String>,
    ) -> bool {
        let planned = self
            .palette()
            .and_then(|(source, model)| plan(source, &model));
        match planned {
            Ok(command) => {
                self.run(&command, store);
                self.editing.status.starts_with("Done")
            }
            Err(status) => {
                self.editing.status = status;
                false
            }
        }
    }

    fn primary_color(&self) -> Rgb {
        self.editing
            .primary
            .and_then(|paint| self.document()?.scene()?.palette().resolve(paint).ok())
            .unwrap_or(NO_PRIMARY)
    }

    fn add_ramp(&mut self, name: &str, mut colors: Vec<Rgb>, store: &mut dyn FileStore) {
        if colors.is_empty() {
            colors.push(self.primary_color());
        }
        colors.truncate(MAX_TONES);
        self.edit_palette(store, |_, model| {
            Ok(Command::AddRamp {
                palette: model.path.clone(),
                name: unique_name(name, &names_of(model)),
                colors,
            })
        });
    }

    fn remove_ramp(&mut self, name: &str, store: &mut dyn FileStore) {
        self.edit_palette(store, |source, model| {
            let ramp = find_ramp(model, name)?;
            if model.ramps.len() <= 1 {
                return Err("The palette needs at least one ramp".to_owned());
            }
            refuse_if_used(source, name)?;
            Ok(Command::RemoveRamp {
                ramp: ramp.path.clone(),
            })
        });
    }

    fn set_ramp(&mut self, ramp: &str, name: String, colors: Vec<Rgb>, store: &mut dyn FileStore) {
        self.edit_palette(store, |source, model| {
            let current = find_ramp(model, ramp)?;
            if colors.is_empty() || colors.len() > MAX_TONES {
                return Err(format!("A ramp has 1 to {MAX_TONES} tones"));
            }
            refuse_if_shortened(source, model, ramp, &colors)?;
            if name != ramp {
                if name.is_empty() {
                    return Err("A ramp needs a name".to_owned());
                }
                if names_of(model).contains(&name) {
                    return Err(format!("Ramp \"{name}\" already exists"));
                }
                refuse_if_used(source, ramp)?;
            }
            Ok(Command::SetRamp {
                ramp: current.path.clone(),
                name,
                colors,
            })
        });
    }

    fn import_palette(&mut self, path: &Path, store: &mut dyn FileStore) {
        let colors = match store
            .read(path)
            .and_then(|text| parse_palette_file(path, &text))
        {
            Ok(colors) => colors,
            Err(error) => {
                self.editing.status = format!("Could not import {}: {error}", path.display());
                return;
            }
        };
        let imported = ramps_by_hue(&colors, MAX_TONES);
        let count = imported.len();
        let done = self.edit_palette(store, |source, model| {
            let mut taken = names_of(model);
            let lines = imported
                .iter()
                .fold(String::new(), |mut lines, (wanted, colors)| {
                    let name = unique_name(wanted, &taken);
                    lines.push_str("    ");
                    lines.push_str(&ramp_kdl(&name, colors));
                    lines.push('\n');
                    taken.push(name);
                    lines
                });
            let text = source
                .node_text(&model.path)
                .ok_or("The palette node vanished")?;
            Ok(Command::ReplaceNode {
                path: model.path.clone(),
                text: with_ramp_lines(&text, &lines),
            })
        });
        if done {
            self.editing.status = format!(
                "Imported {count} ramp{} from {}",
                if count == 1 { "" } else { "s" },
                path.display()
            );
        }
    }

    fn save_palette(&mut self, path: &Path, store: &mut dyn FileStore) {
        self.editing.status = match self.palette() {
            Ok((_, model)) => match store.write(path, &model.to_kdl()) {
                Ok(()) => format!("Saved palette \"{}\" to {}", model.name, path.display()),
                Err(error) => format!("Could not save the palette: {error}"),
            },
            Err(status) => status,
        };
    }

    fn apply_preset(&mut self, name: &str, store: &mut dyn FileStore) {
        let preset = match palettes::preset(name)
            .ok_or_else(|| format!("No preset named \"{name}\""))
            .and_then(|text| preset::parse(name, text))
        {
            Ok(preset) => preset,
            Err(status) => {
                self.editing.status = status;
                return;
            }
        };
        self.edit_palette(store, |source, model| {
            Ok(Command::ReplaceNode {
                path: model.path.clone(),
                text: preset::node_text(source, model, &preset)?,
            })
        });
    }

    fn pick_palette_file(&mut self, window: WindowId, purpose: PickPurpose, name: Option<String>) {
        let filters = match purpose {
            PickPurpose::SavePalette => KDL_FILTERS,
            _ => IMPORT_FILTERS,
        };
        self.requests.push(Request::PickFile {
            window,
            purpose,
            start: self.last_folder.clone(),
            name,
            filters,
        });
    }
}

fn names_of(model: &PaletteModel) -> Vec<String> {
    model.ramps.iter().map(|ramp| ramp.name.clone()).collect()
}

fn find_ramp<'a>(model: &'a PaletteModel, name: &str) -> Result<&'a RampModel, String> {
    model
        .ramps
        .iter()
        .find(|ramp| ramp.name == name)
        .ok_or_else(|| format!("Palette \"{}\" has no ramp \"{name}\"", model.name))
}

fn refuse_if_used(source: &Source, ramp: &str) -> Result<(), String> {
    match PaletteModel::uses_of(source, ramp) {
        Some(user) => Err(format!("Ramp \"{ramp}\" is used by {user}")),
        None => Ok(()),
    }
}

fn refuse_if_shortened(
    source: &Source,
    model: &PaletteModel,
    ramp: &str,
    colors: &[Rgb],
) -> Result<(), String> {
    let current = find_ramp(model, ramp)?.colors.len();
    if colors.len() < current
        && let Some((tone, user)) = PaletteModel::uses_of_tones(source, ramp, colors.len())
    {
        return Err(format!("Tone {tone} of ramp \"{ramp}\" is used by {user}"));
    }
    if colors.len() != current
        && let Some(twin) = PaletteModel::twin_partners(source, ramp)
            .into_iter()
            .find(|twin| find_ramp(model, twin).is_ok_and(|twin| twin.colors.len() != colors.len()))
    {
        return Err(format!(
            "Ramp \"{ramp}\" and its twin \"{twin}\" must have the same number of tones"
        ));
    }
    Ok(())
}
