use px_ui::{Input, Key};

use super::intent::Intent;
use crate::tools::ToolKind;

/// Keyboard shortcuts (Aseprite's where it has them). Command shortcuts always work; single
/// keys only while nothing takes typed text.
pub(super) fn collect(input: &Input, typing: bool, intents: &mut Vec<Intent>) {
    let shift = input.modifiers.shift;
    if input.modifiers.command {
        for key in &input.keys {
            let intent = match key {
                Key::Letter('z') if shift => Intent::Redo,
                Key::Letter('z') => Intent::Undo,
                Key::Letter('y') => Intent::Redo,
                Key::Letter('s') => Intent::Save,
                Key::Letter('c') => Intent::Copy,
                Key::Letter('x') => Intent::Cut,
                Key::Letter('v') => Intent::Paste,
                Key::Letter('a') => Intent::SelectAll,
                Key::Letter('d') => Intent::Deselect,
                Key::Letter('n') if shift => Intent::NewWindow,
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
            Key::Letter('h') if shift => Intent::Flip { horizontal: false },
            Key::Letter('h') => Intent::Flip { horizontal: true },
            Key::Comma => Intent::StepFrame(-1),
            Key::Period => Intent::StepFrame(1),
            Key::Enter => Intent::TogglePlay,
            Key::Delete | Key::Backspace => Intent::DeleteSelection,
            Key::Plus => Intent::Zoom {
                steps: 1,
                at: input.pointer.unwrap_or_default(),
            },
            Key::Minus => Intent::Zoom {
                steps: -1,
                at: input.pointer.unwrap_or_default(),
            },
            Key::Digit(0) => Intent::ZoomFit,
            Key::Escape => Intent::Deselect,
            _ => continue,
        };
        intents.push(intent);
    }
}
