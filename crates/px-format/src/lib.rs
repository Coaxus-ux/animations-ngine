//! Loads KDL scene files (`docs/FORMAT.md`) into the engine's scene model.
//!
//! M0 scope: one file holding `palette` and `scene` nodes; shapes `rect`, `ellipse` and
//! `line`; numeric values or `"=expression"` strings; paints written `ramp.tone`.
//!
//! # Allocation
//!
//! Allocation-conscious: this is the authoring layer. It parses, validates and compiles
//! once, then hands an immutable [`px_engine::Scene`] to the allocation-free renderer.
#![forbid(unsafe_code)]

mod diagnostic;
mod document;
mod palette;
mod props;
mod scene;
mod shapes;
mod values;

pub use diagnostic::{Diagnostic, LoadError, Severity};
pub use document::{Loaded, load_scene};
