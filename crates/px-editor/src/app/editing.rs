use std::collections::BTreeSet;

use px_core::Paint;

use crate::domain::{Raster, Selection};
use crate::tools::ToolKind;

/// Editing state every window shares: the tool, the paints, the selection and clipboard,
/// the picked object and the playback.
#[derive(Clone, Debug, Default)]
pub struct Editing {
    pub tool: ToolKind,
    pub primary: Option<Paint>,
    pub secondary: Option<Paint>,
    pub filled: bool,
    /// The fill bucket fills every matching pixel, not only the touching ones.
    pub global_fill: bool,
    pub selection: Selection,
    pub clipboard: Option<Raster>,
    /// The shape picked with the pick tool, for the inspector and one-click recolor.
    pub picked: Option<usize>,
    /// Names of layers that can't be painted.
    pub locked: BTreeSet<String>,
    pub playing: bool,
    /// When playback last advanced a frame, in input milliseconds.
    pub played_at: u64,
    /// The last thing that happened, for the status bar.
    pub status: String,
}
