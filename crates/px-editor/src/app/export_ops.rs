use std::path::Path;

use px_ui::Input;

use super::dialog::{Dialog, DialogKind, Form, PickPurpose, keys};
use super::intent::Intent;
use super::window::Request;
use super::{App, Targets};
use crate::app::Status;
use crate::domain::Document;
use crate::domain::export::{self, ExportJob, ExportKind, FrameScope, layout_count, layout_index};
use crate::ports::FileStore;

const GIF_FILTER: &[(&str, &[&str])] = &[("GIF image", &["gif"])];
const PNG_FILTER: &[(&str, &[&str])] = &[("PNG image", &["png"])];

impl App {
    pub(super) fn export_dialog(&self, sheet: bool) -> Dialog {
        let mut job = self.last_export.clone().unwrap_or_default();
        if sheet && !matches!(job.kind, ExportKind::Sheet { .. }) {
            job.kind = ExportJob::default().kind;
        }
        let mut dialog = Dialog::new(DialogKind::Export);
        fill_form(&mut dialog.form, &job);
        *dialog.form.text(keys::OUT, "") = self
            .document()
            .and_then(|document| {
                Some(
                    document
                        .path()
                        .with_file_name(file_name(document, job.kind)?),
                )
            })
            .map_or_else(String::new, |path| path.display().to_string());
        dialog
    }

    pub(super) fn apply_export(
        &mut self,
        intent: Intent,
        targets: Targets,
        _input: &Input,
        store: &mut dyn FileStore,
    ) -> Option<Intent> {
        match intent {
            Intent::Export(job) if job.out.as_os_str().is_empty() => {
                self.request_output_path(targets, &job);
            }
            Intent::Export(job) => {
                self.export(&job, targets, store);
                self.last_export = Some(job);
            }
            Intent::RepeatExport => self.repeat_export(targets, store),
            Intent::Picked {
                purpose: PickPurpose::ExportOut,
                path: Some(path),
            } => {
                self.window_mut(targets.source, |window| {
                    if let Some(dialog) = &mut window.dialog {
                        *dialog.form.text(keys::OUT, "") = path.display().to_string();
                    }
                });
            }
            other => return Some(other),
        }
        None
    }

    fn repeat_export(&mut self, targets: Targets, store: &mut dyn FileStore) {
        let Some(mut job) = self.last_export.clone() else {
            self.editing.status = Status::problem("Nothing exported yet");
            return;
        };
        match self
            .document()
            .and_then(|document| file_name(document, job.kind))
        {
            Some(name) => {
                job.out = job.out.with_file_name(name);
                self.export(&job, targets, store);
            }
            None if self.document().is_some() => {
                let dialog = self.export_dialog(false);
                self.window_mut(targets.source, |window| {
                    window.dialog = Some(dialog);
                    window.ui.focus = None;
                });
            }
            None => self.export(&job, targets, store),
        }
    }

    fn request_output_path(&mut self, targets: Targets, job: &ExportJob) {
        let name = self
            .document()
            .and_then(|document| file_name(document, job.kind));
        let start = self.last_folder.clone().or_else(|| {
            self.document()
                .and_then(|document| document.path().parent())
                .map(Path::to_path_buf)
        });
        self.requests.push(Request::PickFile {
            window: targets.source,
            purpose: PickPurpose::ExportOut,
            start,
            name,
            filters: match job.kind {
                ExportKind::Gif => GIF_FILTER,
                _ => PNG_FILTER,
            },
        });
    }

    fn export(&mut self, job: &ExportJob, targets: Targets, store: &mut dyn FileStore) {
        let hidden = self
            .window(targets.canvas)
            .map(|window| window.view.hidden.clone())
            .unwrap_or_default();
        let Some(document) = self.document() else {
            self.editing.status = Status::problem("Nothing to export: open a scene first");
            return;
        };
        let written = export::run(job, document, &hidden).and_then(|outputs| {
            for output in &outputs {
                store.write_bytes(&output.path, &output.bytes)?;
            }
            Ok(outputs.len())
        });
        self.editing.status = Status::from_result(written.map(|count| match count {
            1 => format!("Exported 1 file to {}", folder_of(&job.out)),
            count => format!("Exported {count} files to {}", folder_of(&job.out)),
        }));
    }
}

fn file_name(document: &Document, kind: ExportKind) -> Option<String> {
    if document.is_untitled() {
        return None;
    }
    let stem = document.path().file_stem()?;
    Some(format!("{}.{}", stem.to_string_lossy(), kind.extension()))
}

fn folder_of(path: &Path) -> String {
    match path.parent() {
        Some(folder) if !folder.as_os_str().is_empty() => folder.display().to_string(),
        _ => ".".to_owned(),
    }
}

fn fill_form(form: &mut Form, job: &ExportJob) {
    let (layout, padding, json) = match job.kind {
        ExportKind::Sheet {
            layout,
            padding,
            json,
        } => (layout, padding, json),
        _ => (
            px_export::SheetLayout::HorizontalStrip,
            px_export::Padding::default(),
            true,
        ),
    };
    *form.choice(keys::KIND, 0) = job.kind.index();
    *form.choice(keys::LAYOUT, 0) = layout_index(layout);
    *form.number(keys::COUNT, 1) = i64::from(layout_count(layout));
    *form.number(keys::BORDER, 0) = i64::from(padding.border);
    *form.number(keys::SHAPE, 0) = i64::from(padding.shape);
    *form.flag(keys::JSON, true) = json;
    *form.number(keys::SCALE, 1) = i64::from(job.scale);
    *form.number(keys::DETAIL, 1) = i64::from(job.detail.grain());
    *form.flag(keys::TRANSPARENT, true) = job.transparent;
    *form.flag(keys::TRIM, false) = job.trim;
    *form.flag(keys::SPLIT_LAYERS, false) = job.split_layers;
    *form.choice(keys::FRAMES, 0) = usize::from(matches!(job.frames, FrameScope::Current(_)));
}
