pub const PRESETS: [(&str, &str); 4] = [
    ("basic-16", include_str!("palettes/basic-16.kdl")),
    ("gameboy-4", include_str!("palettes/gameboy-4.kdl")),
    ("gray-8", include_str!("palettes/gray-8.kdl")),
    ("dusk-12", include_str!("palettes/dusk-12.kdl")),
];

pub const DEFAULT_PRESET: &str = PRESETS[0].0;

#[must_use]
pub fn preset(name: &str) -> Option<&'static str> {
    PRESETS
        .iter()
        .find(|(known, _)| *known == name)
        .map(|(_, text)| *text)
}

pub fn names() -> impl Iterator<Item = &'static str> {
    PRESETS.iter().map(|(name, _)| *name)
}
