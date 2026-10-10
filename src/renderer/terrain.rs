use glam::Vec3;

const BASE_FREQUENCY: f32 = 1.5;
const OCTAVES: u32 = 4;
const LACUNARITY: f32 = 2.0;
const PERSISTENCE: f32 = 0.5;
const NORMAL_SAMPLE_DISTANCE: f32 = 1.0e-3;

#[derive(Clone, Copy, Debug)]
pub(super) struct Terrain {
    seed: u32,
    base_radius: f32,
    max_elevation: f32,
}

impl Terrain {
    pub(super) fn new(seed: u32, base_radius: f32, max_elevation: f32) -> Self {
        assert!(
            base_radius.is_finite() && base_radius > 0.0,
            "terrain base radius must be finite and positive"
        );
        assert!(
            max_elevation.is_finite() && max_elevation >= 0.0 && max_elevation < base_radius,
            "terrain maximum elevation must be finite, nonnegative, and less than its base radius"
        );

        Self {
            seed,
            base_radius,
            max_elevation,
        }
    }

    pub(super) fn minimum_radius(self) -> f32 {
        self.base_radius - self.max_elevation
    }

    pub(super) fn position(self, direction: Vec3) -> Vec3 {
        let direction = normalized_direction(direction);
        direction * (self.base_radius + self.height_for_normalized_direction(direction))
    }

    pub(super) fn color(self, position: Vec3, normal: Vec3) -> [f32; 3] {
        let elevation = if self.max_elevation > 0.0 {
            ((position.length() - self.base_radius) / self.max_elevation).clamp(-1.0, 1.0)
        } else {
            0.0
        };
        // Slope is relative to the planet's local up direction, including at the poles.
        let slope = 1.0 - normal.normalize().dot(position.normalize()).clamp(0.0, 1.0);
        let lowland = Vec3::new(0.07, 0.19, 0.06);
        let upland = Vec3::new(0.24, 0.29, 0.12);
        let rock = Vec3::new(0.30, 0.27, 0.23);
        let snow = Vec3::new(0.85, 0.90, 0.94);
        let vegetation = lowland.lerp(upland, smooth_blend(-0.15, 0.25, elevation));
        let rock_weight = smooth_blend(0.03, 0.18, slope).max(smooth_blend(0.20, 0.45, elevation));
        let snow_weight =
            smooth_blend(0.35, 0.60, elevation) * (1.0 - smooth_blend(0.05, 0.20, slope));

        vegetation
            .lerp(rock, rock_weight)
            .lerp(snow, snow_weight)
            .to_array()
    }

    pub(super) fn normal(self, direction: Vec3) -> Vec3 {
        let direction = normalized_direction(direction);
        if self.max_elevation == 0.0 {
            return direction;
        }

        let reference = if direction.y.abs() < 0.9 {
            Vec3::Y
        } else {
            Vec3::X
        };
        let tangent = reference.cross(direction).normalize();
        let bitangent = direction.cross(tangent);
        let tangent_delta = self.position(direction + tangent * NORMAL_SAMPLE_DISTANCE)
            - self.position(direction - tangent * NORMAL_SAMPLE_DISTANCE);
        let bitangent_delta = self.position(direction + bitangent * NORMAL_SAMPLE_DISTANCE)
            - self.position(direction - bitangent * NORMAL_SAMPLE_DISTANCE);

        tangent_delta.cross(bitangent_delta).normalize()
    }

    fn height_for_normalized_direction(self, direction: Vec3) -> f32 {
        fractal_noise(direction * BASE_FREQUENCY, self.seed) * self.max_elevation
    }
}

fn normalized_direction(direction: Vec3) -> Vec3 {
    assert!(
        direction.is_finite() && direction.length_squared() > 0.0,
        "terrain direction must be finite and nonzero"
    );
    direction.normalize()
}

fn smooth_blend(start: f32, end: f32, value: f32) -> f32 {
    let amount = ((value - start) / (end - start)).clamp(0.0, 1.0);
    amount * amount * (3.0 - 2.0 * amount)
}

fn fractal_noise(point: Vec3, seed: u32) -> f32 {
    let mut value = 0.0;
    let mut frequency = 1.0;
    let mut amplitude = 1.0;
    let mut amplitude_sum = 0.0;

    for _ in 0..OCTAVES {
        value += value_noise(point * frequency, seed) * amplitude;
        amplitude_sum += amplitude;
        frequency *= LACUNARITY;
        amplitude *= PERSISTENCE;
    }

    (value / amplitude_sum).clamp(-1.0, 1.0)
}

