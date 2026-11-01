use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use px_core::MAX_CANVAS_SIDE;

#[derive(Debug, Parser)]
#[command(name = "pxe", version, about = "Pixel art engine command-line tool")]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Command,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    #[command(about = "Render one frame to a PNG")]
    Render {
        #[command(flatten)]
        input: Input,
        #[arg(long, default_value_t = 0, help = "Frame index to render")]
        frame: u32,
        #[arg(long, help = "Output PNG path (default: `out/<scene>-<frame>.png`)")]
        out: Option<PathBuf>,
    },
    #[command(about = "Open a live preview that reloads whenever the scene file changes")]
    Watch {
        #[command(flatten)]
        input: Input,
    },
    #[command(about = "Time every frame of the example and benchmark scenes (or the given files)")]
    Bench {
        #[arg(help = "Scene files (default: `examples/*.kdl` and `benchmarks/*/*.kdl`)")]
        files: Vec<PathBuf>,
        #[arg(
            long,
            default_value_t = 33.3,
            help = "Slowest allowed frame, in milliseconds"
        )]
        budget_ms: f64,
        #[arg(
            long,
            default_value_t = 3,
            help = "Times each frame is rendered; the fastest run counts"
        )]
        rounds: u32,
        #[arg(long, value_name = "NAME=V1,V2,…", value_parser = parse_sweep, help = "Render each scene once per value of a scene param, e.g. `--sweep crowd=10,100,1000`, to find where frames cross the budget")]
        sweep: Option<Sweep>,
    },
    #[command(
        about = "Print what plays when: every placed scene on a time ruler, with the markers."
    )]
    Timeline {
        #[command(flatten)]
        input: Input,
    },
    #[command(
        about = "Open the pixel-art editor, on a scene file when one is given (creating it with `--size`)."
    )]
    Edit {
        #[arg(help = "Scene file (.kdl)")]
        file: Option<PathBuf>,
        #[arg(long, value_parser = parse_size, help = "Canvas size for a new file, e.g. `--size 64x64`")]
        size: Option<(u16, u16)>,
        #[arg(long, help = "Window pixels per UI pixel (1..=4)")]
        scale: Option<u8>,
    },
    #[command(about = "Render the scene as a GIF, a PNG, a PNG sequence or a sprite sheet")]
    Export {
        #[command(flatten)]
        input: Input,
        #[command(flatten)]
        options: crate::export::Options,
    },
}

#[derive(Debug, Args)]
pub(crate) struct Input {
    #[arg(help = "Scene file (.kdl)")]
    pub(crate) file: PathBuf,
    #[arg(long, help = "Scene to use when the file holds several")]
    pub(crate) scene: Option<String>,
    #[arg(long, default_value_t = 4, help = "Integer upscale factor (1..=16)")]
    pub(crate) scale: u8,
    #[arg(long = "param", value_name = "NAME=VALUE", value_parser = parse_param, help = "Override a scene param, e.g. `--param wind=0.8` (repeatable)")]
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
