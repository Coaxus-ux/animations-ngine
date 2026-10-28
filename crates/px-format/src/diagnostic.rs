use std::fmt;

use crate::sources::SourceFile;

const MAX_DIAGNOSTICS: usize = 64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    pub severity: Severity,
    /// Index into the files of the load ([`LoadError::files`], `Loaded::files`).
    pub file: usize,
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

    /// Renders against the files of the load this diagnostic came from.
    #[must_use]
    pub fn display(&self, files: &[SourceFile]) -> String {
        match files.get(self.file) {
            Some(file) => self.render(&file.path.display().to_string(), &file.text),
            None => self.render("<unknown>", ""),
        }
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
    pub files: Vec<SourceFile>,
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
    file: usize,
}

impl Diagnostics {
    /// Attributes later diagnostics to `file`; returns the previous file to restore.
    pub(crate) fn in_file(&mut self, file: usize) -> usize {
        core::mem::replace(&mut self.file, file)
    }

    pub(crate) fn error(&mut self, offset: usize, message: impl Into<String>) {
        self.push(Severity::Error, offset, message.into());
    }

    pub(crate) fn warning(&mut self, offset: usize, message: impl Into<String>) {
        self.push(Severity::Warning, offset, message.into());
    }

    /// Repeats are dropped: a scene placed many times reports each problem once.
    fn push(&mut self, severity: Severity, offset: usize, message: String) {
        let diagnostic = Diagnostic {
            severity,
            file: self.file,
            offset,
            message,
        };
        if self.items.len() < MAX_DIAGNOSTICS && !self.items.contains(&diagnostic) {
            self.items.push(diagnostic);
        }
    }

    pub(crate) fn has_errors(&self) -> bool {
        self.items.iter().any(|d| d.severity == Severity::Error)
    }

    pub(crate) fn finish(mut self) -> Result<Vec<Diagnostic>, Vec<Diagnostic>> {
        if self.has_errors() {
            self.items
                .sort_by_key(|d| (d.severity != Severity::Error, d.file, d.offset));
            Err(self.items)
        } else {
            Ok(self.items)
        }
    }
}
