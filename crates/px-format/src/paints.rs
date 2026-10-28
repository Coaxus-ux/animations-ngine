use core::ops::Deref;

use px_core::Paint;

use crate::diagnostic::Diagnostics;
use crate::palette::NamedPalette;
use crate::params::SceneParams;

/// The scene's palette, where a paint can also name a text param: `"$coat"` or `"$coat.lit"`.
pub(crate) struct Paints<'a> {
    pub(crate) palette: &'a NamedPalette,
    pub(crate) params: &'a SceneParams,
}

impl Paints<'_> {
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

impl Deref for Paints<'_> {
    type Target = NamedPalette;

    fn deref(&self) -> &NamedPalette {
        self.palette
    }
}
