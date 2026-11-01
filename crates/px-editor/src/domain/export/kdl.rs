use std::fmt::Write as _;
use std::path::PathBuf;

use kdl::KdlNode;
use px_export::Padding;

use super::{ExportJob, ExportKind, FrameScope, layout_count, layout_from, layout_index};

pub const KIND_NAMES: [&str; 4] = ["gif", "png", "sequence", "sheet"];
pub const LAYOUT_NAMES: [&str; 4] = ["strip-h", "strip-v", "rows", "columns"];

impl ExportJob {
    #[must_use]
    pub fn to_kdl(&self) -> String {
        let (layout, padding, json) = match self.kind {
            ExportKind::Sheet {
                layout,
                padding,
                json,
            } => (layout, padding, json),
            _ => (layout_from(0, 1), Padding::default(), false),
        };
        let mut text = format!(
            "    export kind={:?} layout={:?} count={} border={} shape={} json=#{} scale={} transparent=#{} trim=#{} split-layers=#{}",
            KIND_NAMES[self.kind.index()],
            LAYOUT_NAMES[layout_index(layout)],
            layout_count(layout),
            padding.border,
            padding.shape,
            json,
            self.scale,
            self.transparent,
            self.trim,
            self.split_layers,
        );
        match self.frames {
            FrameScope::All => text.push_str(" frames=\"all\""),
            FrameScope::Current(frame) => {
                let _ = write!(text, " frames={frame}");
            }
        }
        let _ = writeln!(text, " out={:?}", self.out.to_string_lossy());
        text
    }

    #[must_use]
    pub fn from_node(node: &KdlNode) -> Option<Self> {
        let string = |key: &str| node.entry(key).and_then(|entry| entry.value().as_string());
        let integer = |key: &str| node.entry(key).and_then(|entry| entry.value().as_integer());
        let flag = |key: &str| node.entry(key).and_then(|entry| entry.value().as_bool());
        let kind_index = KIND_NAMES
            .iter()
            .position(|name| Some(*name) == string("kind"))?;
        let layout_index = LAYOUT_NAMES
            .iter()
            .position(|name| Some(*name) == string("layout"))
            .unwrap_or(0);
        let small = |key: &str| integer(key).and_then(|value| u16::try_from(value).ok());
        let layout = layout_from(layout_index, small("count").unwrap_or(1));
        let padding = Padding {
            border: small("border").unwrap_or(0),
            shape: small("shape").unwrap_or(0),
        };
        let frames = match node.entry("frames").map(kdl::KdlEntry::value) {
            Some(value) if value.as_string() == Some("all") => FrameScope::All,
            Some(value) => FrameScope::Current(
                value
                    .as_integer()
                    .and_then(|frame| u32::try_from(frame).ok())?,
            ),
            None => FrameScope::All,
        };
        Some(Self {
            kind: ExportKind::from_index(kind_index, layout, padding, flag("json").unwrap_or(true)),
            scale: integer("scale")
                .and_then(|scale| u8::try_from(scale).ok())
                .unwrap_or(1),
            transparent: flag("transparent").unwrap_or(true),
            trim: flag("trim").unwrap_or(false),
            split_layers: flag("split-layers").unwrap_or(false),
            frames,
            out: PathBuf::from(string("out").unwrap_or_default()),
        })
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]
mod tests {
    use kdl::KdlDocument;
    use px_export::SheetLayout;

    use super::*;

    fn round_trip(job: &ExportJob) -> ExportJob {
        let text = format!("editor {{\n{}}}\n", job.to_kdl());
        let document = KdlDocument::parse_v2(&text).unwrap();
        let node = &document.nodes()[0].children().unwrap().nodes()[0];
        ExportJob::from_node(node).unwrap()
    }

    #[test]
    fn a_sheet_job_survives_the_layout_file() {
        let job = ExportJob {
            kind: ExportKind::Sheet {
                layout: SheetLayout::Rows { columns: 4 },
                padding: Padding {
                    border: 2,
                    shape: 1,
                },
                json: false,
            },
            scale: 3,
            transparent: false,
            trim: true,
            split_layers: true,
            frames: FrameScope::Current(7),
            out: PathBuf::from("out/hero sheet.png"),
        };
        assert_eq!(round_trip(&job), job);
    }

    #[test]
    fn a_gif_job_survives_the_layout_file() {
        let job = ExportJob {
            kind: ExportKind::Gif,
            scale: 2,
            transparent: true,
            trim: false,
            split_layers: false,
            frames: FrameScope::All,
            out: PathBuf::from("room.gif"),
        };
        assert_eq!(round_trip(&job), job);
        assert_eq!(round_trip(&ExportJob::default()), ExportJob::default());
    }
}
