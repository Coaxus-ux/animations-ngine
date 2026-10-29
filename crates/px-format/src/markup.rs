//! Inline tags that style part of a text: `[b]…[/b]`, `[i]…[/i]` and `[size=2]…[/size]`.
//! `[[` writes a `[`; anything else in brackets is plain text.

/// The largest `[size=N]`.
pub(crate) const MAX_SCALE: u8 = 16;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Style {
    pub(crate) bold: bool,
    pub(crate) italic: bool,
    /// Glyph pixels per font pixel.
    pub(crate) scale: u8,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            bold: false,
            italic: false,
            scale: 1,
        }
    }
}

/// Every character of `text` with its style, tags removed. Tags may span lines and nest.
pub(crate) fn styled(text: &str, base: Style) -> Vec<(char, Style)> {
    let mut tags = Tags {
        base,
        bold: 0,
        italic: 0,
        sizes: Vec::new(),
    };
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(character) = rest.chars().next() {
        let after = &rest[character.len_utf8()..];
        if let Some(after) = rest.strip_prefix("[[") {
            out.push(('[', tags.style()));
            rest = after;
        } else if let Some((tag, after)) = after.split_once(']').filter(|_| character == '[')
            && tags.apply(tag)
        {
            rest = after;
        } else {
            out.push((character, tags.style()));
            rest = after;
        }
    }
    out
}

struct Tags {
    base: Style,
    bold: u32,
    italic: u32,
    sizes: Vec<u8>,
}

impl Tags {
    fn style(&self) -> Style {
        Style {
            bold: self.base.bold || self.bold > 0,
            italic: self.base.italic || self.italic > 0,
            scale: self.sizes.last().copied().unwrap_or(self.base.scale),
        }
    }

    /// Whether `tag` is one this module knows (and so is not drawn).
    fn apply(&mut self, tag: &str) -> bool {
        match tag {
            "b" => self.bold += 1,
            "i" => self.italic += 1,
            "/b" if self.bold > 0 => self.bold -= 1,
            "/i" if self.italic > 0 => self.italic -= 1,
            "/size" => return self.sizes.pop().is_some(),
            _ => {
                let size = tag
                    .strip_prefix("size=")
                    .and_then(|size| size.parse::<u8>().ok())
                    .filter(|size| (1..=MAX_SCALE).contains(size));
                let Some(size) = size else { return false };
                self.sizes.push(size);
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::{Style, styled};

    fn letters(text: &str) -> String {
        styled(text, Style::default())
            .into_iter()
            .map(|(character, _)| character)
            .collect()
    }

    #[test]
    fn known_tags_are_removed_and_others_drawn() {
        assert_eq!(letters("a[b]b[/b][i]c[/i][size=3]d[/size]"), "abcd");
        assert_eq!(
            letters("[x] [size=0] [size=99] [/b] [[b]"),
            "[x] [size=0] [size=99] [/b] [b]"
        );
        assert_eq!(letters("[b"), "[b");
    }

    #[test]
    fn tags_nest_and_span_lines() {
        let styles: Vec<Style> =
            styled("[b]a[i]\nb[/b][size=2]c[size=4]d[/size]e", Style::default())
                .into_iter()
                .map(|(_, style)| style)
                .collect();
        let style = |bold, italic, scale| Style {
            bold,
            italic,
            scale,
        };
        assert_eq!(
            styles,
            [
                style(true, false, 1),
                style(true, true, 1),
                style(true, true, 1),
                style(false, true, 2),
                style(false, true, 4),
                style(false, true, 2),
            ]
        );
    }

    #[test]
    fn whole_text_styles_stay_on_through_closing_tags() {
        let base = Style {
            bold: true,
            ..Style::default()
        };
        assert!(
            styled("[b]a[/b]b", base)
                .iter()
                .all(|(_, style)| style.bold)
        );
    }
}
