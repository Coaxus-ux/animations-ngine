mod clipboard;
mod dialogs;
pub(crate) mod files;
mod host;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow};
use px_editor::app::{App, Intent, Request, WindowId};
use px_editor::assets::ThemeChoice;
use px_editor::ports::FileStore;
use px_ui::Key;

use clipboard::Clipboard;
use files::DiskFiles;
use host::Host;

const FRAME: Duration = Duration::from_millis(16);

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
    if let Some(path) = layout
        && let Err(error) = files.write(&path, &app.layout_text())
    {
        eprintln!(
            "could not save the window layout to {}: {error}",
            path.display()
        );
    }
    report_unsaved(&keep_unsaved(&app, &mut files));
    Ok(())
}

fn run(app: &mut App, files: &mut DiskFiles) -> Result<()> {
    let start = Instant::now();
    let mut hosts: BTreeMap<WindowId, Host> = BTreeMap::new();
    let mut clipboard = Clipboard::default();
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
                Request::Clipboard(text) => clipboard.write(text),
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
            let mut input = host.input(time_ms);
            if input.shortcut(Key::Letter('v')) {
                input.clipboard = clipboard.read();
            }
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

struct Kept {
    saved: Vec<PathBuf>,
    lost: Vec<String>,
}

fn keep_unsaved(app: &App, files: &mut DiskFiles) -> Kept {
    let mut kept = Kept {
        saved: Vec::new(),
        lost: Vec::new(),
    };
    for slot in app.documents().iter() {
        let document = &slot.document;
        if !document.is_dirty() {
            continue;
        }
        let title = document.path().display().to_string();
        let path = if document.is_untitled() {
            document
                .path()
                .file_name()
                .zip(config_dir())
                .map(|(name, folder)| folder.join("unsaved").join(name))
        } else {
            Some(document.path().with_extension("unsaved.kdl"))
        };
        match path.map(|path| files.write(&path, document.text()).map(|()| path)) {
            Some(Ok(path)) => kept.saved.push(path),
            Some(Err(error)) => kept.lost.push(format!("{title}: {error}")),
            None => kept
                .lost
                .push(format!("{title}: no place to keep it (HOME is not set)")),
        }
    }
    kept
}

fn report_unsaved(kept: &Kept) {
    if kept.saved.is_empty() && kept.lost.is_empty() {
        return;
    }
    let mut lines = Vec::new();
    if !kept.saved.is_empty() {
        lines.push("Unsaved changes were kept in:".to_owned());
        lines.extend(
            kept.saved
                .iter()
                .map(|path| format!("  {}", path.display())),
        );
    }
    if !kept.lost.is_empty() {
        lines.push("These changes could not be kept and are lost:".to_owned());
        lines.extend(kept.lost.iter().map(|line| format!("  {line}")));
    }
    let text = lines.join("\n");
    eprintln!("{text}");
    dialogs::tell("Unsaved changes", &text, !kept.lost.is_empty());
}

fn layout_path() -> Option<PathBuf> {
    Some(config_dir()?.join("editor.kdl"))
}

fn config_dir() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join(".config/px"))
}
