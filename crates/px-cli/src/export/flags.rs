use anyhow::{Result, bail};
use px_editor::domain::export::{ExportKind, KIND_NAMES, LAYOUT_NAMES, layout_from, layout_index};
use px_export::{Padding, SheetLayout};

use super::Options;

pub(super) fn plain_kind(index: usize) -> ExportKind {
    ExportKind::from_index(
        index,
        SheetLayout::HorizontalStrip,
        Padding::default(),
        false,
    )
}

pub(super) fn editor_only(options: &Options) -> Vec<&'static str> {
    given(&[
        ("--transparent", options.transparent),
        ("--trim", options.trim),
        ("--split-layers", options.split_layers),
        ("--frame", options.frame.is_some()),
    ])
}

pub(super) fn sheet_only(options: &Options) -> Result<()> {
    let first = given(&[
        ("--layout", options.layout.is_some()),
        ("--count", options.count.is_some()),
        ("--padding", options.padding.is_some()),
        ("--border", options.border.is_some()),
        ("--meta", options.meta.is_some()),
    ])
    .into_iter()
    .next();
    match first {
        Some(flag) => bail!("{flag} applies to --format sheet"),
        None => Ok(()),
    }
}

pub(super) fn for_kind(options: &Options, kind: ExportKind) -> Result<()> {
    if options.trim && kind == ExportKind::Gif {
        bail!(
            "--trim applies to --format {}",
            kinds_where(|kind| kind != ExportKind::Gif)
        );
    }
    if options.split_layers && !kind.splits() {
        bail!(
            "--split-layers applies to --format {}",
            kinds_where(ExportKind::splits)
        );
    }
    if options.jobs.is_some() {
        bail!(
            "--jobs applies to --format {} without --transparent or --frame",
            KIND_NAMES[ExportKind::Gif.index()]
        );
    }
    Ok(())
}

pub(super) fn restriction(options: &Options) -> String {
    if plain_kind(options.format) == ExportKind::Gif {
        format!("with {}", list(&editor_only(options)))
    } else {
        format!("as --format {}", KIND_NAMES[options.format])
    }
}

pub(super) fn layout(options: &Options) -> Result<SheetLayout> {
    let layout = layout_from(options.layout.unwrap_or(0), options.count.unwrap_or(1));
    let name = LAYOUT_NAMES[layout_index(layout)];
    match (layout, options.count) {
        (SheetLayout::HorizontalStrip | SheetLayout::VerticalStrip, None) => Ok(layout),
        (SheetLayout::Rows { .. } | SheetLayout::Columns { .. }, Some(count)) if count > 0 => {
            Ok(layout)
        }
        (SheetLayout::Rows { .. } | SheetLayout::Columns { .. }, Some(_)) => {
            bail!("--count must be at least 1")
        }
        (SheetLayout::Rows { .. }, None) => bail!("--layout {name} needs --count (columns)"),
        (SheetLayout::Columns { .. }, None) => bail!("--layout {name} needs --count (rows)"),
        (SheetLayout::HorizontalStrip | SheetLayout::VerticalStrip, Some(_)) => {
            bail!("--count applies to --layout rows or columns")
        }
    }
}

fn given(flags: &[(&'static str, bool)]) -> Vec<&'static str> {
    flags
        .iter()
        .filter(|(_, given)| *given)
        .map(|(flag, _)| *flag)
        .collect()
}

fn kinds_where(keep: impl Fn(ExportKind) -> bool) -> String {
    let names: Vec<&str> = KIND_NAMES
        .iter()
        .enumerate()
        .filter(|(index, _)| keep(plain_kind(*index)))
        .map(|(_, name)| *name)
        .collect();
    list(&names)
}

fn list(items: &[&str]) -> String {
    match items {
        [] => String::new(),
        [only] => (*only).to_owned(),
        [rest @ .., last] => format!("{} or {last}", rest.join(", ")),
    }
}
