//! Format-preserving edits of KDL source (EDITOR.md §3, D-014). Every change splices the
//! text at the parser's spans and reparses, so comments, spacing, order, disabled nodes
//! and every untouched byte stay exactly as written.

mod path;
mod text;

use std::fmt;

use kdl::{KdlDocument, KdlError, KdlNode, KdlValue};

pub use path::{NodePath, Step};

use text::{Lines, Span};

const INDENT: &str = "    ";

/// A KDL document kept as its exact text.
#[derive(Clone, Debug)]
pub struct Source {
    text: String,
    document: KdlDocument,
}

#[derive(Debug)]
pub enum EditError {
    /// The text (or the text after an edit) is not valid KDL.
    Parse(String),
    NotFound(NodePath),
}

impl fmt::Display for EditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(message) => write!(f, "not valid KDL: {message}"),
            Self::NotFound(path) => write!(f, "no node at {path}"),
        }
    }
}

impl std::error::Error for EditError {}

impl Source {
    pub fn parse(text: impl Into<String>) -> Result<Self, EditError> {
        let text = text.into();
        let document = parse(&text)?;
        Ok(Self { text, document })
    }

    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    #[must_use]
    pub fn node(&self, path: &NodePath) -> Option<&KdlNode> {
        path.find(&self.document)
    }

    /// The deepest node whose text contains byte `offset`.
    #[must_use]
    pub fn path_at(&self, offset: usize) -> Option<NodePath> {
        let mut path = NodePath::root();
        let mut nodes = self.document.nodes();
        let mut found = None;
        while let Some(node) = nodes.iter().find(|node| Span::of(node).contains(offset)) {
            path = path.step_to(nodes, node);
            found = Some(path.clone());
            nodes = node.children().map(KdlDocument::nodes).unwrap_or_default();
        }
        found
    }

    /// Byte range of the node's own text, from its name to its last token.
    #[must_use]
    pub fn span(&self, path: &NodePath) -> Option<(usize, usize)> {
        self.node(path).map(|node| {
            let span = Span::of(node).trimmed(&self.text);
            (span.start, span.end)
        })
    }

    /// The whitespace the node's line starts with.
    #[must_use]
    pub fn indent(&self, path: &NodePath) -> String {
        self.node(path)
            .map(|node| Lines(&self.text).indent_at(node.span().offset()).to_owned())
            .unwrap_or_default()
    }

    /// Sets `key=value`, replacing the property if the node has it, else adding it after the
    /// node's last argument or property.
    pub fn set_property(
        &mut self,
        path: &NodePath,
        key: &str,
        value: &KdlValue,
    ) -> Result<(), EditError> {
        let node = self.found(path)?;
        let text = format!("{key}={}", repr(value));
        let existing = node
            .entries()
            .iter()
            .rev()
            .find(|entry| entry.name().is_some_and(|name| name.value() == key));
        let edit = if let Some(entry) = existing {
            (Span::of_entry(entry).trimmed(&self.text), text)
        } else {
            let after = node.entries().last().map_or_else(
                || node.span().offset() + node.name().len(),
                |entry| Span::of_entry(entry).trimmed(&self.text).end,
            );
            (Span::at(after), format!(" {text}"))
        };
        self.splice(&[edit])
    }

    /// Sets argument number `index` of the node (counting arguments only, not properties).
    pub fn set_argument(
        &mut self,
        path: &NodePath,
        index: usize,
        value: &KdlValue,
    ) -> Result<(), EditError> {
        let node = self.found(path)?;
        let entry = node
            .entries()
            .iter()
            .filter(|entry| entry.name().is_none())
            .nth(index)
            .ok_or_else(|| EditError::NotFound(path.clone()))?;
        let span = Span::of_entry(entry).trimmed(&self.text);
        self.splice(&[(span, repr(value))])
    }

    /// Replaces the node's text with `text`; extra lines of `text` should already carry the
    /// node's indentation (`indent`).
    pub fn replace(&mut self, path: &NodePath, text: &str) -> Result<(), EditError> {
        let span = Span::of(self.found(path)?).trimmed(&self.text);
        self.splice(&[(span, text.to_owned())])
    }

    /// Replaces the node's block (`{ … }`) with `children` (nodes one per line, their later
    /// lines indented relative to them), keeping the node's name, arguments and properties.
    pub fn set_children(&mut self, path: &NodePath, children: &str) -> Result<(), EditError> {
        let node = self.found(path)?;
        let span = Span::of(node).trimmed(&self.text);
        let head_end = node.entries().last().map_or_else(
            || node.span().offset() + node.name().len(),
            |entry| Span::of_entry(entry).trimmed(&self.text).end,
        );
        let indent = Lines(&self.text).indent_at(span.start).to_owned();
        let body = indented(children, &format!("{indent}{INDENT}"));
        let edit = (
            Span {
                start: head_end,
                end: span.end,
            },
            format!(" {{\n{body}{indent}}}"),
        );
        self.splice(&[edit])
    }

