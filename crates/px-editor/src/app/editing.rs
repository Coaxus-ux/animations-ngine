use std::collections::BTreeSet;

use px_core::Paint;

use super::Status;
use crate::domain::{Raster, Selection};
use crate::tools::ToolKind;

#[derive(Clone, Debug, Default)]
pub struct Editing {
    pub tool: ToolKind,
    pub primary: Option<Paint>,
    pub secondary: Option<Paint>,
    pub filled: bool,
    pub global_fill: bool,
    pub selection: Selection,
    pub clipboard: Option<Raster>,
    pub picked: Option<usize>,
    pub locked: BTreeSet<String>,
    pub playing: bool,
    pub played_at: u64,
    pub status: Status,
}
