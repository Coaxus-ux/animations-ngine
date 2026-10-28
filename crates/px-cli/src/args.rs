use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

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
    },
    /// Render every frame to a looping GIF.
    Export {
        #[command(flatten)]
        input: Input,
        /// Output GIF path (default: `out/<scene>.gif`).
        #[arg(long)]
        out: Option<PathBuf>,
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

fn parse_param(text: &str) -> Result<(String, String), String> {
    text.split_once('=')
        .map(|(name, value)| (name.trim().to_owned(), value.trim().to_owned()))
        .filter(|(name, _)| !name.is_empty())
        .ok_or_else(|| format!("expected NAME=VALUE, found \"{text}\""))
}
