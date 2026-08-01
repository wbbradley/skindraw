use std::f32::consts::FRAC_PI_2;

use glam::{Vec2, Vec3};

use crate::{
    atlas::face_region,
    model::{BodyPart, Face, Layer, ModelBox, ModelHit, ModelKind, Ray, model_boxes},
};

const PITCH_MARGIN: f32 = 0.001;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LayerVisibility {
    pub base: bool,
    pub outer: bool,
}

impl LayerVisibility {
    pub const ALL: Self = Self {
        base: true,
        outer: true,
    };
    pub const BASE_ONLY: Self = Self {
        base: true,
        outer: false,
    };
    pub const OUTER_ONLY: Self = Self {
        base: false,
        outer: true,
    };

    fn includes(self, layer: Layer) -> bool {
        match layer {
            Layer::Base => self.base,
            Layer::Outer => self.outer,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    pub target: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    pub orthographic_height: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            target: Vec3::new(0.0, 16.0, 0.0),
            yaw: 0.0,
            pitch: 0.0,
            distance: 64.0,
            orthographic_height: 40.0,
        }
    }
}

impl Camera {
    pub fn orbit(&mut self, yaw_delta: f32, pitch_delta: f32) {
        self.yaw += yaw_delta;
        self.pitch =
            (self.pitch + pitch_delta).clamp(-FRAC_PI_2 + PITCH_MARGIN, FRAC_PI_2 - PITCH_MARGIN);
    }

    pub fn position(self) -> Vec3 {
        self.target + self.offset_direction() * self.distance
    }

    pub fn view_direction(self) -> Vec3 {
        -self.offset_direction()
    }

    pub fn ray_for_pointer(
        self,
        pointer: Vec2,
        viewport_origin: Vec2,
        viewport_size: Vec2,
    ) -> Option<Ray> {
        if viewport_size.x <= 0.0
            || viewport_size.y <= 0.0
            || pointer.x < viewport_origin.x
            || pointer.y < viewport_origin.y
            || pointer.x >= viewport_origin.x + viewport_size.x
            || pointer.y >= viewport_origin.y + viewport_size.y
        {
            return None;
        }
        let normalized = (pointer - viewport_origin) / viewport_size;
        let ndc = Vec2::new(normalized.x * 2.0 - 1.0, 1.0 - normalized.y * 2.0);
        let direction = self.view_direction();
        let right = direction.cross(Vec3::Y).normalize();
        let up = right.cross(direction).normalize();
        let half_height = self.orthographic_height * 0.5;
        let half_width = half_height * viewport_size.x / viewport_size.y;
        let origin = self.position() + right * ndc.x * half_width + up * ndc.y * half_height;
        Some(Ray::new(origin, direction))
    }

    fn offset_direction(self) -> Vec3 {
        let pitch_cos = self.pitch.cos();
        Vec3::new(
            self.yaw.sin() * pitch_cos,
            self.pitch.sin(),
            self.yaw.cos() * pitch_cos,
        )
    }
}

pub fn pick_model(ray: Ray, kind: ModelKind, visibility: LayerVisibility) -> Option<ModelHit> {
    pick_model_part(ray, kind, visibility, None)
}

pub fn pick_model_part(
    ray: Ray,
    kind: ModelKind,
    visibility: LayerVisibility,
    part: Option<BodyPart>,
) -> Option<ModelHit> {
    model_boxes(kind)
        .into_iter()
        .filter(|model_box| visibility.includes(model_box.layer))
        .filter(|model_box| part.is_none_or(|part| model_box.part == part))
        .filter_map(|model_box| intersect_box(ray, model_box).map(|hit| (model_box, hit)))
        .filter_map(|(model_box, (distance, face, point))| {
            let region = face_region(kind, model_box.part, model_box.layer, face);
            let (u_fraction, v_fraction) = face_fractions(model_box, face, point);
            let local_u = fraction_to_index(u_fraction, region.rect.width);
            let local_v = fraction_to_index(v_fraction, region.rect.height);
            region.texel(local_u, local_v).map(|texel| ModelHit {
                part: model_box.part,
                layer: model_box.layer,
                face,
                distance,
                texel,
            })
        })
        .min_by(|left, right| left.distance.total_cmp(&right.distance))
}

