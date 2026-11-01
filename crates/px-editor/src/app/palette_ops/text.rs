use std::path::Path;

use px_core::{MAX_RAMPS, Rgb};

use crate::domain::palette_import::{from_gpl, from_hex};

/// `wanted` when free, else `wanted-2`, `wanted-3`, …
pub(super) fn unique_name(wanted: &str, taken: &[String]) -> String {
    let wanted = if wanted.is_empty() { "ramp" } else { wanted };
    let free = |name: &str| !taken.iter().any(|taken| taken == name);
    if free(wanted) {
        return wanted.to_owned();
    }
    (2..=MAX_RAMPS + 1)
        .map(|n| format!("{wanted}-{n}"))
        .find(|name| free(name))
        .unwrap_or_else(|| wanted.to_owned())
}

/// Colors from a `.hex` or `.gpl` file; an unknown extension tries both.
pub(super) fn parse_palette_file(path: &Path, text: &str) -> Result<Vec<Rgb>, String> {
    let extension = path
        .extension()
        .map(|extension| extension.to_string_lossy().to_ascii_lowercase());
    let colors = match extension.as_deref() {
        Some("hex") => from_hex(text)?,
        Some("gpl") => from_gpl(text)?,
        _ => from_hex(text).or_else(|_| from_gpl(text))?,
    };
    if colors.is_empty() {
        return Err("the file has no colors".to_owned());
    }
    Ok(colors)
}

/// The palette node's text (as `Source::node_text` gives it) with `lines` added before
/// its closing brace, so comments and `twin=` properties survive the import.
pub(super) fn with_ramp_lines(node_text: &str, lines: &str) -> String {
    match node_text.trim_end().rsplit_once('}') {
        Some((head, _)) => format!("{}\n{lines}}}", head.trim_end()),
        None => format!("{} {{\n{lines}}}", node_text.trim_end()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_taken_name_gets_the_first_free_suffix() {
        let taken = ["sky".to_owned(), "sky-2".to_owned()];
        assert_eq!(unique_name("sky", &taken), "sky-3");
        assert_eq!(unique_name("sea", &taken), "sea");
        assert_eq!(unique_name("", &[]), "ramp");
    }

    #[test]
    fn ramp_lines_go_before_the_closing_brace_keeping_the_rest() {
        let node = "palette \"p\" {\n    // kept\n    ramp \"a\" \"#000000\" twin=\"b\"\n}";
        assert_eq!(
            with_ramp_lines(node, "    ramp \"b\" \"#ffffff\"\n"),
            "palette \"p\" {\n    // kept\n    ramp \"a\" \"#000000\" twin=\"b\"\n    ramp \"b\" \"#ffffff\"\n}"
        );
        assert_eq!(
            with_ramp_lines("palette \"p\"", "    ramp \"b\" \"#ffffff\"\n"),
            "palette \"p\" {\n    ramp \"b\" \"#ffffff\"\n}"
        );
    }

    #[test]
    fn the_extension_picks_the_parser_and_unknown_ones_try_both() {
        let hex = "0b0b10\nff9a8a\n";
        let gpl = "GIMP Palette\n0 0 0 black\n";
        assert_eq!(
            parse_palette_file(Path::new("a.HEX"), hex)
                .unwrap_or_default()
                .len(),
            2
        );
        assert_eq!(
            parse_palette_file(Path::new("a.gpl"), gpl)
                .unwrap_or_default()
                .len(),
            1
        );
        assert_eq!(
            parse_palette_file(Path::new("a.txt"), gpl)
                .unwrap_or_default()
                .len(),
            1
        );
        assert!(parse_palette_file(Path::new("a.hex"), gpl).is_err());
        assert!(parse_palette_file(Path::new("a.hex"), "").is_err());
    }
}
