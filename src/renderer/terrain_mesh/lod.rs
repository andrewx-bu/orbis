use std::collections::{HashMap, HashSet};

use glam::Vec3;

use super::{CUBE_FACES, MAX_SUBDIVISION_LEVEL, PATCH_RESOLUTION, PatchId, Terrain};

const SPLIT_ERROR_PIXELS: f32 = 2.0;
const MERGE_ERROR_PIXELS: f32 = 1.0;

pub(super) struct LodView {
    pub eye: Vec3,
    pub forward: Vec3,
    pub vertical_field_of_view: f32,
    pub viewport_height: u32,
    pub near_plane: f32,
}

struct PatchMetrics {
    center: Vec3,
    radius: f32,
    geometric_error: f32,
}

impl PatchMetrics {
    fn measure(patch: PatchId, terrain: Terrain, base_radius: f32, max_elevation: f32) -> Self {
        let bounds = patch.bounds();
        let face = patch.face.basis();
        let width = bounds.maximum_horizontal - bounds.minimum_horizontal;
        let direction = |u: f32, v: f32| {
            (face.normal
                + face.horizontal * (bounds.minimum_horizontal + width * u)
                + face.vertical * (bounds.minimum_vertical + width * v))
                .normalize()
        };

        // Sample at twice the mesh resolution, then compare with the actual
        // first-to-third triangle diagonal used by generate_patch.
        // This estimates error, rather than proving a bound on unsampled terrain.
        let resolution = PATCH_RESOLUTION as usize * 2;
        let edge = resolution + 1;
        let mut samples = Vec::with_capacity(edge * edge);
        for row in 0..=resolution {
            for column in 0..=resolution {
                samples.push(terrain.position(direction(
                    column as f32 / resolution as f32,
                    row as f32 / resolution as f32,
                )));
            }
        }
        let mut geometric_error = 0.0_f32;
        for row in (0..resolution).step_by(2) {
            for column in (0..resolution).step_by(2) {
                let first = samples[row * edge + column];
                let second = samples[row * edge + column + 2];
                let third = samples[(row + 2) * edge + column + 2];
                let fourth = samples[(row + 2) * edge + column];
                for y in 0..=2 {
                    for x in 0..=2 {
                        let u = x as f32 * 0.5;
                        let v = y as f32 * 0.5;
                        let interpolated = if x >= y {
                            first * (1.0 - u) + second * (u - v) + third * v
                        } else {
                            first * (1.0 - v) + third * u + fourth * (v - u)
                        };
                        let actual = samples[(row + y) * edge + column + x];
                        geometric_error = geometric_error.max(actual.distance(interpolated));
                    }
                }
            }
        }

        Self {
            center: direction(0.5, 0.5) * base_radius,
            // Normalizing a cube-face vector cannot stretch distances on that
            // face: its fixed normal component is one. Include all elevations.
            radius: base_radius * width * std::f32::consts::FRAC_1_SQRT_2 + max_elevation,
            geometric_error,
        }
    }

    fn projected_error(&self, view: &LodView) -> f32 {
        let nearest_depth =
            ((self.center - view.eye).dot(view.forward) - self.radius).max(view.near_plane);
        let pixels_per_unit =
            view.viewport_height as f32 / (2.0 * (view.vertical_field_of_view * 0.5).tan());
        self.geometric_error * pixels_per_unit / nearest_depth
    }
}

pub(super) struct LodSelector {
    metrics: HashMap<PatchId, PatchMetrics>,
    split: HashSet<PatchId>,
}

impl LodSelector {
    pub(super) fn new(terrain: Terrain, base_radius: f32, max_elevation: f32) -> Self {
        let mut metrics = HashMap::new();
        let mut pending: Vec<_> = CUBE_FACES.into_iter().map(PatchId::root).collect();
        while let Some(patch) = pending.pop() {
            if patch.level < MAX_SUBDIVISION_LEVEL {
                metrics.insert(
                    patch,
                    PatchMetrics::measure(patch, terrain, base_radius, max_elevation),
                );
                pending.extend(patch.children());
            }
        }
        Self {
            metrics,
            split: HashSet::new(),
        }
    }

    pub(super) fn select(&mut self, view: &LodView) -> Vec<PatchId> {
        assert!(view.eye.is_finite());
        assert!(view.forward.is_finite() && view.forward.is_normalized());
        assert!(
            view.vertical_field_of_view.is_finite()
                && view.vertical_field_of_view > 0.0
                && view.vertical_field_of_view < std::f32::consts::PI
        );
        assert!(view.viewport_height > 0);
        assert!(view.near_plane.is_finite() && view.near_plane > 0.0);

        let mut active = Vec::new();
        let mut split = HashSet::new();
        for face in CUBE_FACES {
            self.visit(PatchId::root(face), view, &mut active, &mut split);
        }
        self.split = split;
        active
    }