fn intersect_box(ray: Ray, model_box: ModelBox) -> Option<(f32, Face, Vec3)> {
    let mut near = f32::NEG_INFINITY;
    let mut far = f32::INFINITY;
    let mut near_face = Face::Front;
    let mut far_face = Face::Back;

    for axis in 0..3 {
        let origin = ray.origin[axis];
        let direction = ray.direction[axis];
        let min = model_box.min[axis];
        let max = model_box.max[axis];
        if direction.abs() < f32::EPSILON {
            if origin < min || origin > max {
                return None;
            }
            continue;
        }
        let first = (min - origin) / direction;
        let second = (max - origin) / direction;
        let (axis_near, axis_far, axis_near_face, axis_far_face) = if first <= second {
            (first, second, min_face(axis), max_face(axis))
        } else {
            (second, first, max_face(axis), min_face(axis))
        };
        if axis_near > near {
            near = axis_near;
            near_face = axis_near_face;
        }
        if axis_far < far {
            far = axis_far;
            far_face = axis_far_face;
        }
        if near > far {
            return None;
        }
    }
    if far < 0.0 {
        return None;
    }
    let (distance, face) = if near >= 0.0 {
        (near, near_face)
    } else {
        (far, far_face)
    };
    Some((distance, face, ray.origin + ray.direction * distance))
}

fn min_face(axis: usize) -> Face {
    match axis {
        0 => Face::Right,
        1 => Face::Bottom,
        _ => Face::Back,
    }
}

fn max_face(axis: usize) -> Face {
    match axis {
        0 => Face::Left,
        1 => Face::Top,
        _ => Face::Front,
    }
}

fn face_fractions(model_box: ModelBox, face: Face, point: Vec3) -> (f32, f32) {
    let size = model_box.size();
    match face {
        Face::Front | Face::Back => (
            (point.x - model_box.min.x) / size.x,
            (model_box.max.y - point.y) / size.y,
        ),
        Face::Left | Face::Right => (
            (point.z - model_box.min.z) / size.z,
            (model_box.max.y - point.y) / size.y,
        ),
        Face::Top | Face::Bottom => (
            (point.x - model_box.min.x) / size.x,
            (point.z - model_box.min.z) / size.z,
        ),
    }
}

