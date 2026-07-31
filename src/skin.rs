use std::{
    fs::File,
    io::{BufReader, BufWriter, Cursor, Read, Write},
    path::{Path, PathBuf},
};

use thiserror::Error;

use crate::{
    atlas::face_region,
    brush::{BrushSize, Stroke, StrokeBuilder},
    model::{BodyPart, Face, Layer, ModelHit, ModelKind, Texel},
};

pub const SKIN_WIDTH: usize = 64;
pub const SKIN_HEIGHT: usize = 64;
const PIXEL_COUNT: usize = SKIN_WIDTH * SKIN_HEIGHT;

#[derive(Debug, Error)]
pub enum SkinError {
    #[error("skin PNG must be 64×64 pixels, but is {width}×{height}")]
    InvalidDimensions { width: u32, height: u32 },
    #[error("unsupported PNG pixel format: {color:?} at {depth:?}")]
    UnsupportedFormat {
        color: png::ColorType,
        depth: png::BitDepth,
    },
    #[error("could not decode PNG: {0}")]
    Decode(#[from] png::DecodingError),
    #[error("could not encode PNG: {0}")]
    Encode(#[from] png::EncodingError),
    #[error("skin file I/O failed: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Skin {
    pixels: Box<[[u8; 4]; PIXEL_COUNT]>,
}

impl Skin {
    pub fn transparent() -> Self {
        Self {
            pixels: Box::new([[0; 4]; PIXEL_COUNT]),
        }
    }

    pub fn blank(kind: ModelKind) -> Self {
        let mut skin = Self::transparent();
        for part in BodyPart::ALL {
            for face in Face::ALL {
                let rect = face_region(kind, part, Layer::Base, face).rect;
                for y in rect.y..rect.y + rect.height {
                    for x in rect.x..rect.x + rect.width {
                        skin.set_pixel(Texel::new(x, y), [255; 4]);
                    }
                }
            }
        }
        skin
    }

    pub fn pixel(&self, texel: Texel) -> [u8; 4] {
        self.pixels[index(texel)]
    }

    pub(crate) fn set_pixel(&mut self, texel: Texel, color: [u8; 4]) {
        self.pixels[index(texel)] = color;
    }

    pub fn pixels(&self) -> &[[u8; 4]; PIXEL_COUNT] {
        &self.pixels
    }

    pub fn from_png(bytes: &[u8]) -> Result<Self, SkinError> {
        let mut decoder = png::Decoder::new(Cursor::new(bytes));
        decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
        let mut reader = decoder.read_info()?;
        if reader.info().width != SKIN_WIDTH as u32 || reader.info().height != SKIN_HEIGHT as u32 {
            return Err(SkinError::InvalidDimensions {
                width: reader.info().width,
                height: reader.info().height,
            });
        }
        let buffer_size = reader
            .output_buffer_size()
            .ok_or(SkinError::UnsupportedFormat {
                color: reader.info().color_type,
                depth: reader.info().bit_depth,
            })?;
        let mut decoded = vec![0; buffer_size];
        let output = reader.next_frame(&mut decoded)?;
        let source = &decoded[..output.buffer_size()];
        let mut skin = Self::transparent();
        match (output.color_type, output.bit_depth) {
            (png::ColorType::Rgba, png::BitDepth::Eight) => {
                for (target, source) in skin.pixels.iter_mut().zip(source.chunks_exact(4)) {
                    target.copy_from_slice(source);
                }
            }
            (png::ColorType::Rgb, png::BitDepth::Eight) => {
                for (target, source) in skin.pixels.iter_mut().zip(source.chunks_exact(3)) {
                    *target = [source[0], source[1], source[2], 255];
                }
            }
            (png::ColorType::Grayscale, png::BitDepth::Eight) => {
                for (target, &value) in skin.pixels.iter_mut().zip(source) {
                    *target = [value, value, value, 255];
                }
            }
            (png::ColorType::GrayscaleAlpha, png::BitDepth::Eight) => {
                for (target, source) in skin.pixels.iter_mut().zip(source.chunks_exact(2)) {
                    *target = [source[0], source[0], source[0], source[1]];
                }
            }
            (color, depth) => return Err(SkinError::UnsupportedFormat { color, depth }),
        }
        Ok(skin)
    }

    pub fn to_png(&self) -> Result<Vec<u8>, SkinError> {
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, SKIN_WIDTH as u32, SKIN_HEIGHT as u32);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header()?;
            let mut rgba = Vec::with_capacity(PIXEL_COUNT * 4);
            for pixel in self.pixels.iter() {
                rgba.extend_from_slice(pixel);
            }
            writer.write_image_data(&rgba)?;
        }
        Ok(bytes)
    }

