#![allow(clippy::unwrap_used, reason = "a developer tool")]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use px_editor::app::{App, Intent};
use px_editor::assets::ThemeChoice;
use px_editor::ports::FileStore;
use px_ui::Input;

#[derive(Default)]
struct Files(BTreeMap<PathBuf, String>);

impl FileStore for Files {
    fn read(&mut self, path: &Path) -> Result<String, String> {
        self.0
            .get(path)
            .cloned()
            .ok_or_else(|| "no such file".to_owned())
    }
    fn write(&mut self, path: &Path, text: &str) -> Result<(), String> {
        self.0.insert(path.to_path_buf(), text.to_owned());
        Ok(())
    }
    fn stamp(&mut self, _path: &Path) -> Option<u64> {
        Some(0)
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (width, height): (usize, usize) = (args[1].parse().unwrap(), args[2].parse().unwrap());
    let mut files = Files::default();
    let text = std::fs::read_to_string("examples/pixel-room.kdl").unwrap();
    files.0.insert(PathBuf::from("room.kdl"), text);
    let mut app = App::new(ThemeChoice::Dark).unwrap();
    app.open(Path::new("room.kdl"), None, &mut files).unwrap();
    let main = app.window_ids().next().unwrap();
    let mode = args.get(4).map_or("", String::as_str);
    match mode {
        "code" => app.dispatch(Intent::CycleCode, main, &mut files),
        "help" => app.dispatch(Intent::ToggleHelp, main, &mut files),
        _ => {}
    }
    app.frame(main, &Input::default(), (width, height), &mut files);
    if mode == "menu" {
        let press = Input {
            pointer: Some((90, 10)),
            pointer_anywhere: (90, 10),
            down: px_ui::Buttons {
                left: true,
                ..px_ui::Buttons::default()
            },
            pressed: px_ui::Buttons {
                left: true,
                ..px_ui::Buttons::default()
            },
            ..Input::default()
        };
        app.frame(main, &press, (width, height), &mut files);
        let hover = Input {
            pointer: Some((120, 130)),
            pointer_anywhere: (120, 130),
            ..Input::default()
        };
        app.frame(main, &hover, (width, height), &mut files);
    }
    let pixels = app
        .frame(main, &Input::default(), (width, height), &mut files)
        .unwrap();
    let colors: Vec<px_core::Rgb> = pixels
        .iter()
        .map(|pixel| {
            let [_, r, g, b] = pixel.to_be_bytes();
            px_core::Rgb::new(r, g, b)
        })
        .collect();
    let size = px_core::Size::new(
        u16::try_from(width).unwrap(),
        u16::try_from(height).unwrap(),
    )
    .unwrap();
    let file = std::fs::File::create(&args[3]).unwrap();
    px_export::write_png_colors(file, size, &colors, px_export::Scale::new(1).unwrap()).unwrap();
}
