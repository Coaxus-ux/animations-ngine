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
}
