use px_core::{Paint, Palette, Tone};
use px_format::PaintNames;

/// What every part of the UI is drawn with: paints of the theme palette by role.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Roles {
    pub window: Paint,
    pub panel: Paint,
    pub panel_light: Paint,
    pub panel_dark: Paint,
    pub header: Paint,
    pub field: Paint,
    pub ink: Paint,
    pub ink_dim: Paint,
    /// Text of controls that can't be used right now.
    pub ink_disabled: Paint,
    pub accent: Paint,
    pub accent_light: Paint,
    pub warning: Paint,
    pub checker_light: Paint,
    pub checker_dark: Paint,
    pub grid: Paint,
    /// Dithered over everything under a dialog.
    pub backdrop: Paint,
    pub tab_inactive: Paint,
    /// Pixels of icons that are see-through.
    pub transparent: Paint,
}

#[derive(Clone, Debug)]
pub struct Theme {
    pub palette: Palette,
    pub roles: Roles,
}

impl Theme {
    /// The theme whose palette has the ramps `window`, `panel` (4 tones), `field`, `ink`
    /// (2), `disabled`, `accent` (2), `warning`, `checker` (2), `grid`, `backdrop`, `tab`
    /// and `key` (icon background); names the first missing ramp or tone otherwise.
    pub fn new(palette: Palette, names: &PaintNames) -> Result<Self, String> {
        let role = |ramp: &str, tone: u8| -> Result<Paint, String> {
            let id = names
                .find(ramp)
                .ok_or_else(|| format!("theme has no ramp `{ramp}`"))?;
            palette
                .paint(id, Tone::new(tone))
                .map_err(|_| format!("theme ramp `{ramp}` needs tone {tone}"))
        };
        let roles = Roles {
            window: role("window", 0)?,
            panel: role("panel", 1)?,
            panel_light: role("panel", 2)?,
            panel_dark: role("panel", 0)?,
            header: role("panel", 3)?,
            field: role("field", 0)?,
            ink: role("ink", 1)?,
            ink_dim: role("ink", 0)?,
            ink_disabled: role("disabled", 0)?,
            accent: role("accent", 0)?,
            accent_light: role("accent", 1)?,
            warning: role("warning", 0)?,
            checker_light: role("checker", 1)?,
            checker_dark: role("checker", 0)?,
            grid: role("grid", 0)?,
            backdrop: role("backdrop", 0)?,
            tab_inactive: role("tab", 0)?,
            transparent: role("key", 0)?,
        };
        Ok(Self { palette, roles })
    }
}