    pub fn load_png(path: impl AsRef<Path>) -> Result<Self, SkinError> {
        let file = File::open(path)?;
        let mut bytes = Vec::new();
        BufReader::new(file).read_to_end(&mut bytes)?;
        Self::from_png(&bytes)
    }

    pub fn save_png(&self, path: impl AsRef<Path>) -> Result<(), SkinError> {
        let file = File::create(path)?;
        let mut output = BufWriter::new(file);
        output.write_all(&self.to_png()?)?;
        output.flush()?;
        Ok(())
    }
}

fn index(texel: Texel) -> usize {
    usize::from(texel.y) * SKIN_WIDTH + usize::from(texel.x)
}

#[derive(Clone, Debug)]
pub struct SkinDocument {
    skin: Skin,
    path: Option<PathBuf>,
    saved_baseline: Skin,
    undo: Vec<Stroke>,
    redo: Vec<Stroke>,
}

impl SkinDocument {
    pub fn new(kind: ModelKind) -> Self {
        Self::from_skin(Skin::blank(kind), None)
    }

    pub fn from_skin(skin: Skin, path: Option<PathBuf>) -> Self {
        Self {
            saved_baseline: skin.clone(),
            skin,
            path,
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    pub fn load_png(path: impl AsRef<Path>) -> Result<Self, SkinError> {
        let path = path.as_ref();
        Ok(Self::from_skin(
            Skin::load_png(path)?,
            Some(path.to_path_buf()),
        ))
    }

    pub fn save_png(&mut self, path: impl AsRef<Path>) -> Result<(), SkinError> {
        let path = path.as_ref();
        self.skin.save_png(path)?;
        self.path = Some(path.to_path_buf());
        self.mark_saved();
        Ok(())
    }

    pub fn skin(&self) -> &Skin {
        &self.skin
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub fn is_dirty(&self) -> bool {
        self.skin != self.saved_baseline
    }

    pub fn mark_saved(&mut self) {
        self.saved_baseline = self.skin.clone();
    }

    pub fn paint(
        &mut self,
        builder: &mut StrokeBuilder,
        kind: ModelKind,
        hit: ModelHit,
        size: BrushSize,
        color: [u8; 4],
    ) {
        builder.paint(&mut self.skin, kind, hit, size, color);
    }

    pub fn commit_stroke(&mut self, builder: StrokeBuilder) -> bool {
        let stroke = builder.finish();
        if stroke.changes.is_empty() {
            return false;
        }
        self.undo.push(stroke);
        self.redo.clear();
        true
    }

    pub fn undo(&mut self) -> bool {
        let Some(stroke) = self.undo.pop() else {
            return false;
        };
        for change in stroke.changes.iter().rev() {
            self.skin.set_pixel(change.texel, change.before);
        }
        self.redo.push(stroke);
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(stroke) = self.redo.pop() else {
            return false;
        };
        for change in &stroke.changes {
            self.skin.set_pixel(change.texel, change.after);
        }
        self.undo.push(stroke);
        true
    }

    pub fn undo_len(&self) -> usize {
        self.undo.len()
    }

    pub fn redo_len(&self) -> usize {
        self.redo.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn encoded_png(color: png::ColorType, width: u32, height: u32, data: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, width, height);
            encoder.set_color(color);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().unwrap();
            writer.write_image_data(data).unwrap();
        }
        bytes
    }

    fn head_front_hit(texel: Texel) -> ModelHit {
        ModelHit {
            part: BodyPart::Head,
            layer: Layer::Base,
            face: Face::Front,
            distance: 1.0,
            texel,
        }
    }

    #[test]
    fn blank_skin_has_opaque_base_islands_and_transparent_outer_pixels() {
        for kind in [ModelKind::Classic, ModelKind::Slim] {
            let skin = Skin::blank(kind);
            for part in BodyPart::ALL {
                for face in Face::ALL {
                    let base = face_region(kind, part, Layer::Base, face).rect;
                    assert_eq!(skin.pixel(Texel::new(base.x, base.y)), [255; 4]);
                    let outer = face_region(kind, part, Layer::Outer, face).rect;
                    assert_eq!(skin.pixel(Texel::new(outer.x, outer.y))[3], 0);
                }
            }
            assert_eq!(skin.pixel(Texel::new(63, 0))[3], 0);
        }
    }

    #[test]
    fn rgb_and_rgba_decode_to_exact_pixels() {
        let rgb = [10, 20, 30].repeat(PIXEL_COUNT);
        let decoded = Skin::from_png(&encoded_png(png::ColorType::Rgb, 64, 64, &rgb)).unwrap();
        assert_eq!(decoded.pixel(Texel::new(0, 0)), [10, 20, 30, 255]);

        let rgba = [1, 2, 3, 4].repeat(PIXEL_COUNT);
        let decoded = Skin::from_png(&encoded_png(png::ColorType::Rgba, 64, 64, &rgba)).unwrap();
        assert_eq!(decoded.pixel(Texel::new(63, 63)), [1, 2, 3, 4]);
    }

    #[test]
    fn grayscale_indexed_and_sixteen_bit_pngs_expand_to_rgba8() {
        let grayscale = [77].repeat(PIXEL_COUNT);
        let decoded =
            Skin::from_png(&encoded_png(png::ColorType::Grayscale, 64, 64, &grayscale)).unwrap();
        assert_eq!(decoded.pixel(Texel::new(4, 5)), [77, 77, 77, 255]);

        let mut indexed = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut indexed, 64, 64);
            encoder.set_color(png::ColorType::Indexed);
            encoder.set_depth(png::BitDepth::Eight);
            encoder.set_palette(vec![10, 20, 30, 40, 50, 60]);
            encoder.set_trns(vec![70, 80]);
            let mut writer = encoder.write_header().unwrap();
            writer.write_image_data(&[1].repeat(PIXEL_COUNT)).unwrap();
        }
        let decoded = Skin::from_png(&indexed).unwrap();
        assert_eq!(decoded.pixel(Texel::new(0, 0)), [40, 50, 60, 80]);

        let mut sixteen_bit = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut sixteen_bit, 64, 64);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Sixteen);
            let mut writer = encoder.write_header().unwrap();
            let pixel = [0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0];
            writer.write_image_data(&pixel.repeat(PIXEL_COUNT)).unwrap();
        }
        let decoded = Skin::from_png(&sixteen_bit).unwrap();
        assert_eq!(decoded.pixel(Texel::new(63, 63)), [0x12, 0x56, 0x9a, 0xde]);
    }

    #[test]
    fn invalid_dimensions_and_malformed_pngs_are_clear_errors() {
        let data = [0, 0, 0, 255].repeat(32 * 64);
        let error = Skin::from_png(&encoded_png(png::ColorType::Rgba, 32, 64, &data)).unwrap_err();
        assert!(matches!(
            error,
            SkinError::InvalidDimensions {
                width: 32,
                height: 64
            }
        ));
        assert!(matches!(
            Skin::from_png(b"not a png").unwrap_err(),
            SkinError::Decode(_)
        ));
    }

    #[test]
    fn rgba_round_trip_is_byte_accurate_in_memory_and_on_disk() {
        let mut skin = Skin::transparent();
        for y in 0..64 {
            for x in 0..64 {
                skin.set_pixel(
                    Texel::new(x, y),
                    [x, y, x.wrapping_mul(y), x.wrapping_add(y)],
                );
            }
        }
        assert_eq!(Skin::from_png(&skin.to_png().unwrap()).unwrap(), skin);

        let directory = tempdir().unwrap();
        let path = directory.path().join("round-trip.png");
        skin.save_png(&path).unwrap();
        assert_eq!(Skin::load_png(&path).unwrap(), skin);
    }

    #[test]
    fn history_dirty_baseline_and_redo_branching_work_together() {
        let mut document = SkinDocument::new(ModelKind::Classic);
        assert!(!document.is_dirty());

        let mut first = StrokeBuilder::new();
        document.paint(
            &mut first,
            ModelKind::Classic,
            head_front_hit(Texel::new(8, 8)),
            BrushSize::One,
            [1, 2, 3, 4],
        );
        assert!(document.commit_stroke(first));
        assert!(document.is_dirty());
        document.mark_saved();
        assert!(!document.is_dirty());

        let mut second = StrokeBuilder::new();
        document.paint(
            &mut second,
            ModelKind::Classic,
            head_front_hit(Texel::new(9, 8)),
            BrushSize::One,
            [5, 6, 7, 8],
        );
        document.commit_stroke(second);
        assert!(document.is_dirty());
        document.undo();
        assert!(!document.is_dirty());
        assert_eq!(document.redo_len(), 1);

        let mut branch = StrokeBuilder::new();
        document.paint(
            &mut branch,
            ModelKind::Classic,
            head_front_hit(Texel::new(10, 8)),
            BrushSize::One,
            [9, 10, 11, 12],
        );
        document.commit_stroke(branch);
        assert_eq!(document.redo_len(), 0);
        assert!(!document.redo());
    }

    #[test]
    fn document_save_and_load_manage_path_and_clean_baseline() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("document.png");
        let mut document = SkinDocument::new(ModelKind::Slim);
        document.save_png(&path).unwrap();
        assert_eq!(document.path(), Some(path.as_path()));
        assert!(!document.is_dirty());
        let loaded = SkinDocument::load_png(&path).unwrap();
        assert_eq!(loaded.path(), Some(path.as_path()));
        assert!(!loaded.is_dirty());
        assert_eq!(loaded.skin(), document.skin());
    }
}
