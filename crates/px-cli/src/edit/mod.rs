//! `pxe edit`: the editor's adapter. Only this code touches windows and files (D-015);
//! everything else is `px-editor`.

mod files;
mod host;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow};
use px_editor::app::{App, Request, WindowId};
use px_editor::assets::ThemeChoice;
use px_editor::ports::FileStore;

use files::DiskFiles;
use host::Host;

const FRAME: Duration = Duration::from_millis(16);

/// Opens `file` in the editor; creates it first when it doesn't exist and `size` is given.
pub(crate) fn edit(file: &Path, size: Option<(u16, u16)>, scale: Option<u8>) -> Result<()> {
    let mut files = DiskFiles;
    let mut app = App::new(ThemeChoice::Dark).map_err(|error| anyhow!(error))?;
    let layout = layout_path();
    if let Some(text) = layout.as_ref().and_then(|path| files.read(path).ok()) {
        app.apply_layout(&text);
    }
    if let Some(scale) = scale {
        app.set_scale(Some(i32::from(scale)));
    }
    app.open(file, size, &mut files)
        .map_err(|error| anyhow!(error))
        .with_context(|| format!("cannot open {}", file.display()))?;
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

/// Writes unsaved changes next to the file instead of losing them on quit.
fn keep_unsaved(app: &App, files: &mut DiskFiles) {
    let Some(document) = app.document().filter(|document| document.is_dirty()) else {
        return;
    };
    let path = document.path().with_extension("unsaved.kdl");
    if files.write(&path, document.text()).is_ok() {
        eprintln!("unsaved changes kept in {}", path.display());
    }
}

/// `~/.config/px/editor.kdl`: the window layout, kept per user.
fn layout_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join(".config/px/editor.kdl"))
}
