use std::collections::BTreeMap;
use std::fmt::Write as _;

use px_core::Paint;
use px_engine::Cel;

use super::EMPTY;
use crate::source_map::PaintNames;

const CODES: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";

#[must_use]
pub fn write_pixels(cels: &[Cel], names: &PaintNames) -> Option<String> {
    let mut paints: Vec<Paint> = cels
        .iter()
        .flat_map(|cel| cel.pixels.iter().flatten().copied())
        .collect();
    paints.sort_by_key(|paint| (paint.ramp.index(), paint.tone.index()));
    paints.dedup();
    let letters: Vec<char> = CODES.chars().collect();
    let width = if paints.len() <= letters.len() { 1 } else { 2 };
    let code = |index: usize| -> String {
        match width {
            1 => letters[index].to_string(),
            _ => [
                letters[index / letters.len()],
                letters[index % letters.len()],
            ]
            .iter()
            .collect(),
        }
    };
    let mut codes = BTreeMap::new();
    let mut text = String::from("legend {");
    for (index, paint) in paints.iter().enumerate() {
        let name = names.paint(*paint)?;
        let _ = write!(text, " {} \"{name}\";", code(index));
        codes.insert((paint.ramp.index(), paint.tone.index()), code(index));
    }
    text.push_str(" }\n");
    let empty: String = std::iter::repeat_n(EMPTY, width).collect();
    for cel in cels {
        write_cel(&mut text, cel, |paint| match paint {
            Some(paint) => codes[&(paint.ramp.index(), paint.tone.index())].clone(),
            None => empty.clone(),
        });
    }
    Some(text)
}

fn write_cel(text: &mut String, cel: &Cel, code: impl Fn(Option<Paint>) -> String) {
    let columns = usize::from(cel.width.max(1));
    let painted = |row: usize, column: usize| cel.pixels[row * columns + column].is_some();
    let rows = cel.pixels.len() / columns;
    let used_rows: Vec<usize> = (0..rows)
        .filter(|row| (0..columns).any(|column| painted(*row, column)))
        .collect();
    let used_columns: Vec<usize> = (0..columns)
        .filter(|column| (0..rows).any(|row| painted(row, *column)))
        .collect();
    let (top, bottom, left, right) = match (
        used_rows.first(),
        used_rows.last(),
        used_columns.first(),
        used_columns.last(),
    ) {
        (Some(top), Some(bottom), Some(left), Some(right)) => (*top, *bottom, *left, *right),
        _ => (0, 0, 0, 0),
    };
    let offset = |base: i32, by: usize| base + i32::try_from(by).unwrap_or(0);
    let (x, y) = (offset(cel.x, left), offset(cel.y, top));
    text.push_str("cel");
    for (key, value) in [
        ("frame", i64::from(cel.frame)),
        ("x", x.into()),
        ("y", y.into()),
    ] {
        if value != 0 {
            let _ = write!(text, " {key}={value}");
        }
    }
    text.push_str(" \"\"\"\n");
    for row in top..=bottom {
        for column in left..=right {
            let paint = cel.pixels.get(row * columns + column).copied().flatten();
            text.push_str(&code(paint));
        }
        text.push('\n');
    }
    text.push_str("\"\"\"\n");
}
