pub mod palettes;

use px_format::{Font, LoadOptions, MemorySources, load};
use px_ui::{Bitmap, Icons, Theme};

const DARK: &str = include_str!("../../assets/theme-dark.kdl");
const LIGHT: &str = include_str!("../../assets/theme-light.kdl");
const ICON_FILES: [(&str, &str, &[&str]); 3] = [
    (
        "icons-tools.kdl",
        include_str!("../../assets/icons-tools.kdl"),
        &[
            "pencil",
            "eraser",
            "fill",
            "line",
            "rect",
            "ellipse",
            "eyedropper",
            "select",
            "lasso",
            "move",
            "pick",
        ],
    ),
    (
        "icons-actions.kdl",
        include_str!("../../assets/icons-actions.kdl"),
        &[
            "undo", "redo", "save", "code", "window", "play", "pause", "onion", "grid",
        ],
    ),
    (
        "icons-small.kdl",
        include_str!("../../assets/icons-small.kdl"),
        &[
            "eye", "eye-off", "lock", "unlock", "popout", "plus", "trash", "up", "down", "cel",
            "hold", "home", "close", "minus", "folder", "file", "swatch", "scene", "pixels",
            "shape", "node",
        ],
    ),
];
const SWATCH: &str = "use \"theme.kdl\"\nscene \"theme\" palette=\"theme\" width=1 height=1 fps=1 duration=\"1f\" background=\"key.0\" {}\n";

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ThemeChoice {
    #[default]
    Dark,
    Light,
}

pub struct Assets {
    pub theme: Theme,
    pub icons: Icons,
    pub font: Font,
}

impl Assets {
    pub fn load(choice: ThemeChoice) -> Result<Self, String> {
        let palette = match choice {
            ThemeChoice::Dark => DARK,
            ThemeChoice::Light => LIGHT,
        };
        let sources = ICON_FILES.iter().fold(
            MemorySources::default()
                .with("theme.kdl", palette)
                .with("swatch.kdl", SWATCH),
            |sources, (file, text, _)| sources.with(*file, *text),
        );
        let loaded = load(
            std::path::Path::new("swatch.kdl"),
            &mut sources.clone(),
            LoadOptions::default(),
        )
        .map_err(|error| format!("theme: {}", first_message(&error)))?;
        let theme = Theme::new(loaded.scene.palette().clone(), &loaded.paint_names)?;
        let mut icons = Icons::default();
        for (file, _, names) in ICON_FILES {
            for name in names {
                let options = LoadOptions {
                    scene: Some(name),
                    ..LoadOptions::default()
                };
                let icon = load(std::path::Path::new(file), &mut sources.clone(), options)
                    .map_err(|error| format!("icon {name}: {}", first_message(&error)))?;
                let bitmap = Bitmap::render(&icon.scene, theme.roles.transparent)
                    .map_err(|error| format!("icon {name}: {error}"))?;
                icons.insert(*name, bitmap);
            }
        }
        let font = Font::builtin().ok_or("the built-in font failed to load")?;
        Ok(Self { theme, icons, font })
    }
}

fn first_message(error: &px_format::LoadError) -> String {
    error
        .diagnostics
        .first()
        .map(|diagnostic| diagnostic.display(&error.files))
        .unwrap_or_default()
}
