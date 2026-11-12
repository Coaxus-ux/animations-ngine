use clap::Args;
use px_engine::Detail;

use crate::args::{DETAIL_HELP, Sweep, parse_detail, parse_size, parse_sweep};

#[derive(Debug, Args)]
pub(crate) struct Options {
    #[arg(
        long,
        help = "Slowest allowed frame, in milliseconds, for every file [default: a sequence's `frame-budget`, else 33.3]"
    )]
    pub(crate) budget_ms: Option<f64>,
    #[arg(
        long,
        default_value_t = 3,
        help = "Times each frame is rendered and the fastest run counts; with `--editor`, the number of playback loops, every frame counted"
    )]
    pub(crate) rounds: u32,
    #[arg(long, default_value = "1", value_parser = parse_detail, help = DETAIL_HELP)]
    pub(crate) detail: Detail,
    #[arg(long, value_name = "NAME=V1,V2,…", value_parser = parse_sweep, help = "Render each scene once per value of a scene param, e.g. `--sweep crowd=10,100,1000`, to find where frames cross the budget")]
    pub(crate) sweep: Option<Sweep>,
    #[arg(long, value_name = "WxH", value_parser = parse_size, help = "Time whole editor frames (interface, canvas and playback) in a window of this size, e.g. `--editor 1920x1080`, instead of bare renders")]
    pub(crate) editor: Option<(u16, u16)>,
}
