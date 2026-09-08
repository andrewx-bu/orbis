use glam::Vec3;
use wgpu::{Device, RenderPass};

use super::{
    mesh::{GpuMesh, MeshData, Vertex},
    terrain::Terrain,
};

const PATCH_RESOLUTION: u32 = 32;
const PLANET_RADIUS: f32 = 0.6;
const TERRAIN_COLOR: [f32; 3] = [0.2, 0.7, 0.35];
const DEFAULT_TERRAIN_SEED: u32 = 0;
const TERRAIN_MAX_ELEVATION: f32 = 0.08;
const CUBE_FACES: [CubeFace; 6] = [
    CubeFace::Front,
    CubeFace::Back,
    CubeFace::Left,
    CubeFace::Right,
    CubeFace::Top,
    CubeFace::Bottom,
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CubeFace {
    Front,
    Back,
    Left,
    Right,
    Top,
    Bottom,
}

impl CubeFace {
    const fn basis(self) -> FaceBasis {
        match self {
            Self::Front => FaceBasis::new(Vec3::Z, Vec3::X, Vec3::Y),
            Self::Back => FaceBasis::new(Vec3::NEG_Z, Vec3::NEG_X, Vec3::Y),
            Self::Left => FaceBasis::new(Vec3::NEG_X, Vec3::Z, Vec3::Y),
            Self::Right => FaceBasis::new(Vec3::X, Vec3::NEG_Z, Vec3::Y),
            Self::Top => FaceBasis::new(Vec3::Y, Vec3::X, Vec3::NEG_Z),
            Self::Bottom => FaceBasis::new(Vec3::NEG_Y, Vec3::X, Vec3::Z),
        }
    }
}

#[derive(Clone, Copy)]
struct FaceBasis {
    normal: Vec3,
    horizontal: Vec3,
    vertical: Vec3,
}

impl FaceBasis {
    const fn new(normal: Vec3, horizontal: Vec3, vertical: Vec3) -> Self {
        Self {
            normal,
            horizontal,
            vertical,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PatchId {
    face: CubeFace,
    level: u32,
    x: u32,
    y: u32,
}

impl PatchId {
    const fn new(face: CubeFace, level: u32, x: u32, y: u32) -> Self {
        let patches_per_edge = patches_per_edge(level);
        assert!(
            x < patches_per_edge && y < patches_per_edge,
            "terrain patch coordinates must be within their level"
        );

        Self { face, level, x, y }
    }

    const fn root(face: CubeFace) -> Self {
        Self::new(face, 0, 0, 0)
    }

    fn children(self) -> [Self; 4] {
        let level = self.level + 1;
        let x = self.x * 2;
        let y = self.y * 2;

        [
            Self::new(self.face, level, x, y),
            Self::new(self.face, level, x + 1, y),
            Self::new(self.face, level, x, y + 1),
            Self::new(self.face, level, x + 1, y + 1),
        ]
    }

    fn bounds(self) -> PatchBounds {
        let patches_per_edge = patches_per_edge(self.level);
        let width = 2.0 / patches_per_edge as f32;
        let minimum_horizontal = -1.0 + self.x as f32 * width;
        let minimum_vertical = -1.0 + self.y as f32 * width;

        PatchBounds {
            minimum_horizontal,
            maximum_horizontal: minimum_horizontal + width,
            minimum_vertical,
            maximum_vertical: minimum_vertical + width,
        }
    }
}

const fn patches_per_edge(level: u32) -> u32 {
    let Some(patches_per_edge) = 1_u32.checked_shl(level) else {
        panic!("terrain patch level exceeds the supported range");
    };

    patches_per_edge
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct PatchBounds {
    minimum_horizontal: f32,
    maximum_horizontal: f32,
    minimum_vertical: f32,
    maximum_vertical: f32,
}

fn generate_patch(resolution: u32, terrain: Terrain, patch: PatchId) -> MeshData {
    let counts = patch_mesh_counts(resolution);
    let face = patch.face.basis();
    let bounds = patch.bounds();
    validate_patch_spacing(bounds, resolution);
    let mut vertices = Vec::with_capacity(counts.vertex_count as usize);
    let mut indices = Vec::with_capacity(counts.index_count as usize);

    for row in 0..=resolution {
        let vertical = axis_coordinate(
            bounds.minimum_vertical,
            bounds.maximum_vertical,
            row,
            resolution,
        );

        for column in 0..=resolution {
            let horizontal = axis_coordinate(
                bounds.minimum_horizontal,
                bounds.maximum_horizontal,
                column,
                resolution,
            );
            let direction =
                (face.normal + face.horizontal * horizontal + face.vertical * vertical).normalize();
            vertices.push(Vertex::new(
                terrain.position(direction).to_array(),
                TERRAIN_COLOR,
                terrain.normal(direction).to_array(),
            ));
        }
    }

    for row in 0..resolution {
        for column in 0..resolution {
            let first = row * counts.vertices_per_edge + column;
            let second = first + 1;
            let fourth = first + counts.vertices_per_edge;
            let third = fourth + 1;
            indices.extend_from_slice(&[first, second, third, first, third, fourth]);
        }
    }

    MeshData::new(vertices, indices)
}

struct PatchMeshCounts {
    vertices_per_edge: u32,
    vertex_count: u32,
    index_count: u32,
}

fn patch_mesh_counts(resolution: u32) -> PatchMeshCounts {
    assert!(resolution > 0, "terrain patch resolution must be positive");

    let vertices_per_edge = resolution
        .checked_add(1)
        .expect("terrain patch resolution exceeds the supported vertex count");
    let vertex_count = vertices_per_edge
        .checked_mul(vertices_per_edge)
        .expect("terrain patch resolution exceeds the supported vertex count");
    let index_count = resolution
        .checked_mul(resolution)
        .and_then(|quad_count| quad_count.checked_mul(6))
        .expect("terrain patch resolution exceeds the supported index count");

    PatchMeshCounts {
        vertices_per_edge,
        vertex_count,
        index_count,
    }
}

fn validate_patch_spacing(bounds: PatchBounds, resolution: u32) {
    for (minimum, maximum) in [
        (bounds.minimum_horizontal, bounds.maximum_horizontal),
        (bounds.minimum_vertical, bounds.maximum_vertical),
    ] {
        let mut previous = minimum;

        for index in 1..=resolution {
            let coordinate = axis_coordinate(minimum, maximum, index, resolution);
            assert!(
                coordinate > previous,
                "terrain patch level and resolution exceed f32 coordinate precision"
            );
            previous = coordinate;
        }
    }
}

fn axis_coordinate(minimum: f32, maximum: f32, index: u32, resolution: u32) -> f32 {
    minimum + (maximum - minimum) * index as f32 / resolution as f32
}

pub(super) struct TerrainMesh {
    patches: Vec<GpuMesh>,
}

fn initial_patches() -> impl Iterator<Item = PatchId> {
    CUBE_FACES
        .into_iter()
        .flat_map(|face| PatchId::root(face).children())
}

impl TerrainMesh {
    pub(super) fn new(device: &Device) -> Self {
        let terrain = Terrain::new(DEFAULT_TERRAIN_SEED, PLANET_RADIUS, TERRAIN_MAX_ELEVATION);
        let patches = initial_patches()
            .map(|patch| {
                let mesh = generate_patch(PATCH_RESOLUTION, terrain, patch);
                GpuMesh::new(device, &mesh)
            })
            .collect();

        Self { patches }
    }

    pub(super) fn draw<'pass>(&'pass self, render_pass: &mut RenderPass<'pass>) {
        for patch in &self.patches {
            patch.draw(render_pass);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root_patch_meshes(resolution: u32, terrain: Terrain) -> Vec<MeshData> {
        CUBE_FACES
            .into_iter()
            .map(|face| generate_patch(resolution, terrain, PatchId::root(face)))
            .collect()
    }

    #[test]
    fn root_patches_cover_each_cube_face() {
        let patches = CUBE_FACES.map(PatchId::root);

        assert!(
            patches
                .iter()
                .all(|patch| patch.level == 0 && patch.x == 0 && patch.y == 0)
        );
        assert!(patches.iter().all(|patch| {
            patch.bounds()
                == PatchBounds {
                    minimum_horizontal: -1.0,
                    maximum_horizontal: 1.0,
                    minimum_vertical: -1.0,
                    maximum_vertical: 1.0,
                }
        }));
    }

    #[test]
    fn child_patches_tile_the_parent() {
        let parent = PatchId::new(CubeFace::Front, 1, 1, 0);
        let [lower_left, lower_right, upper_left, upper_right] = parent.children();
        assert_eq!(
            parent.children(),
            [
                PatchId::new(CubeFace::Front, 2, 2, 0),
                PatchId::new(CubeFace::Front, 2, 3, 0),
                PatchId::new(CubeFace::Front, 2, 2, 1),
                PatchId::new(CubeFace::Front, 2, 3, 1),
            ]
        );
        assert_eq!(
            lower_left.bounds().minimum_horizontal,
            parent.bounds().minimum_horizontal
        );
        assert_eq!(
            lower_left.bounds().minimum_vertical,
            parent.bounds().minimum_vertical
        );
        assert_eq!(
            upper_right.bounds().maximum_horizontal,
            parent.bounds().maximum_horizontal
        );
        assert_eq!(
            upper_right.bounds().maximum_vertical,
            parent.bounds().maximum_vertical
        );
        assert_eq!(
            lower_left.bounds().maximum_horizontal,
            lower_right.bounds().minimum_horizontal
        );
        assert_eq!(
            lower_left.bounds().maximum_vertical,
            upper_left.bounds().minimum_vertical
        );
    }

    #[test]
    fn initial_patches_cover_every_face_at_the_original_triangle_count() {
        let patches: Vec<_> = initial_patches().collect();
        assert_eq!(patches.len(), 24);

        for face in CUBE_FACES {
            for y in 0..2 {
                for x in 0..2 {
                    let expected = PatchId::new(face, 1, x, y);
                    assert_eq!(
                        patches.iter().filter(|&&patch| patch == expected).count(),
                        1
                    );
                }
            }
        }

        let triangle_count =
            patches.len() * patch_mesh_counts(PATCH_RESOLUTION).index_count as usize / 3;
        assert_eq!(triangle_count, 6 * 64 * 64 * 2);
    }

    #[test]
    #[should_panic(expected = "terrain patch coordinates must be within their level")]
    fn patch_coordinates_must_be_within_their_level() {
        PatchId::new(CubeFace::Front, 1, 2, 0);
    }

    #[test]
    #[should_panic(expected = "terrain patch level exceeds the supported range")]
    fn patch_bounds_reject_an_unsupported_level() {
        PatchId {
            face: CubeFace::Front,
            level: u32::BITS,
            x: 0,
            y: 0,
        }
        .bounds();
    }

    #[test]
    #[should_panic(expected = "terrain patch resolution exceeds the supported vertex count")]
    fn terrain_patch_rejects_vertex_count_overflow() {
        patch_mesh_counts(u32::MAX);
    }

    #[test]
    #[should_panic(expected = "terrain patch resolution exceeds the supported index count")]
    fn terrain_patch_rejects_index_count_overflow() {
        patch_mesh_counts(30_000);
    }

    #[test]
    #[should_panic(expected = "terrain patch level and resolution exceed f32 coordinate precision")]
    fn terrain_patch_rejects_collapsed_vertex_spacing() {
        const RESOLUTION: u32 = 64;
        let level = 20;
        let edge = patches_per_edge(level) - 1;

        generate_patch(
            RESOLUTION,
            Terrain::new(0, 2.5, 0.0),
            PatchId::new(CubeFace::Front, level, edge, edge),
        );
    }

    #[test]
    fn root_patches_have_expected_geometry() {
        const RESOLUTION: u32 = 4;
        let meshes = root_patch_meshes(RESOLUTION, Terrain::new(0, 2.5, 0.0));
        let vertices_per_face = ((RESOLUTION + 1) * (RESOLUTION + 1)) as usize;
        let indices_per_face = (RESOLUTION * RESOLUTION * 6) as usize;

        assert_eq!(meshes.len(), CUBE_FACES.len());
        for mesh in meshes {
            assert_eq!(mesh.vertices.len(), vertices_per_face);
            assert_eq!(mesh.indices.len(), indices_per_face);
            assert!(
                mesh.indices
                    .iter()
                    .all(|&index| (index as usize) < mesh.vertices.len())
            );
        }
    }

    #[test]
    fn terrain_patch_has_expected_geometry() {
        const RESOLUTION: u32 = 4;
        let mesh = generate_patch(
            RESOLUTION,
            Terrain::new(0, 2.5, 0.0),
            PatchId::new(CubeFace::Top, 3, 4, 6),
        );

        assert_eq!(
            mesh.vertices.len(),
            ((RESOLUTION + 1) * (RESOLUTION + 1)) as usize
        );
        assert_eq!(mesh.indices.len(), (RESOLUTION * RESOLUTION * 6) as usize);
        assert!(
            mesh.indices
                .iter()
                .all(|&index| (index as usize) < mesh.vertices.len())
        );
    }

    #[test]
    fn sibling_patch_boundaries_share_positions_and_normals() {
        const RESOLUTION: u32 = 8;
        let terrain = Terrain::new(42, 2.5, 0.2);
        let left = generate_patch(RESOLUTION, terrain, PatchId::new(CubeFace::Front, 2, 1, 2));
        let right = generate_patch(RESOLUTION, terrain, PatchId::new(CubeFace::Front, 2, 2, 2));
        let vertices_per_edge = (RESOLUTION + 1) as usize;

        for row in 0..vertices_per_edge {
            let left_vertex = &left.vertices[row * vertices_per_edge + RESOLUTION as usize];
            let right_vertex = &right.vertices[row * vertices_per_edge];

            assert!(
                Vec3::from_array(left_vertex.position)
                    .abs_diff_eq(Vec3::from_array(right_vertex.position), 1.0e-6)
            );
            assert!(
                Vec3::from_array(left_vertex.normal)
                    .abs_diff_eq(Vec3::from_array(right_vertex.normal), 1.0e-6)
            );
        }
    }

    #[test]
    fn cube_sphere_vertices_follow_terrain_radius_and_normals() {
        const RADIUS: f32 = 2.5;
        const MAX_ELEVATION: f32 = 0.2;
        let meshes = root_patch_meshes(8, Terrain::new(0, RADIUS, MAX_ELEVATION));

        assert!(meshes.iter().flat_map(|mesh| &mesh.vertices).all(|vertex| {
            let position = Vec3::from_array(vertex.position);
            let normal = Vec3::from_array(vertex.normal);
            position.is_finite()
                && ((RADIUS - MAX_ELEVATION)..=(RADIUS + MAX_ELEVATION))
                    .contains(&position.length())
                && normal.is_finite()
                && normal.is_normalized()
                && normal.dot(position.normalize()) > 0.0
        }));
    }

    #[test]
    fn cube_sphere_triangles_have_outward_winding() {
        let meshes = root_patch_meshes(8, Terrain::new(0, 2.5, 0.2));

        for mesh in meshes {
            let (triangles, remainder) = mesh.indices.as_chunks::<3>();
            assert!(remainder.is_empty());

            for &[first, second, third] in triangles {
                let first = &mesh.vertices[first as usize];
                let second = &mesh.vertices[second as usize];
                let third = &mesh.vertices[third as usize];
                let first_position = Vec3::from_array(first.position);
                let second_position = Vec3::from_array(second.position);
                let third_position = Vec3::from_array(third.position);
                let winding_normal = (second_position - first_position)
                    .cross(third_position - first_position)
                    .normalize();

                for vertex in [first, second, third] {
                    let normal = Vec3::from_array(vertex.normal);
                    assert!(
                        winding_normal.dot(normal) > 0.0,
                        "expected {winding_normal:?} to face outward with {normal:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn cube_sphere_face_boundaries_share_positions_and_normals() {
        const RESOLUTION: u32 = 8;
        let meshes = root_patch_meshes(RESOLUTION, Terrain::new(0, 2.5, 0.2));
        assert_patch_boundaries_match(&meshes, RESOLUTION);
    }

    #[test]
    fn initial_patch_boundaries_share_positions_and_normals() {
        const RESOLUTION: u32 = 8;
        let terrain = Terrain::new(42, PLANET_RADIUS, TERRAIN_MAX_ELEVATION);
        let meshes: Vec<_> = initial_patches()
            .map(|patch| generate_patch(RESOLUTION, terrain, patch))
            .collect();
        assert_patch_boundaries_match(&meshes, RESOLUTION);
    }

    fn assert_patch_boundaries_match(meshes: &[MeshData], resolution: u32) {
        let vertices_per_edge = (resolution + 1) as usize;

        for (patch_index, mesh) in meshes.iter().enumerate() {
            for row in 0..vertices_per_edge {
                for column in 0..vertices_per_edge {
                    if row != 0
                        && row != vertices_per_edge - 1
                        && column != 0
                        && column != vertices_per_edge - 1
                    {
                        continue;
                    }

                    let index = row * vertices_per_edge + column;
                    let vertex = &mesh.vertices[index];
                    let direction = Vec3::from_array(vertex.position).normalize();
                    let matching_vertex = meshes
                        .iter()
                        .enumerate()
                        .filter(|(other_patch_index, _)| *other_patch_index != patch_index)
                        .flat_map(|(_, mesh)| &mesh.vertices)
                        .find(|other| {
                            Vec3::from_array(other.position)
                                .normalize()
                                .abs_diff_eq(direction, 1.0e-6)
                        })
                        .expect("each patch-boundary vertex must belong to another patch");

                    assert!(
                        Vec3::from_array(matching_vertex.position)
                            .abs_diff_eq(Vec3::from_array(vertex.position), 1.0e-5)
                    );
                    assert!(
                        Vec3::from_array(matching_vertex.normal)
                            .abs_diff_eq(Vec3::from_array(vertex.normal), 1.0e-5)
                    );
                }
            }
        }
    }
}
