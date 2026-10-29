use std::io::BufWriter;
use std::path::Path;

use anyhow::Result;
use px_core::{Frame, Rgb};
use px_director::{Player, Sequence, SequenceView};
use px_export::{Scale, write_png_colors};

use crate::commands::create;

pub(crate) fn write_png(
    sequence: &Sequence,
    frame: Frame,
    scale: Scale,
    path: &Path,
) -> Result<()> {
    let mut player = Player::new(sequence);
    let mut colors = Vec::new();
    resolve(&player.render(sequence, frame)?, &mut colors)?;
    write_png_colors(
        BufWriter::new(create(path)?),
        sequence.size(),
        &colors,
        scale,
    )?;
    Ok(())
}

pub(crate) fn resolve(view: &SequenceView<'_>, colors: &mut Vec<Rgb>) -> Result<()> {
    colors.clear();
    for index in 0..view.paints().len() {
        colors.push(view.resolve(index).map_err(px_export::ExportError::from)?);
    }
    Ok(())
}
