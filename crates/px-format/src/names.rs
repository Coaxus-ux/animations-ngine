use core::ops::Deref;

use px_core::Paint;

use crate::diagnostic::Diagnostics;
use crate::font::Fonts;
use crate::palette::NamedPalette;
use crate::params::SceneParams;

pub(crate) struct Names<'a> {
    pub(crate) palette: &'a NamedPalette,
    pub(crate) params: &'a SceneParams,
    pub(crate) fonts: &'a Fonts,
}

impl Names<'_> {
    pub(crate) fn paint(
        &self,
        text: &str,
        offset: usize,
        diags: &mut Diagnostics,
    ) -> Option<Paint> {
        let Some(reference) = text.strip_prefix('$') else {
            return self.palette.paint(text, offset, diags);
        };
        let (name, tone) = match reference.split_once('.') {
            Some((name, tone)) => (name, Some(tone)),
            None => (reference, None),
        };
        match (self.params.text(name), tone) {
            (Ok(value), Some(tone)) => {
                self.palette
                    .paint(&format!("{value}.{tone}"), offset, diags)
            }
            (Ok(value), None) => self.palette.paint(value, offset, diags),
            (Err(message), _) => {
                diags.error(offset, message);
                None
            }
        }
    }
}

impl Deref for Names<'_> {
    type Target = NamedPalette;

    fn deref(&self) -> &NamedPalette {
        self.palette
    }
}
