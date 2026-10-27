use std::fmt;

const MAX_DIAGNOSTICS: usize = 64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub offset: usize,
    pub message: String,
}

impl Diagnostic {
    #[must_use]
    pub fn render(&self, path: &str, source: &str) -> String {
        let (line, column) = line_col(source, self.offset);
        let label = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        format!("{path}:{line}:{column} {label} {}", self.message)
    }
}

fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = source.get(..offset.min(source.len())).unwrap_or(source);
    let line = before.matches('\n').count() + 1;
    let column = before
        .rsplit('\n')
        .next()
        .map_or(0, |text| text.chars().count())
        + 1;
    (line, column)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoadError {
    pub diagnostics: Vec<Diagnostic>,
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let errors = self
            .diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .count();
        write!(f, "scene file has {errors} error(s)")
    }
}

impl std::error::Error for LoadError {}

#[derive(Debug, Default)]
pub(crate) struct Diagnostics {
    items: Vec<Diagnostic>,
}

impl Diagnostics {
    pub(crate) fn error(&mut self, offset: usize, message: impl Into<String>) {
        self.push(Severity::Error, offset, message.into());
    }

    pub(crate) fn warning(&mut self, offset: usize, message: impl Into<String>) {
        self.push(Severity::Warning, offset, message.into());
    }

    fn push(&mut self, severity: Severity, offset: usize, message: String) {
        if self.items.len() < MAX_DIAGNOSTICS {
            self.items.push(Diagnostic {
                severity,
                offset,
                message,
            });
        }
    }

    pub(crate) fn has_errors(&self) -> bool {
        self.items.iter().any(|d| d.severity == Severity::Error)
    }

    pub(crate) fn finish(mut self) -> Result<Vec<Diagnostic>, LoadError> {
        if self.has_errors() {
            self.items
                .sort_by_key(|d| (d.severity != Severity::Error, d.offset));
            Err(LoadError {
                diagnostics: self.items,
            })
        } else {
            Ok(self.items)
        }
    }
}
