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

const LIMIT: usize = 200;

impl History {
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
