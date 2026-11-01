use crate::json::JsonWriter;
use crate::sheet::CellRect;

const APP: &str = "https://github.com/px/pxe";

/// Sizes and cells are unscaled pixels; the JSON multiplies them by `scale`, matching the
/// sheet image written at that scale.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SheetMeta<'a> {
    /// The scene name, used as the frame key prefix.
    pub title: &'a str,
    /// The sheet image's file name.
    pub image: &'a str,
    pub size: (u32, u32),
    pub scale: u8,
    pub frame: (u16, u16),
    pub cells: &'a [CellRect],
    pub duration_ms: u32,
    pub layers: &'a [&'a str],
}

/// Aseprite's sprite-sheet JSON (hash format), which Godot, Unity and Phaser importers read.
#[must_use]
pub fn aseprite_json(meta: &SheetMeta<'_>) -> String {
    let scaled = |value: u32| i64::from(value) * i64::from(meta.scale);
    let (frame_w, frame_h) = (
        scaled(u32::from(meta.frame.0)),
        scaled(u32::from(meta.frame.1)),
    );
    let mut json = JsonWriter::new();
    json.begin_object();
    json.key("frames");
    json.begin_object();
    for (index, cell) in meta.cells.iter().enumerate() {
        json.key(&format!("{} {index}.kdl", meta.title));
        json.begin_object();
        json.key("frame");
        rect(
            &mut json,
            scaled(cell.x),
            scaled(cell.y),
            scaled(cell.w),
            scaled(cell.h),
        );
        json.key("rotated");
        json.bool(false);
        json.key("trimmed");
        json.bool(false);
        json.key("spriteSourceSize");
        rect(&mut json, 0, 0, frame_w, frame_h);
        json.key("sourceSize");
        size(&mut json, frame_w, frame_h);
        json.key("duration");
        json.int(i64::from(meta.duration_ms));
        json.end_object();
    }
    json.end_object();
    json.key("meta");
    json.begin_object();
    json.key("app");
    json.string(APP);
    json.key("version");
    json.string(env!("CARGO_PKG_VERSION"));
    json.key("image");
    json.string(meta.image);
    json.key("format");
    json.string("RGBA8888");
    json.key("size");
    size(&mut json, scaled(meta.size.0), scaled(meta.size.1));
    json.key("scale");
    json.string(&meta.scale.to_string());
    json.key("frameTags");
    json.begin_array();
    json.end_array();
    json.key("layers");
    json.begin_array();
    for layer in meta.layers {
        json.begin_object();
        json.key("name");
        json.string(layer);
        json.key("opacity");
        json.int(255);
        json.key("blendMode");
        json.string("normal");
        json.end_object();
    }
    json.end_array();
    json.key("slices");
    json.begin_array();
    json.end_array();
    json.end_object();
    json.end_object();
    json.finish()
}

fn rect(json: &mut JsonWriter, x: i64, y: i64, w: i64, h: i64) {
    json.begin_object();
    json.key("x");
    json.int(x);
    json.key("y");
    json.int(y);
    json.key("w");
    json.int(w);
    json.key("h");
    json.int(h);
    json.end_object();
}

fn size(json: &mut JsonWriter, w: i64, h: i64) {
    json.begin_object();
    json.key("w");
    json.int(w);
    json.key("h");
    json.int(h);
    json.end_object();
}
