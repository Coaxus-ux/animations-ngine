use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use kdl::{KdlDocument, KdlNode};

pub const MAX_RECENT: usize = 10;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Recent {
    paths: Vec<PathBuf>,
}

impl Recent {
    pub fn push(&mut self, path: &Path) {
        self.remove(path);
        self.paths.insert(0, path.to_path_buf());
        self.paths.truncate(MAX_RECENT);
    }

    pub fn remove(&mut self, path: &Path) {
        self.paths.retain(|known| known != path);
    }

    pub fn clear(&mut self) {
        self.paths.clear();
    }

    #[must_use]
    pub fn paths(&self) -> &[PathBuf] {
        &self.paths
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }

    #[must_use]
    pub fn to_kdl(&self) -> String {
        let mut text = String::new();
        for path in &self.paths {
            let _ = writeln!(text, "    recent {:?}", path.to_string_lossy());
        }
        text
    }

    #[must_use]
    pub fn from_node(editor: &KdlNode) -> Self {
        let paths = editor
            .children()
            .map(KdlDocument::nodes)
            .unwrap_or_default()
            .iter()
            .filter(|node| node.name().value() == "recent")
            .filter_map(|node| node.entries().first())
            .filter_map(|entry| entry.value().as_string())
            .map(PathBuf::from)
            .take(MAX_RECENT)
            .collect();
        Self { paths }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pushing_a_known_path_moves_it_to_the_front_and_the_list_stays_bounded() {
        let mut recent = Recent::default();
        for n in 0..12 {
            recent.push(Path::new(&format!("{n}.kdl")));
        }
        recent.push(Path::new("3.kdl"));
        assert_eq!(recent.paths().len(), MAX_RECENT);
        assert_eq!(recent.paths()[0], PathBuf::from("3.kdl"));
        assert_eq!(
            recent
                .paths()
                .iter()
                .filter(|p| p.as_path() == Path::new("3.kdl"))
                .count(),
            1
        );
    }

    #[test]
    fn recent_files_round_trip_through_kdl() {
        let mut recent = Recent::default();
        recent.push(Path::new("a/b.kdl"));
        recent.push(Path::new("c d.kdl"));
        let text = format!("editor {{\n{}}}\n", recent.to_kdl());
        let document = KdlDocument::parse_v2(&text).unwrap();
        let editor = &document.nodes()[0];
        assert_eq!(Recent::from_node(editor), recent);
    }
}
