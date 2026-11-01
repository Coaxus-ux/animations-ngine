use std::path::PathBuf;

use px_ui::{Input, Key};

use super::dialog::DialogKind;
use super::intent::Intent;
use crate::tools::ToolKind;

/// Keyboard shortcuts (Aseprite's where it has them). None while a dialog is open (it owns
/// Enter and Escape); command shortcuts always work otherwise; single keys only while nothing
/// takes typed text. `tabs` is the active tab and how many there are: shortcuts that act on
/// a document do nothing on the home tab, as their menu entries are disabled there.
pub(super) fn collect(
    input: &Input,
    typing: bool,
    modal: bool,
    tabs: (Option<usize>, usize),
    intents: &mut Vec<Intent>,
) {
    if modal {
        return;
    }
    let shift = input.modifiers.shift;
    let open = tabs.0.is_some();
    if input.modifiers.command {
        for key in &input.keys {
            let intent = match key {
                Key::Letter('n') if shift => Intent::NewWindow,
                Key::Letter('n') => Intent::ShowDialog(DialogKind::New),
                Key::Letter('o') => Intent::OpenFile(PathBuf::new()),
                Key::Letter('q') => Intent::Quit,
                Key::Home if open => Intent::GoHome,
                Key::Tab => match neighbour_tab(tabs, shift) {
                    Some(index) => Intent::ActivateTab(index),
                    None => continue,
                },
                Key::Letter('w') => match tabs.0 {
                    Some(_) if shift => Intent::CloseAll,
                    Some(index) => Intent::CloseTab {
                        index,
                        discard: false,
                    },
                    None => continue,
                },
                _ if !open => continue,
                Key::Letter('z') if shift => Intent::Redo,
                Key::Letter('z') => Intent::Undo,
                Key::Letter('y') => Intent::Redo,
                Key::Letter('s') if shift && input.modifiers.alt => {
                    Intent::ShowDialog(DialogKind::Export)
                }
                Key::Letter('s') if shift => Intent::SaveAs(PathBuf::new()),
                Key::Letter('s') => Intent::Save,
                Key::Letter('x') if shift => Intent::RepeatExport,
                Key::Letter('c') => Intent::Copy,
                Key::Letter('x') => Intent::Cut,
                Key::Letter('v') => Intent::Paste,
                Key::Letter('a') => Intent::SelectAll,
                Key::Letter('d') => Intent::Deselect,
                Key::Letter('e') => Intent::ShowDialog(DialogKind::ExportSheet),
                Key::Letter('p') => Intent::ShowDialog(DialogKind::Properties),
                _ => continue,
            };
            intents.push(intent);
        }
        return;
    }
    if input.key(Key::Function(2)) {
        intents.push(Intent::CycleCode);
    }
    if input.key(Key::Function(1)) {
        intents.push(Intent::ToggleHelp);
    }
    if input.key(Key::Function(4)) {
        intents.push(Intent::TogglePaletteEdit);
    }
    if typing {
        return;
    }
    for key in &input.keys {
        if let Some(tool) = ToolKind::for_key(*key, shift) {
            intents.push(Intent::Tool(tool));
            continue;
        }
        let intent = match key {
            Key::Letter('x') => Intent::SwapPaints,
            Key::Plus => Intent::Zoom {
                steps: 1,
                at: input.pointer.unwrap_or_default(),
            },
            Key::Minus => Intent::Zoom {
                steps: -1,
                at: input.pointer.unwrap_or_default(),
            },
            Key::Digit(0) => Intent::ZoomFit,
            _ if !open => continue,
            Key::Letter('c') => Intent::ShowDialog(DialogKind::CanvasSize),
            Key::Letter('h') if shift => Intent::Flip { horizontal: false },
            Key::Letter('h') => Intent::Flip { horizontal: true },
            Key::Comma => Intent::StepFrame(-1),
            Key::Period => Intent::StepFrame(1),
            Key::Enter => Intent::TogglePlay,
            Key::Delete | Key::Backspace => Intent::DeleteSelection,
            Key::Escape => Intent::Deselect,
            _ => continue,
        };
        intents.push(intent);
    }
}

/// The tab after (or, with `backwards`, before) the active one, wrapping around.
pub(crate) fn neighbour_tab(
    (active, count): (Option<usize>, usize),
    backwards: bool,
) -> Option<usize> {
    let active = active?;
    if count < 2 {
        return None;
    }
    Some(if backwards {
        (active + count - 1) % count
    } else {
        (active + 1) % count
    })
}