    fn visit(
        &self,
        patch: PatchId,
        view: &LodView,
        active: &mut Vec<PatchId>,
        split: &mut HashSet<PatchId>,
    ) {
        if patch.level < MAX_SUBDIVISION_LEVEL {
            let threshold = if self.split.contains(&patch) {
                MERGE_ERROR_PIXELS
            } else {
                SPLIT_ERROR_PIXELS
            };
            if self.metrics[&patch].projected_error(view) > threshold {
                split.insert(patch);
                for child in patch.children() {
                    self.visit(child, view, active, split);
                }
                return;
            }
        }
        active.push(patch);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::terrain_mesh::{PLANET_RADIUS, TERRAIN_MAX_ELEVATION, uniform_patches};

    fn view(distance: f32) -> LodView {
        LodView {
            eye: Vec3::Z * distance,
            forward: Vec3::NEG_Z,
            vertical_field_of_view: std::f32::consts::FRAC_PI_2,
            viewport_height: 1000,
            near_plane: 0.1,
        }
    }

    fn selector() -> LodSelector {
        LodSelector::new(
            Terrain::new(42, PLANET_RADIUS, TERRAIN_MAX_ELEVATION),
            PLANET_RADIUS,
            TERRAIN_MAX_ELEVATION,
        )
    }

    fn assert_complete_cover(active: &[PatchId]) {
        for finest in uniform_patches(MAX_SUBDIVISION_LEVEL) {
            let covering = active.iter().filter(|patch| {
                let shift = finest.level - patch.level;
                patch.face == finest.face
                    && patch.x == finest.x >> shift
                    && patch.y == finest.y >> shift
            });
            assert_eq!(covering.count(), 1);
        }
    }

    #[test]
    fn camera_changes_keep_complete_coverage_and_bounded_detail() {
        let mut selector = selector();
        for distance in [25.0, 4.0, 2.0, 1.25, 0.7, 0.0, 2.0, 25.0] {
            let active = selector.select(&view(distance));
            assert!((6..=96).contains(&active.len()));
            assert!(
                active
                    .iter()
                    .all(|patch| patch.level <= MAX_SUBDIVISION_LEVEL)
            );
            assert_complete_cover(&active);
            assert_eq!(selector.select(&view(distance)), active);
        }
        assert_eq!(selector.select(&view(1000.0)), uniform_patches(0));
        let mut close = view(0.0);
        close.viewport_height = 100_000;
        assert_eq!(
            selector.select(&close),
            uniform_patches(MAX_SUBDIVISION_LEVEL)
        );
    }

    #[test]
    fn split_and_merge_thresholds_preserve_history_in_between() {
        let mut selector = selector();
        // A synthetic one-unit error at the origin gives 500 / distance pixels.
        for metrics in selector.metrics.values_mut() {
            metrics.center = Vec3::ZERO;
            metrics.radius = 0.0;
            metrics.geometric_error = 1.0;
        }
        assert_eq!(selector.select(&view(500.0)), uniform_patches(0));
        assert_eq!(selector.select(&view(300.0)), uniform_patches(0));
        assert_eq!(selector.select(&view(250.0)), uniform_patches(0));
        assert_eq!(selector.select(&view(200.0)), uniform_patches(2));
        assert_eq!(selector.select(&view(300.0)), uniform_patches(2));
        assert_eq!(selector.select(&view(500.0)), uniform_patches(0));
        assert!(selector.split.is_empty());
        assert_eq!(selector.select(&view(300.0)), uniform_patches(0));
    }

    #[test]
    fn independent_patch_errors_produce_mixed_levels() {
        let mut selector = selector();
        for (patch, metrics) in &mut selector.metrics {
            metrics.geometric_error = if patch.face == CUBE_FACES[0] {
                1.0
            } else {
                0.0
            };
        }
        let active = selector.select(&view(2.0));
        assert_eq!(active.iter().filter(|patch| patch.level == 0).count(), 5);
        assert_eq!(active.iter().filter(|patch| patch.level == 2).count(), 16);
        assert_complete_cover(&active);
    }

    #[test]
    fn projection_responds_to_viewport_fov_and_depth() {
        let metrics = PatchMetrics {
            center: Vec3::ZERO,
            radius: 0.5,
            geometric_error: 0.01,
        };
        let mut camera = view(2.0);
        let original = metrics.projected_error(&camera);
        camera.viewport_height *= 2;
        assert_eq!(metrics.projected_error(&camera), original * 2.0);
        camera.vertical_field_of_view *= 0.5;
        assert!(metrics.projected_error(&camera) > original * 2.0);
        assert!(metrics.projected_error(&view(4.0)) < original);
        assert!(metrics.projected_error(&view(0.0)).is_finite());
    }

    #[test]
    fn measured_error_includes_sphere_curvature_and_bounds_include_terrain() {
        let flat = Terrain::new(0, PLANET_RADIUS, 0.0);
        let terrain = Terrain::new(42, PLANET_RADIUS, TERRAIN_MAX_ELEVATION);
        for face in CUBE_FACES {
            let root = PatchId::root(face);
            let parent = PatchMetrics::measure(root, flat, PLANET_RADIUS, 0.0);
            assert!(parent.geometric_error > 0.0);
            for child in root.children() {
                let child = PatchMetrics::measure(child, flat, PLANET_RADIUS, 0.0);
                assert!(child.geometric_error > 0.0);
                assert!(child.geometric_error < parent.geometric_error);
            }
            let metrics =
                PatchMetrics::measure(root, terrain, PLANET_RADIUS, TERRAIN_MAX_ELEVATION);
            assert!(metrics.geometric_error.is_finite() && metrics.geometric_error > 0.0);
            let mesh = super::super::generate_patch(PATCH_RESOLUTION, terrain, root);
            for vertex in mesh.vertices {
                assert!(
                    Vec3::from_array(vertex.position).distance(metrics.center) <= metrics.radius
                );
            }
        }
    }
}
