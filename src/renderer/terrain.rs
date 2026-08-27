use glam::Vec3;

const OCTAVE_COUNT: usize = 5;
const BASE_FREQUENCY: f32 = 2.0;
const LACUNARITY: f32 = 2.0;
const PERSISTENCE: f32 = 0.5;
const NORMAL_SAMPLE_DISTANCE: f32 = 1.0e-3;
pub(super) const MAX_RELATIVE_HEIGHT: f32 = 0.08;

#[derive(Clone, Copy, Debug)]
pub(super) struct Terrain {
    seed: u32,
}

impl Terrain {
    pub(super) const fn new(seed: u32) -> Self {
        Self { seed }
    }

    pub(super) fn height(self, unit_direction: Vec3, radius: f32) -> f32 {
        debug_assert!(unit_direction.is_normalized());
        debug_assert!(radius.is_finite() && radius > 0.0);

        self.normalized_height(unit_direction) * radius * MAX_RELATIVE_HEIGHT
    }

    pub(super) fn surface_position(self, unit_direction: Vec3, radius: f32) -> Vec3 {
        unit_direction * (radius + self.height(unit_direction, radius))
    }

    pub(super) fn surface_normal(self, unit_direction: Vec3, radius: f32) -> Vec3 {
        let tangent = unit_direction
            .cross(least_aligned_axis(unit_direction))
            .normalize();
        let bitangent = unit_direction.cross(tangent).normalize();
        let tangent_delta = self.directional_surface_delta(unit_direction, tangent, radius);
        let bitangent_delta = self.directional_surface_delta(unit_direction, bitangent, radius);
        let normal = tangent_delta.cross(bitangent_delta).normalize();

        if normal.dot(unit_direction) < 0.0 {
            -normal
        } else {
            normal
        }
    }

    fn directional_surface_delta(self, unit_direction: Vec3, tangent: Vec3, radius: f32) -> Vec3 {
        let before = (unit_direction - tangent * NORMAL_SAMPLE_DISTANCE).normalize();
        let after = (unit_direction + tangent * NORMAL_SAMPLE_DISTANCE).normalize();

        self.surface_position(after, radius) - self.surface_position(before, radius)
    }

    fn normalized_height(self, unit_direction: Vec3) -> f32 {
        let mut frequency = BASE_FREQUENCY;
        let mut amplitude = 1.0;
        let mut amplitude_sum = 0.0;
        let mut height = 0.0;

        for _ in 0..OCTAVE_COUNT {
            height += value_noise(unit_direction * frequency, self.seed) * amplitude;
            amplitude_sum += amplitude;
            frequency *= LACUNARITY;
            amplitude *= PERSISTENCE;
        }

        height / amplitude_sum
    }
}

fn least_aligned_axis(direction: Vec3) -> Vec3 {
    let absolute = direction.abs();

    if absolute.x <= absolute.y && absolute.x <= absolute.z {
        Vec3::X
    } else if absolute.y <= absolute.z {
        Vec3::Y
    } else {
        Vec3::Z
    }
}

fn value_noise(point: Vec3, seed: u32) -> f32 {
    let floor = point.floor();
    let base = [floor.x as i32, floor.y as i32, floor.z as i32];
    let fraction = point - floor;
    let blend = fraction * fraction * (Vec3::splat(3.0) - 2.0 * fraction);

    let mut corners = [0.0; 8];

    for z in 0..=1 {
        for y in 0..=1 {
            for x in 0..=1 {
                let index = x + y * 2 + z * 4;
                corners[index] = lattice_value(
                    [base[0] + x as i32, base[1] + y as i32, base[2] + z as i32],
                    seed,
                );
            }
        }
    }

    let lower_front = interpolate(corners[0], corners[1], blend.x);
    let upper_front = interpolate(corners[2], corners[3], blend.x);
    let lower_back = interpolate(corners[4], corners[5], blend.x);
    let upper_back = interpolate(corners[6], corners[7], blend.x);
    let front = interpolate(lower_front, upper_front, blend.y);
    let back = interpolate(lower_back, upper_back, blend.y);

    interpolate(front, back, blend.z)
}

fn lattice_value(point: [i32; 3], seed: u32) -> f32 {
    let mut hash = seed
        ^ (point[0] as u32).wrapping_mul(0x9e37_79b1)
        ^ (point[1] as u32).wrapping_mul(0x85eb_ca77)
        ^ (point[2] as u32).wrapping_mul(0xc2b2_ae3d);

    hash ^= hash >> 16;
    hash = hash.wrapping_mul(0x7feb_352d);
    hash ^= hash >> 15;
    hash = hash.wrapping_mul(0x846c_a68b);
    hash ^= hash >> 16;

    hash as f32 / u32::MAX as f32 * 2.0 - 1.0
}

fn interpolate(start: f32, end: f32, amount: f32) -> f32 {
    start + (end - start) * amount
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_DIRECTIONS: [Vec3; 6] = [
        Vec3::X,
        Vec3::Y,
        Vec3::Z,
        Vec3::new(1.0, 1.0, 1.0),
        Vec3::new(-1.0, 2.0, 0.5),
        Vec3::new(0.25, -0.75, 1.0),
    ];

    #[test]
    fn terrain_is_deterministic_for_a_seed() {
        let first = Terrain::new(42);
        let second = Terrain::new(42);

        for direction in SAMPLE_DIRECTIONS.map(Vec3::normalize) {
            assert_eq!(first.height(direction, 2.5), second.height(direction, 2.5));
        }
    }

    #[test]
    fn terrain_changes_with_the_seed() {
        let first = Terrain::new(42);
        let second = Terrain::new(43);

        assert!(
            SAMPLE_DIRECTIONS
                .map(Vec3::normalize)
                .iter()
                .any(|&direction| {
                    first.height(direction, 2.5) != second.height(direction, 2.5)
                })
        );
    }

    #[test]
    fn terrain_heights_are_finite_and_bounded() {
        const RADIUS: f32 = 2.5;
        let terrain = Terrain::new(42);
        let maximum_height = RADIUS * MAX_RELATIVE_HEIGHT;

        for direction in SAMPLE_DIRECTIONS.map(Vec3::normalize) {
            let height = terrain.height(direction, RADIUS);

            assert!(height.is_finite());
            assert!((-maximum_height..=maximum_height).contains(&height));
        }
    }

    #[test]
    fn value_noise_is_continuous_across_lattice_boundaries() {
        const OFFSET: f32 = 1.0e-4;
        let before = value_noise(Vec3::new(1.0 - OFFSET, -0.25, 0.75), 42);
        let boundary = value_noise(Vec3::new(1.0, -0.25, 0.75), 42);
        let after = value_noise(Vec3::new(1.0 + OFFSET, -0.25, 0.75), 42);

        assert!((before - boundary).abs() <= 1.0e-5);
        assert!((after - boundary).abs() <= 1.0e-5);
    }
}