fn fraction_to_index(fraction: f32, length: u8) -> u8 {
    (fraction.clamp(0.0, 1.0 - f32::EPSILON) * f32::from(length)).floor() as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{BodyPart, Texel};

    fn ray(origin: Vec3, direction: Vec3) -> Ray {
        Ray::new(origin, direction)
    }

    #[test]
    fn camera_clamps_pitch_but_not_yaw() {
        let mut camera = Camera::default();
        camera.orbit(20.0, 20.0);
        assert_eq!(camera.yaw, 20.0);
        assert!(camera.pitch < FRAC_PI_2);
        camera.orbit(-50.0, -40.0);
        assert_eq!(camera.yaw, -30.0);
        assert!(camera.pitch > -FRAC_PI_2);
    }

    #[test]
    fn pointer_rays_are_orthographic_and_viewport_scoped() {
        let camera = Camera::default();
        let center = camera
            .ray_for_pointer(Vec2::new(100.0, 50.0), Vec2::ZERO, Vec2::new(200.0, 100.0))
            .unwrap();
        let corner = camera
            .ray_for_pointer(Vec2::new(0.0, 0.0), Vec2::ZERO, Vec2::new(200.0, 100.0))
            .unwrap();
        assert_eq!(center.direction, corner.direction);
        assert_eq!(center.origin.x, 0.0);
        assert_eq!(center.origin.y, 16.0);
        assert!(
            camera
                .ray_for_pointer(Vec2::new(200.0, 50.0), Vec2::ZERO, Vec2::new(200.0, 100.0))
                .is_none()
        );
    }

    #[test]
    fn rays_hit_all_six_head_faces_at_known_texels() {
        let cases = [
            (
                Vec3::new(0.5, 28.5, 40.0),
                -Vec3::Z,
                BodyPart::Head,
                Face::Front,
                Texel::new(12, 11),
            ),
            (
                Vec3::new(0.5, 28.5, -40.0),
                Vec3::Z,
                BodyPart::Head,
                Face::Back,
                Texel::new(27, 11),
            ),
            (
                Vec3::new(-40.0, 28.5, 0.5),
                Vec3::X,
                BodyPart::Head,
                Face::Right,
                Texel::new(4, 11),
            ),
            (
                Vec3::new(40.0, 28.5, 0.5),
                -Vec3::X,
                BodyPart::Head,
                Face::Left,
                Texel::new(19, 11),
            ),
            (
                Vec3::new(0.5, 40.0, 0.5),
                -Vec3::Y,
                BodyPart::Head,
                Face::Top,
                Texel::new(12, 4),
            ),
            (
                Vec3::new(0.5, -20.0, 0.5),
                Vec3::Y,
                BodyPart::LeftLeg,
                Face::Bottom,
                Texel::new(24, 50),
            ),
        ];
        for (origin, direction, part, face, texel) in cases {
            let hit = pick_model(
                ray(origin, direction),
                ModelKind::Classic,
                LayerVisibility::BASE_ONLY,
            )
            .unwrap();
            assert_eq!(hit.part, part);
            assert_eq!(hit.face, face);
            assert_eq!(hit.texel, texel);
        }
    }

    #[test]
    fn nearest_outer_layer_wins_and_layers_can_be_filtered() {
        let front_ray = ray(Vec3::new(0.5, 28.5, 40.0), -Vec3::Z);
        let outer = pick_model(front_ray, ModelKind::Classic, LayerVisibility::ALL).unwrap();
        assert_eq!(outer.layer, Layer::Outer);
        let base = pick_model(front_ray, ModelKind::Classic, LayerVisibility::BASE_ONLY).unwrap();
        assert_eq!(base.layer, Layer::Base);
        let outer_only =
            pick_model(front_ray, ModelKind::Classic, LayerVisibility::OUTER_ONLY).unwrap();
        assert_eq!(outer_only.layer, Layer::Outer);
    }

    #[test]
    fn misses_and_slim_arm_bounds_are_respected() {
        assert!(
            pick_model(
                ray(Vec3::new(30.0, 30.0, 40.0), -Vec3::Z),
                ModelKind::Classic,
                LayerVisibility::ALL
            )
            .is_none()
        );
        let narrow_edge = ray(Vec3::new(-7.75, 18.0, 40.0), -Vec3::Z);
        let classic =
            pick_model(narrow_edge, ModelKind::Classic, LayerVisibility::BASE_ONLY).unwrap();
        assert_eq!(classic.part, BodyPart::RightArm);
        assert!(pick_model(narrow_edge, ModelKind::Slim, LayerVisibility::BASE_ONLY).is_none());
    }

    #[test]
    fn body_part_filter_excludes_other_cuboids_from_picking() {
        let front_ray = ray(Vec3::new(0.5, 28.5, 40.0), -Vec3::Z);
        assert_eq!(
            pick_model_part(
                front_ray,
                ModelKind::Classic,
                LayerVisibility::ALL,
                Some(BodyPart::Head),
            )
            .unwrap()
            .part,
            BodyPart::Head
        );
        assert!(
            pick_model_part(
                front_ray,
                ModelKind::Classic,
                LayerVisibility::ALL,
                Some(BodyPart::LeftLeg),
            )
            .is_none()
        );

        let through_body = ray(Vec3::new(40.0, 18.0, 0.0), -Vec3::X);
        let hidden_arm_face = pick_model_part(
            through_body,
            ModelKind::Classic,
            LayerVisibility::BASE_ONLY,
            Some(BodyPart::RightArm),
        )
        .unwrap();
        assert_eq!(hidden_arm_face.part, BodyPart::RightArm);
        assert_eq!(hidden_arm_face.face, Face::Left);
    }
}
