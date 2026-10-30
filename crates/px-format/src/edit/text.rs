use kdl::{KdlEntry, KdlNode};

/// A byte range of the source text.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct Span {
    pub(super) start: usize,
    pub(super) end: usize,
}

impl Span {
    pub(super) fn of(node: &KdlNode) -> Self {
        let span = node.span();
        Self {
            start: span.offset(),
            end: span.offset() + span.len(),
        }
    }

    pub(super) fn of_entry(entry: &KdlEntry) -> Self {
        let span = entry.span();
        Self {
            start: span.offset(),
            end: span.offset() + span.len(),
        }
    }

    pub(super) const fn at(offset: usize) -> Self {
        Self {
            start: offset,
            end: offset,
        }
    }

    pub(super) const fn contains(self, offset: usize) -> bool {
        offset >= self.start && offset < self.end
    }

    /// Without the whitespace the parser counts as part of a node's text at its end.
    pub(super) fn trimmed(self, text: &str) -> Self {
        let kept = text[self.start..self.end].trim_end().len();
        Self {
            end: self.start + kept,
            ..self
        }
    }
}

/// Line arithmetic over the source text.
pub(super) struct Lines<'t>(pub(super) &'t str);

impl Lines<'_> {
    pub(super) fn line_start(&self, offset: usize) -> usize {
        self.0[..offset]
            .rfind('\n')
            .map_or(0, |newline| newline + 1)
    }

    /// Just past the newline ending the line `offset` is on (or the end of the text).
    pub(super) fn line_end(&self, offset: usize) -> usize {
        self.0[offset..]
            .find('\n')
            .map_or(self.0.len(), |newline| offset + newline + 1)
    }

    /// The whitespace before `offset` on its line, or nothing if other text precedes it.
    pub(super) fn indent_at(&self, offset: usize) -> &str {
        let before = &self.0[self.line_start(offset)..offset];
        if before.chars().all(char::is_whitespace) {
            before
        } else {
            ""
        }
    }

    /// `span` widened to its whole lines (with a trailing comment) when it is alone on them.
    pub(super) fn whole_lines(&self, span: Span) -> Span {
        let start = self.line_start(span.start);
        let end = self.line_end(span.end);
        let before = &self.0[start..span.start];
        let after = self.0[span.end..end].trim_start();
        let alone = before.chars().all(char::is_whitespace)
            && (after.is_empty() || after.starts_with("//"));
        if alone { Span { start, end } } else { span }
    }
}
