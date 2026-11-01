//! `pxe edit`: the editor's adapter. Only this code touches windows and files (D-015);
//! everything else is `px-editor`.

mod dialogs;
mod files;
mod host;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow};
use px_editor::app::{App, Intent, Request, WindowId};
use px_editor::assets::ThemeChoice;
use px_editor::ports::FileStore;

use files::DiskFiles;
use host::Host;

const FRAME: Duration = Duration::from_millis(16);

/// Opens the editor, on `file` when given; creates it first when it doesn't exist and
/// `size` is given.
pub(crate) fn edit(file: Option<&Path>, size: Option<(u16, u16)>, scale: Option<u8>) -> Result<()> {
    let mut files = DiskFiles;
    let mut app = App::new(ThemeChoice::Dark).map_err(|error| anyhow!(error))?;
    let layout = layout_path();
    if let Some(text) = layout.as_ref().and_then(|path| files.read(path).ok()) {
        app.apply_layout(&text);
    }
    if let Some(scale) = scale {
        app.set_scale(Some(i32::from(scale)));
    }
    if let Some(file) = file {
        app.open(file, size, &mut files)
            .map_err(|error| anyhow!(error))
            .with_context(|| format!("cannot open {}", file.display()))?;
    }
    run(&mut app, &mut files)?;
    if let Some(path) = layout {
        let _ = files.write(&path, &app.layout_text());
    }
    keep_unsaved(&app, &mut files);
    Ok(())
}

fn run(app: &mut App, files: &mut DiskFiles) -> Result<()> {
    let start = Instant::now();
    let mut hosts: BTreeMap<WindowId, Host> = BTreeMap::new();
    loop {
        for request in app.take_requests() {
            match request {
                Request::Open {
                    id,
                    title,
                    width,
                    height,
                } => {
                    let host = Host::open(&title, width, height).map_err(|error| anyhow!(error))?;
                    hosts.insert(id, host);
                }
                Request::Close { id } => {
                    hosts.remove(&id);
                }
                Request::Title { id, title } => {
                    if let Some(host) = hosts.get_mut(&id) {
                        host.window.set_title(&title);
                    }
                }
                Request::PickFile {
                    window,
                    purpose,
                    start,
                    name,
                    filters,
                } => {
                    let path = dialogs::pick(purpose, start.as_deref(), name.as_deref(), filters);
                    app.dispatch(Intent::Picked { purpose, path }, window, files);
                }
            }
        }
        if hosts.is_empty() || app.should_quit() {
            return Ok(());
        }
        let started = Instant::now();
        let time_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
        let mut closed = Vec::new();
        for (id, host) in &mut hosts {
            if !host.window.is_open() {
                closed.push(*id);
                continue;
            }
            let input = host.input(time_ms);
            let (width, height) = host.window.get_size();
            match app.frame(*id, &input, (width, height), files) {
                Some(pixels) if width > 0 && height > 0 => {
                    host.window.update_with_buffer(pixels, width, height)?;
                }
                _ => host.window.update(),
            }
        }
        for id in closed {
            hosts.remove(&id);
            app.window_closed(id);
        }
        thread::sleep(FRAME.saturating_sub(started.elapsed()));
    }
}

/// Writes unsaved changes where they can be found again instead of losing them on quit:
/// next to the file, or for a scene that never had one, under the user's config folder.
fn keep_unsaved(app: &App, files: &mut DiskFiles) {
    for slot in app.documents().iter() {
        let document = &slot.document;
        if !document.is_dirty() {
            continue;
        }
        let path = if document.is_untitled() {
            let Some(name) = document.path().file_name() else {
                continue;
            };
            let Some(folder) = config_dir() else { continue };
            folder.join("unsaved").join(name)
        } else {
            document.path().with_extension("unsaved.kdl")
        };
        if files.write(&path, document.text()).is_ok() {
            eprintln!("unsaved changes kept in {}", path.display());
        }
    }
}

/// `~/.config/px/editor.kdl`: the window layout, kept per user.
fn layout_path() -> Option<PathBuf> {
    Some(config_dir()?.join("editor.kdl"))
}

fn config_dir() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join(".config/px"))
}
