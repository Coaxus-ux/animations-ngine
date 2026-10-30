/// Undo and redo as whole document texts (D-014): every change, from a brush stroke to an
/// edit in the code view, is one entry, so both views share one history.
#[derive(Clone, Debug, Default)]
pub struct History {
    undo: Vec<Entry>,
    redo: Vec<Entry>,
}

#[derive(Clone, Debug)]
struct Entry {
    label: String,
    text: String,
}

/// How many changes can be undone; older ones are forgotten.
const LIMIT: usize = 200;

impl History {
    /// Records that `label` changed the document away from `before`.
    pub fn push(&mut self, label: &str, before: String) {
        self.undo.push(Entry {
            label: label.to_owned(),
            text: before,
        });
        if self.undo.len() > LIMIT {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    /// The text to go back to from `current`, and what is being undone.
    pub fn undo(&mut self, current: String) -> Option<(String, String)> {
        let entry = self.undo.pop()?;
        self.redo.push(Entry {
            label: entry.label.clone(),
            text: current,
        });
        Some((entry.text, entry.label))
    }

    pub fn redo(&mut self, current: String) -> Option<(String, String)> {
        let entry = self.redo.pop()?;
        self.undo.push(Entry {
            label: entry.label.clone(),
            text: current,
        });
        Some((entry.text, entry.label))
    }

    #[must_use]
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    #[must_use]
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
}
