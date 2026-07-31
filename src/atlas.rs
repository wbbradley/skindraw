use crate::model::{BodyPart, Face, Layer, ModelKind, Texel};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AtlasRect {
    pub x: u8,
    pub y: u8,
    pub width: u8,
    pub height: u8,
}

impl AtlasRect {
    pub const fn contains(self, texel: Texel) -> bool {
        texel.x >= self.x
            && texel.y >= self.y
            && texel.x < self.x + self.width
            && texel.y < self.y + self.height
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FaceRegion {
    pub rect: AtlasRect,
    pub flip_u: bool,
    pub flip_v: bool,
}

impl FaceRegion {
    pub fn texel(self, local_u: u8, local_v: u8) -> Option<Texel> {
        if local_u >= self.rect.width || local_v >= self.rect.height {
            return None;
        }
        let u = if self.flip_u {
            self.rect.width - 1 - local_u
        } else {
            local_u
        };
        let v = if self.flip_v {
            self.rect.height - 1 - local_v
        } else {
            local_v
        };
        Some(Texel::new(self.rect.x + u, self.rect.y + v))
    }
}

#[derive(Clone, Copy)]
struct CuboidUv {
    u: u8,
    v: u8,
    width: u8,
    height: u8,
    depth: u8,
}

pub fn face_region(kind: ModelKind, part: BodyPart, layer: Layer, face: Face) -> FaceRegion {
    let cuboid = cuboid_uv(kind, part, layer);
    let (x, y, width, height, flip_u, flip_v) = match face {
        Face::Top => (
            cuboid.u + cuboid.depth,
            cuboid.v,
            cuboid.width,
            cuboid.depth,
            false,
            false,
        ),
        Face::Bottom => (
            cuboid.u + cuboid.depth + cuboid.width,
            cuboid.v,
            cuboid.width,
            cuboid.depth,
            false,
            false,
        ),
        Face::Right => (
            cuboid.u,
            cuboid.v + cuboid.depth,
            cuboid.depth,
            cuboid.height,
            false,
            false,
        ),
        Face::Front => (
            cuboid.u + cuboid.depth,
            cuboid.v + cuboid.depth,
            cuboid.width,
            cuboid.height,
            false,
            false,
        ),
        Face::Left => (
            cuboid.u + cuboid.depth + cuboid.width,
            cuboid.v + cuboid.depth,
            cuboid.depth,
            cuboid.height,
            true,
            false,
        ),
        Face::Back => (
            cuboid.u + cuboid.depth * 2 + cuboid.width,
            cuboid.v + cuboid.depth,
            cuboid.width,
            cuboid.height,
            true,
            false,
        ),
    };
    FaceRegion {
        rect: AtlasRect {
            x,
            y,
            width,
            height,
        },
        flip_u,
        flip_v,
    }
}

fn cuboid_uv(kind: ModelKind, part: BodyPart, layer: Layer) -> CuboidUv {
    let arm_width = if kind == ModelKind::Slim { 3 } else { 4 };
    let (u, v, width, height, depth) = match (part, layer) {
        (BodyPart::Head, Layer::Base) => (0, 0, 8, 8, 8),
        (BodyPart::Head, Layer::Outer) => (32, 0, 8, 8, 8),
        (BodyPart::Torso, Layer::Base) => (16, 16, 8, 12, 4),
        (BodyPart::Torso, Layer::Outer) => (16, 32, 8, 12, 4),
        (BodyPart::RightArm, Layer::Base) => (40, 16, arm_width, 12, 4),
        (BodyPart::RightArm, Layer::Outer) => (40, 32, arm_width, 12, 4),
        (BodyPart::LeftArm, Layer::Base) => (32, 48, arm_width, 12, 4),
        (BodyPart::LeftArm, Layer::Outer) => (48, 48, arm_width, 12, 4),
        (BodyPart::RightLeg, Layer::Base) => (0, 16, 4, 12, 4),
        (BodyPart::RightLeg, Layer::Outer) => (0, 32, 4, 12, 4),
        (BodyPart::LeftLeg, Layer::Base) => (16, 48, 4, 12, 4),
        (BodyPart::LeftLeg, Layer::Outer) => (0, 48, 4, 12, 4),
    };
    CuboidUv {
        u,
        v,
        width,
        height,
        depth,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_region_matches_independent_cuboid_layout_table() {
        let cases = [
            (BodyPart::Head, Layer::Base, 0, 0, 8, 8, 8),
            (BodyPart::Head, Layer::Outer, 32, 0, 8, 8, 8),
            (BodyPart::Torso, Layer::Base, 16, 16, 8, 12, 4),
            (BodyPart::Torso, Layer::Outer, 16, 32, 8, 12, 4),
            (BodyPart::RightArm, Layer::Base, 40, 16, 4, 12, 4),
            (BodyPart::RightArm, Layer::Outer, 40, 32, 4, 12, 4),
            (BodyPart::LeftArm, Layer::Base, 32, 48, 4, 12, 4),
            (BodyPart::LeftArm, Layer::Outer, 48, 48, 4, 12, 4),
            (BodyPart::RightLeg, Layer::Base, 0, 16, 4, 12, 4),
            (BodyPart::RightLeg, Layer::Outer, 0, 32, 4, 12, 4),
            (BodyPart::LeftLeg, Layer::Base, 16, 48, 4, 12, 4),
            (BodyPart::LeftLeg, Layer::Outer, 0, 48, 4, 12, 4),
        ];
        for kind in [ModelKind::Classic, ModelKind::Slim] {
            for &(part, layer, u, v, classic_width, height, depth) in &cases {
                let width = if kind == ModelKind::Slim
                    && matches!(part, BodyPart::RightArm | BodyPart::LeftArm)
                {
                    3
                } else {
                    classic_width
                };
                let expected = [
                    (Face::Top, u + depth, v, width, depth, false, false),
                    (
                        Face::Bottom,
                        u + depth + width,
                        v,
                        width,
                        depth,
                        false,
                        false,
                    ),
                    (Face::Right, u, v + depth, depth, height, false, false),
                    (
                        Face::Front,
                        u + depth,
                        v + depth,
                        width,
                        height,
                        false,
                        false,
                    ),
                    (
                        Face::Left,
                        u + depth + width,
                        v + depth,
                        depth,
                        height,
                        true,
                        false,
                    ),
                    (
                        Face::Back,
                        u + depth * 2 + width,
                        v + depth,
                        width,
                        height,
                        true,
                        false,
                    ),
                ];
                for &(face, x, y, face_width, face_height, flip_u, flip_v) in &expected {
                    let region = face_region(kind, part, layer, face);
                    assert_eq!(
                        region,
                        FaceRegion {
                            rect: AtlasRect {
                                x,
                                y,
                                width: face_width,
                                height: face_height
                            },
                            flip_u,
                            flip_v
                        },
                        "{kind:?} {part:?} {layer:?} {face:?}"
                    );
                    assert_eq!(
                        region.texel(0, 0),
                        Some(Texel::new(
                            x + if flip_u { face_width - 1 } else { 0 },
                            y + if flip_v { face_height - 1 } else { 0 }
                        ))
                    );
                    assert!(region.texel(face_width, 0).is_none());
                }
            }
        }
    }
}
