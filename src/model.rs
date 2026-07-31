use glam::Vec3;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ModelKind {
    Classic,
    Slim,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum BodyPart {
    Head,
    Torso,
    RightArm,
    LeftArm,
    RightLeg,
    LeftLeg,
}

impl BodyPart {
    pub const ALL: [Self; 6] = [
        Self::Head,
        Self::Torso,
        Self::RightArm,
        Self::LeftArm,
        Self::RightLeg,
        Self::LeftLeg,
    ];
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Layer {
    Base,
    Outer,
}

impl Layer {
    pub const ALL: [Self; 2] = [Self::Base, Self::Outer];
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Face {
    /// The surface facing +Z.
    Front,
    /// The surface facing -Z.
    Back,
    /// The character's left surface, facing +X.
    Left,
    /// The character's right surface, facing -X.
    Right,
    /// The surface facing +Y.
    Top,
    /// The surface facing -Y.
    Bottom,
}

impl Face {
    pub const ALL: [Self; 6] = [
        Self::Front,
        Self::Back,
        Self::Left,
        Self::Right,
        Self::Top,
        Self::Bottom,
    ];
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Texel {
    pub x: u8,
    pub y: u8,
}

impl Texel {
    pub const fn new(x: u8, y: u8) -> Self {
        Self { x, y }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

impl Ray {
    pub fn new(origin: Vec3, direction: Vec3) -> Self {
        Self {
            origin,
            direction: direction.normalize_or_zero(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ModelBox {
    pub part: BodyPart,
    pub layer: Layer,
    pub min: Vec3,
    pub max: Vec3,
}

impl ModelBox {
    pub fn size(self) -> Vec3 {
        self.max - self.min
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ModelHit {
    pub part: BodyPart,
    pub layer: Layer,
    pub face: Face,
    pub distance: f32,
    pub texel: Texel,
}

pub fn model_boxes(kind: ModelKind) -> Vec<ModelBox> {
    let arm_width = match kind {
        ModelKind::Classic => 4.0,
        ModelKind::Slim => 3.0,
    };
    let bases = [
        (
            BodyPart::Head,
            Vec3::new(-4.0, 24.0, -4.0),
            Vec3::new(4.0, 32.0, 4.0),
        ),
        (
            BodyPart::Torso,
            Vec3::new(-4.0, 12.0, -2.0),
            Vec3::new(4.0, 24.0, 2.0),
        ),
        (
            BodyPart::RightArm,
            Vec3::new(-4.0 - arm_width, 12.0, -2.0),
            Vec3::new(-4.0, 24.0, 2.0),
        ),
        (
            BodyPart::LeftArm,
            Vec3::new(4.0, 12.0, -2.0),
            Vec3::new(4.0 + arm_width, 24.0, 2.0),
        ),
        (
            BodyPart::RightLeg,
            Vec3::new(-4.0, 0.0, -2.0),
            Vec3::new(0.0, 12.0, 2.0),
        ),
        (
            BodyPart::LeftLeg,
            Vec3::new(0.0, 0.0, -2.0),
            Vec3::new(4.0, 12.0, 2.0),
        ),
    ];

    let mut boxes = Vec::with_capacity(12);
    for (part, min, max) in bases {
        boxes.push(ModelBox {
            part,
            layer: Layer::Base,
            min,
            max,
        });
        let inflation = if part == BodyPart::Head { 0.5 } else { 0.25 };
        boxes.push(ModelBox {
            part,
            layer: Layer::Outer,
            min: min - Vec3::splat(inflation),
            max: max + Vec3::splat(inflation),
        });
    }
    boxes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_boxes_have_expected_arm_widths_and_inflation() {
        for (kind, width) in [(ModelKind::Classic, 4.0), (ModelKind::Slim, 3.0)] {
            let boxes = model_boxes(kind);
            assert_eq!(boxes.len(), 12);
            let arm = boxes
                .iter()
                .find(|item| item.part == BodyPart::RightArm && item.layer == Layer::Base)
                .unwrap();
            assert_eq!(arm.size(), Vec3::new(width, 12.0, 4.0));
            let sleeve = boxes
                .iter()
                .find(|item| item.part == BodyPart::RightArm && item.layer == Layer::Outer)
                .unwrap();
            assert_eq!(sleeve.size(), Vec3::new(width + 0.5, 12.5, 4.5));
        }

        let boxes = model_boxes(ModelKind::Classic);
        let hat = boxes
            .iter()
            .find(|item| item.part == BodyPart::Head && item.layer == Layer::Outer)
            .unwrap();
        assert_eq!(hat.min, Vec3::new(-4.5, 23.5, -4.5));
        assert_eq!(hat.max, Vec3::new(4.5, 32.5, 4.5));
    }
}