fn value_noise(point: Vec3, seed: u32) -> f32 {
    let minimum = point.floor();
    let fraction = point - minimum;
    let minimum = minimum.as_ivec3();
    let blend = fraction * fraction * (Vec3::splat(3.0) - 2.0 * fraction);

    let mut corners = [0.0; 8];
    for z in 0..=1 {
        for y in 0..=1 {
            for x in 0..=1 {
                let index = (z * 4 + y * 2 + x) as usize;
                corners[index] = lattice_value(minimum.x + x, minimum.y + y, minimum.z + z, seed);
            }
        }
    }

    let lower_front = lerp(corners[0], corners[1], blend.x);
    let lower_back = lerp(corners[4], corners[5], blend.x);
    let upper_front = lerp(corners[2], corners[3], blend.x);
    let upper_back = lerp(corners[6], corners[7], blend.x);
    let lower = lerp(lower_front, lower_back, blend.z);
    let upper = lerp(upper_front, upper_back, blend.z);

    lerp(lower, upper, blend.y)
}

fn lattice_value(x: i32, y: i32, z: i32, seed: u32) -> f32 {
    let mut hash = seed;
    hash ^= (x as u32).wrapping_mul(0x9e37_79b1);
    hash = hash.rotate_left(13);
    hash ^= (y as u32).wrapping_mul(0x85eb_ca77);
    hash = hash.rotate_left(11);
    hash ^= (z as u32).wrapping_mul(0xc2b2_ae3d);
    hash ^= hash >> 16;
    hash = hash.wrapping_mul(0x7feb_352d);
    hash ^= hash >> 15;
    hash = hash.wrapping_mul(0x846c_a68b);
    hash ^= hash >> 16;

    let unit = (hash >> 8) as f32 / 16_777_215.0;
    unit * 2.0 - 1.0
}

fn lerp(start: f32, end: f32, amount: f32) -> f32 {
    start + (end - start) * amount
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE_RADIUS: f32 = 0.6;
    const MAX_ELEVATION: f32 = 0.08;
    const DIRECTIONS: [Vec3; 6] = [
        Vec3::X,
        Vec3::Y,
        Vec3::Z,
        Vec3::new(1.0, 2.0, 3.0),
        Vec3::new(-2.0, 1.0, 0.5),
        Vec3::new(0.25, -1.0, 2.0),
    ];

    #[test]
    fn terrain_materials_follow_elevation_and_local_slope() {
        let terrain = Terrain::new(42, BASE_RADIUS, MAX_ELEVATION);
        for up in [Vec3::X, Vec3::Y, Vec3::Z] {
            let lowland = terrain.color(up * (BASE_RADIUS - MAX_ELEVATION), up);
            let summit = up * (BASE_RADIUS + MAX_ELEVATION);
            assert_eq!(lowland, [0.07, 0.19, 0.06]);
            assert_eq!(terrain.color(summit, up), [0.85, 0.90, 0.94]);
            assert_eq!(
                terrain.color(summit, up.any_orthonormal_vector()),
                [0.30, 0.27, 0.23]
            );
        }
    }

    #[test]
    fn flat_terrain_material_is_finite() {
        let terrain = Terrain::new(42, BASE_RADIUS, 0.0);
        let color = terrain.color(Vec3::Y * BASE_RADIUS, Vec3::Y);
        assert!(
            color
                .iter()
                .all(|value| value.is_finite() && (0.0..=1.0).contains(value))
        );
    }

    #[test]
    fn terrain_height_is_deterministic_for_a_seed() {
        let first = Terrain::new(42, BASE_RADIUS, MAX_ELEVATION);
        let second = Terrain::new(42, BASE_RADIUS, MAX_ELEVATION);

        for direction in DIRECTIONS {
            let direction = direction.normalize();
            assert_eq!(
                first.height_for_normalized_direction(direction),
                second.height_for_normalized_direction(direction)
            );
        }
    }

    #[test]
    fn terrain_seed_changes_height_samples() {
        let first = Terrain::new(42, BASE_RADIUS, MAX_ELEVATION);
        let second = Terrain::new(43, BASE_RADIUS, MAX_ELEVATION);

        assert!(
            DIRECTIONS
                .iter()
                .map(|direction| direction.normalize())
                .any(|direction| {
                    first.height_for_normalized_direction(direction)
                        != second.height_for_normalized_direction(direction)
                })
        );
    }

    #[test]
    fn terrain_samples_are_bounded_and_outward_facing() {
        let terrain = Terrain::new(42, BASE_RADIUS, MAX_ELEVATION);

        for direction in DIRECTIONS {
            let direction = direction.normalize();
            let height = terrain.height_for_normalized_direction(direction);
            let position = terrain.position(direction);
            let normal = terrain.normal(direction);

            assert!(height.is_finite());
            assert!((-MAX_ELEVATION..=MAX_ELEVATION).contains(&height));
            assert!(position.is_finite());
            assert!(
                ((BASE_RADIUS - MAX_ELEVATION)..=(BASE_RADIUS + MAX_ELEVATION))
                    .contains(&position.length())
            );
            assert!(normal.is_finite());
            assert!(normal.is_normalized());
            assert!(normal.dot(direction) > 0.0);
        }
    }
}
