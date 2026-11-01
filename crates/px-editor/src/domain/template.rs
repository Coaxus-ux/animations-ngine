use std::fmt::Write as _;

use px_core::{MAX_CANVAS_SIDE, MAX_FPS, MAX_FRAMES};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewSpec {
    pub name: String,
    pub width: u16,
    pub height: u16,
    pub fps: u16,
    pub frames: u32,
    pub preset: String,
    pub background: Option<String>,
}

impl Default for NewSpec {
    fn default() -> Self {
        Self {
            name: "untitled".to_owned(),
            width: 64,
            height: 64,
            fps: 12,
            frames: 12,
            preset: crate::assets::palettes::DEFAULT_PRESET.to_owned(),
            background: None,
        }
    }
}

impl NewSpec {
    #[must_use]
    pub fn clamped(mut self) -> Self {
        self.width = self.width.clamp(1, MAX_CANVAS_SIDE);
        self.height = self.height.clamp(1, MAX_CANVAS_SIDE);
        self.fps = self.fps.clamp(1, MAX_FPS);
        self.frames = self.frames.clamp(1, MAX_FRAMES);
        if self.name.trim().is_empty() {
            "untitled".clone_into(&mut self.name);
        }
        self
    }
}

#[must_use]
pub fn new_text(spec: &NewSpec, preset: &str) -> String {
    let palette = palette_name(preset).unwrap_or("palette");
    let background = spec
        .background
        .clone()
        .or_else(|| first_ramp(preset).map(|ramp| format!("{ramp}.0")))
        .unwrap_or_default();
    let mut text = preset.trim_end().to_owned();
    let _ = write!(
        text,
        "\n\nscene {:?} palette={palette:?} width={} height={} fps={} duration=\"{}f\" loop=#true background={background:?} {{\n    layer \"layer-1\" {{\n        pixels \"layer-1-art\" x=0 y=0 {{\n            legend {{ }}\n        }}\n    }}\n}}\n",
        spec.name, spec.width, spec.height, spec.fps, spec.frames
    );
    text
}

#[must_use]
pub fn palette_name(preset: &str) -> Option<&str> {
    quoted_after(preset, "palette ")
}

fn first_ramp(preset: &str) -> Option<&str> {
    quoted_after(preset, "ramp ")
}

fn quoted_after<'a>(text: &'a str, keyword: &str) -> Option<&'a str> {
    text.lines()
        .map(str::trim_start)
        .find_map(|line| line.strip_prefix(keyword))
        .and_then(|rest| rest.strip_prefix('"'))
        .and_then(|rest| rest.split('"').next())
}

#[cfg(test)]
mod tests {
    use super::*;

    const PRESET: &str = "palette \"mini\" {\n    ramp \"ink\" \"#000000\" \"#ffffff\"\n}\n";

    #[test]
    fn a_new_scene_uses_the_preset_palette_and_its_first_tone_as_background() {
        let spec = NewSpec {
            name: "hero".to_owned(),
            width: 32,
            height: 24,
            fps: 24,
            frames: 48,
            ..NewSpec::default()
        };
        let text = new_text(&spec, PRESET);
        assert!(text.starts_with("palette \"mini\""), "{text}");
        assert!(
            text.contains(
                "scene \"hero\" palette=\"mini\" width=32 height=24 fps=24 duration=\"48f\" loop=#true background=\"ink.0\""
            ),
            "{text}"
        );
    }

    #[test]
    fn clamping_keeps_every_number_inside_the_limits() {
        let spec = NewSpec {
            width: 0,
            height: 9000,
            fps: 0,
            frames: 0,
            name: "  ".to_owned(),
            ..NewSpec::default()
        }
        .clamped();
        assert_eq!(
            (spec.width, spec.height, spec.fps, spec.frames),
            (1, MAX_CANVAS_SIDE, 1, 1)
        );
        assert_eq!(spec.name, "untitled");
    }
}
