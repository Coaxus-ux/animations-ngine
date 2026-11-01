use crate::json::JsonWriter;
use crate::sheet::CellRect;

const APP: &str = "https://github.com/px/pxe";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SheetMeta<'a> {
    pub title: &'a str,
    pub image: &'a str,
    pub size: (u32, u32),
    pub scale: u8,
    pub frame: (u16, u16),
    pub cells: &'a [CellRect],
    pub duration_ms: u32,
    pub layers: &'a [&'a str],
    pub trim: Option<Trim>,
    pub tags: &'a [FrameTag<'a>],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Trim {
    pub offset: (u32, u32),
    pub source: (u16, u16),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameTag<'a> {
    pub name: &'a str,
    pub from: u32,
    pub to: u32,
}

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
        json.bool(meta.trim.is_some());
        let (offset, source) = meta
            .trim
            .map_or(((0, 0), meta.frame), |trim| (trim.offset, trim.source));
        json.key("spriteSourceSize");
        rect(
            &mut json,
            scaled(offset.0),
            scaled(offset.1),
            frame_w,
            frame_h,
        );
        json.key("sourceSize");
        size(
            &mut json,
            scaled(u32::from(source.0)),
            scaled(u32::from(source.1)),
        );
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
    for tag in meta.tags {
        json.begin_object();
        json.key("name");
        json.string(tag.name);
        json.key("from");
        json.int(i64::from(tag.from));
        json.key("to");
        json.int(i64::from(tag.to));
        json.key("direction");
        json.string("forward");
        json.key("color");
        json.string("#000000ff");
        json.key("repeat");
        json.string("0");
        json.end_object();
    }
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
