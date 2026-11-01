use std::path::{Path, PathBuf};

use px_editor::app::PickPurpose;

pub(super) fn pick(
    purpose: PickPurpose,
    start: Option<&Path>,
    name: Option<&str>,
    filters: &[(&str, &[&str])],
) -> Option<PathBuf> {
    let mut dialog = rfd::FileDialog::new();
    for (label, extensions) in filters {
        dialog = dialog.add_filter(*label, extensions);
    }
    if let Some(start) = start {
        dialog = dialog.set_directory(start);
    }
    if let Some(name) = name {
        dialog = dialog.set_file_name(name);
    }
    match purpose {
        PickPurpose::Open | PickPurpose::ImportPalette => dialog.pick_file(),
        PickPurpose::SaveAs | PickPurpose::ExportOut | PickPurpose::SavePalette => {
            dialog.save_file()
        }
    }
}
