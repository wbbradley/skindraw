use eframe::egui::ecolor::Hsva;
use rand::Rng;
use rand_distr::{Distribution, StandardNormal};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct HsvJitter {
    pub hue_degrees: f32,
    pub saturation_percent: f32,
    pub value_percent: f32,
}

impl HsvJitter {
    pub fn is_zero(self) -> bool {
        standard_deviation(self.hue_degrees) == 0.0
            && standard_deviation(self.saturation_percent) == 0.0
            && standard_deviation(self.value_percent) == 0.0
    }

    pub fn sample<R: Rng + ?Sized>(self, color: [u8; 4], rng: &mut R) -> [u8; 4] {
        if self.is_zero() {
            return color;
        }
        let mut hsv = Hsva::from_srgba_unmultiplied(color);
        let hue_offset: f32 = StandardNormal.sample(rng);
        let saturation_offset: f32 = StandardNormal.sample(rng);
        let value_offset: f32 = StandardNormal.sample(rng);
        hsv.h = (hsv.h + hue_offset * standard_deviation(self.hue_degrees) / 360.0).rem_euclid(1.0);
        hsv.s = (hsv.s + saturation_offset * standard_deviation(self.saturation_percent) / 100.0)
            .clamp(0.0, 1.0);
        hsv.v =
            (hsv.v + value_offset * standard_deviation(self.value_percent) / 100.0).clamp(0.0, 1.0);
        let mut sampled = hsv.to_srgba_unmultiplied();
        sampled[3] = color[3];
        sampled
    }
}

fn standard_deviation(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{SeedableRng, rngs::StdRng};

    #[test]
    fn zero_jitter_is_byte_exact_and_preserves_transparency() {
        let mut rng = StdRng::seed_from_u64(7);
        for color in [[0, 0, 0, 0], [17, 34, 51, 68], [255, 255, 255, 255]] {
            assert_eq!(HsvJitter::default().sample(color, &mut rng), color);
        }
    }

    #[test]
    fn seeded_gaussian_jitter_varies_rgb_but_never_alpha() {
        let jitter = HsvJitter {
            hue_degrees: 18.0,
            saturation_percent: 12.0,
            value_percent: 10.0,
        };
        let color = [120, 80, 160, 73];
        let mut rng = StdRng::seed_from_u64(42);
        let samples: Vec<_> = (0..64).map(|_| jitter.sample(color, &mut rng)).collect();
        assert!(samples.iter().all(|sample| sample[3] == color[3]));
        assert!(samples.iter().any(|sample| sample[..3] != color[..3]));
        assert!(samples.windows(2).any(|pair| pair[0] != pair[1]));
    }

    #[test]
    fn invalid_or_negative_deviations_are_treated_as_zero() {
        let jitter = HsvJitter {
            hue_degrees: f32::NAN,
            saturation_percent: -5.0,
            value_percent: f32::INFINITY,
        };
        let mut rng = StdRng::seed_from_u64(1);
        assert_eq!(jitter.sample([1, 2, 3, 4], &mut rng), [1, 2, 3, 4]);
    }
}
