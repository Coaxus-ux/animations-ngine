mod hue;

use px_core::Rgb;

pub use hue::ramps_by_hue;

pub fn from_hex(text: &str) -> Result<Vec<Rgb>, String> {
    text.lines()
        .enumerate()
        .map(|(index, line)| (index + 1, line.trim()))
        .filter(|(_, line)| !line.is_empty() && !line.starts_with(';'))
        .map(|(number, line)| {
            let digits = line.strip_prefix('#').unwrap_or(line);
            Rgb::parse_hex(&format!("#{digits}")).map_err(|error| format!("line {number}: {error}"))
        })
        .collect()
}

pub fn from_gpl(text: &str) -> Result<Vec<Rgb>, String> {
    let mut lines = text
        .lines()
        .enumerate()
        .map(|(index, line)| (index + 1, line.trim()))
        .filter(|(_, line)| !line.is_empty());
    match lines.next() {
        Some((_, "GIMP Palette")) => {}
        _ => return Err("not a GIMP palette: the first line must be `GIMP Palette`".to_owned()),
    }
    lines
        .filter(|(_, line)| !(line.starts_with('#') || is_header(line)))
        .map(|(number, line)| {
            let mut words = line.split_whitespace();
            let mut channel = || {
                words
                    .next()
                    .and_then(|word| word.parse::<u8>().ok())
                    .ok_or_else(|| format!("line {number}: expected `R G B name`, found `{line}`"))
            };
            Ok(Rgb::new(channel()?, channel()?, channel()?))
        })
        .collect()
}

fn is_header(line: &str) -> bool {
    line.split_whitespace()
        .next()
        .is_some_and(|word| word.ends_with(':'))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "tests fail loudly")]

    use super::*;

    #[test]
    fn a_hex_file_gives_one_color_per_line_ignoring_comments_and_blanks() {
        let text = "; Lospec palette\n0b0b10\n\n#FF9A8A\n";
        assert_eq!(
            from_hex(text).unwrap(),
            vec![Rgb::new(0x0b, 0x0b, 0x10), Rgb::new(0xff, 0x9a, 0x8a)]
        );
    }

    #[test]
    fn a_bad_hex_line_is_reported_with_its_number() {
        let error = from_hex("0b0b10\nzz0000\n").unwrap_err();
        assert!(error.starts_with("line 2:"), "{error}");
    }

    #[test]
    fn a_gpl_file_gives_its_rgb_rows() {
        let text = "GIMP Palette\nName: Mini\nColumns: 2\nChannels: RGBA\n#\n  0   0   0\tblack\n255 154 138 salmon\n";
        assert_eq!(
            from_gpl(text).unwrap(),
            vec![Rgb::new(0, 0, 0), Rgb::new(255, 154, 138)]
        );
        assert!(from_gpl("0 0 0\n").is_err());
        let error = from_gpl("GIMP Palette\n0 0 300 x\n").unwrap_err();
        assert!(error.starts_with("line 2:"), "{error}");
    }
}
