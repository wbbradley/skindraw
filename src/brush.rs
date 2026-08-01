use std::collections::HashMap;

use crate::{
    atlas::{AtlasRect, face_region},
    model::{BodyPart, Face, Layer, ModelHit, ModelKind, Texel},
    skin::Skin,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BrushSize {
    One,
    Two,
    Four,
}

impl BrushSize {
    pub const fn pixels(self) -> u8 {
        match self {
            Self::One => 1,
            Self::Two => 2,
            Self::Four => 4,
        }
    }
}

pub fn brush_footprint(kind: ModelKind, hit: ModelHit, size: BrushSize) -> Vec<Texel> {
    let rect = face_region(kind, hit.part, hit.layer, hit.face).rect;
    clipped_footprint(rect, hit.texel, size)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct PixelChange {
    pub texel: Texel,
    pub before: [u8; 4],
    pub after: [u8; 4],
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct Stroke {
    pub changes: Vec<PixelChange>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct FaceKey {
    part: BodyPart,
    layer: Layer,
    face: Face,
}

#[derive(Debug, Default)]
pub struct StrokeBuilder {
    stroke: Stroke,
    change_indices: HashMap<Texel, usize>,
    previous: Option<(FaceKey, Texel)>,
}

impl StrokeBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn paint(
        &mut self,
        skin: &mut Skin,
        kind: ModelKind,
        hit: ModelHit,
        size: BrushSize,
        color: [u8; 4],
    ) {
        let key = FaceKey {
            part: hit.part,
            layer: hit.layer,
            face: hit.face,
        };
        let rect = face_region(kind, hit.part, hit.layer, hit.face).rect;
        if let Some((previous_key, previous_texel)) = self.previous {
            if previous_key == key {
                if previous_texel == hit.texel {
                    self.stamp(skin, rect, hit.texel, size, color);
                } else {
                    for texel in line(previous_texel, hit.texel).skip(1) {
                        self.stamp(skin, rect, texel, size, color);
                    }
                }
            } else {
                self.stamp(skin, rect, hit.texel, size, color);
            }
        } else {
            self.stamp(skin, rect, hit.texel, size, color);
        }
        self.previous = Some((key, hit.texel));
    }

    pub fn changed_pixel_count(&self) -> usize {
        self.stroke
            .changes
            .iter()
            .filter(|change| change.before != change.after)
            .count()
    }

    pub(crate) fn finish(mut self) -> Stroke {
        self.stroke
            .changes
            .retain(|change| change.before != change.after);
        self.stroke
    }

    fn stamp(
        &mut self,
        skin: &mut Skin,
        rect: AtlasRect,
        center: Texel,
        size: BrushSize,
        color: [u8; 4],
    ) {
        for texel in clipped_footprint(rect, center, size) {
            let before = skin.pixel(texel);
            if before == color {
                continue;
            }
            skin.set_pixel(texel, color);
            if let Some(&index) = self.change_indices.get(&texel) {
                self.stroke.changes[index].after = color;
            } else {
                let index = self.stroke.changes.len();
                self.stroke.changes.push(PixelChange {
                    texel,
                    before,
                    after: color,
                });
                self.change_indices.insert(texel, index);
            }
        }
    }
}

fn clipped_footprint(rect: AtlasRect, center: Texel, size: BrushSize) -> Vec<Texel> {
    let pixels = i16::from(size.pixels());
    let offset = (pixels - 1) / 2;
    let mut footprint = Vec::with_capacity(usize::from(size.pixels()).pow(2));
    for y_offset in 0..pixels {
        for x_offset in 0..pixels {
            let x = i16::from(center.x) - offset + x_offset;
            let y = i16::from(center.y) - offset + y_offset;
            if !(0..64).contains(&x) || !(0..64).contains(&y) {
                continue;
            }
            let texel = Texel::new(x as u8, y as u8);
            if rect.contains(texel) {
                footprint.push(texel);
            }
        }
    }
    footprint
}

struct TexelLine {
    current_x: i16,
    current_y: i16,
    end_x: i16,
    end_y: i16,
    delta_x: i16,
    step_x: i16,
    delta_y: i16,
    step_y: i16,
    error: i16,
    finished: bool,
}

fn line(start: Texel, end: Texel) -> TexelLine {
    let current_x = i16::from(start.x);
    let current_y = i16::from(start.y);
    let end_x = i16::from(end.x);
    let end_y = i16::from(end.y);
    let delta_x = (end_x - current_x).abs();
    let delta_y = -(end_y - current_y).abs();
    TexelLine {
        current_x,
        current_y,
        end_x,
        end_y,
        delta_x,
        step_x: if current_x < end_x { 1 } else { -1 },
        delta_y,
        step_y: if current_y < end_y { 1 } else { -1 },
        error: delta_x + delta_y,
        finished: false,
    }
}

impl Iterator for TexelLine {
    type Item = Texel;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }
        let result = Texel::new(self.current_x as u8, self.current_y as u8);
        if self.current_x == self.end_x && self.current_y == self.end_y {
            self.finished = true;
            return Some(result);
        }
        let doubled_error = self.error * 2;
        if doubled_error >= self.delta_y {
            self.error += self.delta_y;
            self.current_x += self.step_x;
        }
        if doubled_error <= self.delta_x {
            self.error += self.delta_x;
            self.current_y += self.step_y;
        }
        Some(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skin::SkinDocument;

    fn hit(part: BodyPart, face: Face, texel: Texel) -> ModelHit {
        ModelHit {
            part,
            layer: Layer::Base,
            face,
            distance: 1.0,
            texel,
        }
    }

    #[test]
    fn brush_sizes_clip_to_the_current_face() {
        let region = face_region(ModelKind::Classic, BodyPart::Head, Layer::Base, Face::Front);
        for (size, expected) in [
            (BrushSize::One, 1),
            (BrushSize::Two, 4),
            (BrushSize::Four, 9),
        ] {
            let mut skin = Skin::transparent();
            let mut stroke = StrokeBuilder::new();
            stroke.paint(
                &mut skin,
                ModelKind::Classic,
                hit(BodyPart::Head, Face::Front, Texel::new(8, 8)),
                size,
                [1, 2, 3, 4],
            );
            assert_eq!(stroke.changed_pixel_count(), expected);
            assert!(
                stroke
                    .stroke
                    .changes
                    .iter()
                    .all(|change| region.rect.contains(change.texel))
            );
        }
    }

    #[test]
    fn preview_footprint_matches_even_size_anchoring_and_face_clipping() {
        let front = hit(BodyPart::Head, Face::Front, Texel::new(10, 10));
        assert_eq!(
            brush_footprint(ModelKind::Classic, front, BrushSize::Two),
            [
                Texel::new(10, 10),
                Texel::new(11, 10),
                Texel::new(10, 11),
                Texel::new(11, 11),
            ]
        );
        let corner = hit(BodyPart::Head, Face::Front, Texel::new(8, 8));
        assert_eq!(
            brush_footprint(ModelKind::Classic, corner, BrushSize::Four),
            [
                Texel::new(8, 8),
                Texel::new(9, 8),
                Texel::new(10, 8),
                Texel::new(8, 9),
                Texel::new(9, 9),
                Texel::new(10, 9),
                Texel::new(8, 10),
                Texel::new(9, 10),
                Texel::new(10, 10),
            ]
        );
    }

    #[test]
    fn fast_drag_interpolates_and_crossing_faces_does_not() {
        let mut skin = Skin::transparent();
        let mut stroke = StrokeBuilder::new();
        stroke.paint(
            &mut skin,
            ModelKind::Classic,
            hit(BodyPart::Head, Face::Front, Texel::new(8, 8)),
            BrushSize::One,
            [9, 0, 0, 255],
        );
        stroke.paint(
            &mut skin,
            ModelKind::Classic,
            hit(BodyPart::Head, Face::Front, Texel::new(15, 8)),
            BrushSize::One,
            [9, 0, 0, 255],
        );
        assert_eq!(stroke.changed_pixel_count(), 8);
        stroke.paint(
            &mut skin,
            ModelKind::Classic,
            hit(BodyPart::Head, Face::Back, Texel::new(31, 8)),
            BrushSize::One,
            [9, 0, 0, 255],
        );
        assert_eq!(stroke.changed_pixel_count(), 9);
    }

    #[test]
    fn duplicate_pixels_aggregate_and_transparency_is_a_color() {
        let mut skin = Skin::blank(ModelKind::Classic);
        let mut stroke = StrokeBuilder::new();
        let target = hit(BodyPart::Head, Face::Front, Texel::new(8, 8));
        stroke.paint(
            &mut skin,
            ModelKind::Classic,
            target,
            BrushSize::One,
            [10, 20, 30, 255],
        );
        stroke.paint(
            &mut skin,
            ModelKind::Classic,
            target,
            BrushSize::One,
            [0, 0, 0, 0],
        );
        assert_eq!(stroke.stroke.changes.len(), 1);
        assert_eq!(stroke.stroke.changes[0].before, [255; 4]);
        assert_eq!(stroke.stroke.changes[0].after, [0; 4]);
    }

    #[test]
    fn outer_layer_accepts_opaque_and_transparent_replacement() {
        let mut skin = Skin::blank(ModelKind::Classic);
        let target = ModelHit {
            part: BodyPart::Torso,
            layer: Layer::Outer,
            face: Face::Front,
            distance: 1.0,
            texel: Texel::new(20, 36),
        };
        let mut stroke = StrokeBuilder::new();
        stroke.paint(
            &mut skin,
            ModelKind::Classic,
            target,
            BrushSize::One,
            [12, 34, 56, 200],
        );
        stroke.paint(
            &mut skin,
            ModelKind::Classic,
            target,
            BrushSize::One,
            [0; 4],
        );
        assert_eq!(skin.pixel(target.texel), [0; 4]);
        assert_eq!(stroke.changed_pixel_count(), 0);
        assert!(stroke.finish().changes.is_empty());
    }

    #[test]
    fn one_builder_commits_as_one_document_history_entry() {
        let mut document = SkinDocument::new(ModelKind::Classic);
        let mut stroke = StrokeBuilder::new();
        for x in 8..12 {
            document.paint(
                &mut stroke,
                ModelKind::Classic,
                hit(BodyPart::Head, Face::Front, Texel::new(x, 8)),
                BrushSize::One,
                [4, 3, 2, 1],
            );
        }
        assert!(document.commit_stroke(stroke));
        assert_eq!(document.undo_len(), 1);
        assert!(document.undo());
        assert_eq!(document.skin().pixel(Texel::new(8, 8)), [255; 4]);
    }
}
