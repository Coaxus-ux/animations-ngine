use std::io::Cursor;

use png::{ColorType, Decoder, Transformations};
use px_core::{MAX_CANVAS_SIDE, Rgb};

pub(super) struct Image {
    pub(super) width: usize,
    pub(super) pixels: Vec<Option<Rgb>>,
}

impl Image {
    pub(super) fn decode(bytes: &[u8]) -> Result<Self, String> {
        let mut decoder = Decoder::new(Cursor::new(bytes));
        decoder.set_transformations(Transformations::normalize_to_color8());
        let mut reader = decoder.read_info().map_err(|error| error.to_string())?;
        let size = reader
            .output_buffer_size()
            .ok_or("the image is too large to decode")?;
        let mut buffer = vec![0; size];
        let info = reader
            .next_frame(&mut buffer)
            .map_err(|error| error.to_string())?;
        let fits = |side: u32| u16::try_from(side).is_ok_and(|side| side <= MAX_CANVAS_SIDE);
        if !fits(info.width) || !fits(info.height) {
            return Err(format!(
                "the image is {}×{}; each side may be at most {MAX_CANVAS_SIDE}",
                info.width, info.height
            ));
        }
        let channels = info.color_type.samples();
        let width = usize::try_from(info.width).map_err(|error| error.to_string())?;
        let pixels = buffer[..info.buffer_size()]
            .chunks_exact(channels)
            .enumerate()
            .map(|(index, sample)| pixel(info.color_type, sample, index, width))
            .collect::<Result<_, _>>()?;
        Ok(Self { width, pixels })
    }
}

fn pixel(
    color: ColorType,
    sample: &[u8],
    index: usize,
    width: usize,
) -> Result<Option<Rgb>, String> {
    let (rgb, alpha) = match color {
        ColorType::Grayscale => (Rgb::new(sample[0], sample[0], sample[0]), u8::MAX),
        ColorType::GrayscaleAlpha => (Rgb::new(sample[0], sample[0], sample[0]), sample[1]),
        ColorType::Rgb | ColorType::Indexed => (Rgb::new(sample[0], sample[1], sample[2]), u8::MAX),
        ColorType::Rgba => (Rgb::new(sample[0], sample[1], sample[2]), sample[3]),
    };
    match alpha {
        0 => Ok(None),
        u8::MAX => Ok(Some(rgb)),
        _ => Err(format!(
            "pixel ({}, {}) is partly transparent; pixels are opaque or empty",
            index % width,
            index / width
        )),
    }
}
