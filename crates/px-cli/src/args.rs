use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use px_core::MAX_CANVAS_SIDE;

/// Pixel art engine command-line tool.
#[derive(Debug, Parser)]
#[command(name = "pxe", version, about)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Command,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Render one frame to a PNG.
    Render {
        #[command(flatten)]
        input: Input,
        /// Frame index to render.
        #[arg(long, default_value_t = 0)]
        frame: u32,
        /// Output PNG path (default: `out/<scene>-<frame>.png`).
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Open a live preview that reloads whenever the scene file changes.
    Watch {
        #[command(flatten)]
        input: Input,
    },
    /// Time every frame of the example and benchmark scenes (or the given files).
    Bench {
        /// Scene files (default: `examples/*.kdl` and `benchmarks/*/*.kdl`).
        files: Vec<PathBuf>,
        /// Slowest allowed frame, in milliseconds.
        #[arg(long, default_value_t = 33.3)]
        budget_ms: f64,
        /// Times each frame is rendered; the fastest run counts.
        #[arg(long, default_value_t = 3)]
        rounds: u32,
        /// Render each scene once per value of a scene param, e.g. `--sweep crowd=10,100,1000`,
        /// to find where frames cross the budget.
        #[arg(long, value_name = "NAME=V1,V2,…", value_parser = parse_sweep)]
        sweep: Option<Sweep>,
    },
    /// Print what plays when: every placed scene on a time ruler, with the markers.
    Timeline {
        #[command(flatten)]
        input: Input,
    },
    /// Open the pixel-art editor, on a scene file when one is given (creating it with
    /// `--size`).
    Edit {
        /// Scene file (.kdl).
        file: Option<PathBuf>,
        /// Canvas size for a new file, e.g. `--size 64x64`.
        #[arg(long, value_parser = parse_size)]
        size: Option<(u16, u16)>,
        /// Window pixels per UI pixel (1..=4).
        #[arg(long)]
        scale: Option<u8>,
    },
    /// Render the scene as a GIF, a PNG, a PNG sequence or a sprite sheet.
    Export {
        #[command(flatten)]
        input: Input,
        #[command(flatten)]
        options: crate::export::Options,
    },
}

#[derive(Debug, Args)]
pub(crate) struct Input {
    /// Scene file (.kdl).
    pub(crate) file: PathBuf,
    /// Scene to use when the file holds several.
    #[arg(long)]
    pub(crate) scene: Option<String>,
    /// Integer upscale factor (1..=16).
    #[arg(long, default_value_t = 4)]
    pub(crate) scale: u8,
    /// Override a scene param, e.g. `--param wind=0.8` (repeatable).
    #[arg(long = "param", value_name = "NAME=VALUE", value_parser = parse_param)]
    pub(crate) params: Vec<(String, String)>,
}

#[derive(Clone, Debug)]
pub(crate) struct Sweep {
    pub(crate) param: String,
    pub(crate) values: Vec<String>,
}

fn parse_sweep(text: &str) -> Result<Sweep, String> {
    let (param, values) = parse_param(text)?;
    let values: Vec<String> = values
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .collect();
    if values.is_empty() {
        return Err(format!("expected NAME=V1,V2,…, found \"{text}\""));
    }
    Ok(Sweep { param, values })
}

fn parse_size(text: &str) -> Result<(u16, u16), String> {
    let (width, height) = text
        .split_once('x')
        .ok_or_else(|| format!("expected WIDTHxHEIGHT, found \"{text}\""))?;
    let side = |value: &str| {
        value
            .trim()
            .parse::<u16>()
            .ok()
            .filter(|side| (1..=MAX_CANVAS_SIDE).contains(side))
            .ok_or_else(|| format!("sides are 1..={MAX_CANVAS_SIDE}, found \"{value}\""))
    };
    Ok((side(width)?, side(height)?))
}

fn parse_param(text: &str) -> Result<(String, String), String> {
    text.split_once('=')
        .map(|(name, value)| (name.trim().to_owned(), value.trim().to_owned()))
        .filter(|(name, _)| !name.is_empty())
        .ok_or_else(|| format!("expected NAME=VALUE, found \"{text}\""))
}