    /// Removes the node with its whole line when nothing else shares it.
    pub fn remove(&mut self, path: &NodePath) -> Result<(), EditError> {
        let span = Span::of(self.found(path)?).trimmed(&self.text);
        let lines = Lines(&self.text);
        let span = lines.whole_lines(span);
        self.splice(&[(span, String::new())])
    }

    /// Inserts `text` (one node, its later lines indented relative to the node) as child
    /// number `index` of `parent`, indented like its siblings.
    pub fn insert(&mut self, parent: &NodePath, index: usize, text: &str) -> Result<(), EditError> {
        let node = self.found(parent)?;
        let lines = Lines(&self.text);
        let children = node.children().map(KdlDocument::nodes).unwrap_or_default();
        let parent_indent = lines.indent_at(node.span().offset());
        let indent = children.first().map_or_else(
            || format!("{parent_indent}{INDENT}"),
            |first| lines.indent_at(first.span().offset()).to_owned(),
        );
        let body = indented(text, &indent);
        let edit = match (children.get(index), children.last()) {
            (Some(next), _) => (Span::at(lines.line_start(next.span().offset())), body),
            (None, Some(last)) => {
                let end = Span::of(last).trimmed(&self.text).end;
                (Span::at(lines.line_end(end)), body)
            }
            (None, None) => {
                let span = Span::of(node).trimmed(&self.text);
                match self.text[span.start..span.end].rfind('}') {
                    Some(brace) => (Span::at(lines.line_start(span.start + brace)), body),
                    None => (Span::at(span.end), format!(" {{\n{body}{parent_indent}}}")),
                }
            }
        };
        self.splice(&[edit])
    }

    /// Moves child `from` of `parent` to position `to` (counted after removing it).
    pub fn move_child(
        &mut self,
        parent: &NodePath,
        from: usize,
        to: usize,
    ) -> Result<(), EditError> {
        let node = self.found(parent)?;
        let children = node.children().map(KdlDocument::nodes).unwrap_or_default();
        let child = children
            .get(from)
            .ok_or_else(|| EditError::NotFound(parent.clone()))?;
        let span = Span::of(child).trimmed(&self.text);
        let lines = Lines(&self.text);
        let indent = lines.indent_at(span.start).to_owned();
        let moved = dedented(&self.text[span.start..span.end], &indent);
        let path = parent.step_to(children, child);
        self.remove(&path)?;
        self.insert(parent, to, &moved)
    }

    fn found(&self, path: &NodePath) -> Result<&KdlNode, EditError> {
        self.node(path)
            .ok_or_else(|| EditError::NotFound(path.clone()))
    }

    /// Applies non-overlapping edits and reparses; on a parse error nothing changes.
    fn splice(&mut self, edits: &[(Span, String)]) -> Result<(), EditError> {
        let mut text = self.text.clone();
        let mut ordered: Vec<&(Span, String)> = edits.iter().collect();
        ordered.sort_by_key(|(span, _)| std::cmp::Reverse(span.start));
        for (span, replacement) in ordered {
            text.replace_range(span.start..span.end, replacement);
        }
        self.document = parse(&text)?;
        self.text = text;
        Ok(())
    }
}

/// How a value is written: strings always quoted, as the project writes them.
fn repr(value: &KdlValue) -> String {
    match value {
        KdlValue::String(text) => {
            let escaped = text
                .replace('\\', "\\\\")
                .replace('"', "\\\"")
                .replace('\n', "\\n");
            format!("\"{escaped}\"")
        }
        other => other.to_string(),
    }
}

fn parse(text: &str) -> Result<KdlDocument, EditError> {
    KdlDocument::parse_v2(text).map_err(|error: KdlError| EditError::Parse(error.to_string()))
}

/// Every line of `text` prefixed with `indent`, ending in a newline.
fn indented(text: &str, indent: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.lines() {
        if !line.is_empty() {
            out.push_str(indent);
            out.push_str(line);
        }
        out.push('\n');
    }
    out
}

/// `text` with `indent` taken off every line after the first.
fn dedented(text: &str, indent: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for (index, line) in text.lines().enumerate() {
        let line = if index == 0 {
            line
        } else {
            line.strip_prefix(indent).unwrap_or(line)
        };
        out.push_str(line);
        out.push('\n');
    }
    out
}
