use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use wgpu::{
    Buffer, BufferAddress, BufferUsages, Device, IndexFormat, RenderPass, VertexAttribute,
    VertexBufferLayout, VertexStepMode,
    util::{BufferInitDescriptor, DeviceExt},
};

use super::terrain::Terrain;

const CUBE_SPHERE_RESOLUTION: u32 = 64;
const CUBE_SPHERE_RADIUS: f32 = 0.6;
const CUBE_SPHERE_COLOR: [f32; 3] = [0.2, 0.7, 0.35];
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
        let Some(patches_per_edge) = 1_u32.checked_shl(level) else {
            panic!("terrain patch level exceeds the supported range");
        };
        assert!(
            x < patches_per_edge && y < patches_per_edge,
            "terrain patch coordinates must be within their level"
        );

        Self { face, level, x, y }
    }

    const fn root(face: CubeFace) -> Self {
        Self::new(face, 0, 0, 0)
    }

    fn bounds(self) -> PatchBounds {
        let patches_per_edge = 1_u32 << self.level;
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

#[derive(Clone, Copy, Debug, PartialEq)]
struct PatchBounds {
    minimum_horizontal: f32,
    maximum_horizontal: f32,
    minimum_vertical: f32,
    maximum_vertical: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(super) struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
    normal: [f32; 3],
}

impl Vertex {
    const ATTRIBUTES: [VertexAttribute; 3] = wgpu::vertex_attr_array![
        0 => Float32x3,
        1 => Float32x3,
        2 => Float32x3,
    ];

    const fn new(position: [f32; 3], color: [f32; 3], normal: [f32; 3]) -> Self {
        Self {
            position,
            color,
            normal,
        }
    }

    pub(super) fn layout() -> VertexBufferLayout<'static> {
        VertexBufferLayout {
            array_stride: size_of::<Self>() as BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

struct MeshData {
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
}

impl MeshData {
    fn terrain_patch(resolution: u32, terrain: Terrain, patch: PatchId) -> Self {
        assert!(resolution > 0, "terrain patch resolution must be positive");

        let vertices_per_edge = resolution + 1;
        let vertex_count = vertices_per_edge * vertices_per_edge;
        let index_count = resolution * resolution * 6;
        let mut vertices = Vec::with_capacity(vertex_count as usize);
        let mut indices = Vec::with_capacity(index_count as usize);
        let face = patch.face.basis();
        let bounds = patch.bounds();

        for row in 0..=resolution {
            let vertical = bounds.minimum_vertical
                + (bounds.maximum_vertical - bounds.minimum_vertical) * row as f32
                    / resolution as f32;

            for column in 0..=resolution {
                let horizontal = bounds.minimum_horizontal
                    + (bounds.maximum_horizontal - bounds.minimum_horizontal) * column as f32
                        / resolution as f32;
                let direction =
                    (face.normal + face.horizontal * horizontal + face.vertical * vertical)
                        .normalize();
                vertices.push(Vertex::new(
                    terrain.position(direction).to_array(),
                    CUBE_SPHERE_COLOR,
                    terrain.normal(direction).to_array(),
                ));
            }
        }

        for row in 0..resolution {
            for column in 0..resolution {
                let first = row * vertices_per_edge + column;
                let second = first + 1;
                let fourth = first + vertices_per_edge;
                let third = fourth + 1;
                indices.extend_from_slice(&[first, second, third, first, third, fourth]);
            }
        }

        Self { vertices, indices }
    }
}

struct GpuMesh {
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    index_count: u32,
}

impl GpuMesh {
    fn new(device: &Device, vertices: &[Vertex], indices: &[u32]) -> Self {
        let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Orbis terrain patch vertex buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Orbis terrain patch index buffer"),
            contents: bytemuck::cast_slice(indices),
            usage: BufferUsages::INDEX,
        });
        let index_count = indices
            .len()
            .try_into()
            .expect("mesh index count exceeds the supported u32 range");

        Self {
            vertex_buffer,
            index_buffer,
            index_count,
        }
    }

    fn draw<'pass>(&'pass self, render_pass: &mut RenderPass<'pass>) {
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), IndexFormat::Uint32);
        render_pass.draw_indexed(0..self.index_count, 0, 0..1);
    }
}

pub(super) struct TerrainMesh {
    patches: Vec<GpuMesh>,
}

impl TerrainMesh {
    pub(super) fn new(device: &Device) -> Self {
        let terrain = Terrain::new(
            DEFAULT_TERRAIN_SEED,
            CUBE_SPHERE_RADIUS,
            TERRAIN_MAX_ELEVATION,
        );
        let patches = CUBE_FACES
            .into_iter()
            .map(|face| {
                let mesh =
                    MeshData::terrain_patch(CUBE_SPHERE_RESOLUTION, terrain, PatchId::root(face));
                GpuMesh::new(device, &mesh.vertices, &mesh.indices)
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
            .map(|face| MeshData::terrain_patch(resolution, terrain, PatchId::root(face)))
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
        let lower_left = PatchId::new(CubeFace::Front, 2, 2, 0);
        let lower_right = PatchId::new(CubeFace::Front, 2, 3, 0);
        let upper_left = PatchId::new(CubeFace::Front, 2, 2, 1);
        let upper_right = PatchId::new(CubeFace::Front, 2, 3, 1);
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
    #[should_panic(expected = "terrain patch coordinates must be within their level")]
    fn patch_coordinates_must_be_within_their_level() {
        PatchId::new(CubeFace::Front, 1, 2, 0);
    }

    #[test]
    fn vertex_layout_matches_shader_contract() {
        let layout = Vertex::layout();
        let [position, color, normal] = layout.attributes else {
            panic!("vertex layout must contain position, color, and normal attributes");
        };

        assert_eq!(layout.array_stride, 36);
        assert_eq!(layout.step_mode, VertexStepMode::Vertex);
        assert_eq!(position.format, wgpu::VertexFormat::Float32x3);
        assert_eq!(position.offset, 0);
        assert_eq!(position.shader_location, 0);
        assert_eq!(color.format, wgpu::VertexFormat::Float32x3);
        assert_eq!(color.offset, 12);
        assert_eq!(color.shader_location, 1);
        assert_eq!(normal.format, wgpu::VertexFormat::Float32x3);
        assert_eq!(normal.offset, 24);
        assert_eq!(normal.shader_location, 2);
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
        let mesh = MeshData::terrain_patch(
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
        let left =
            MeshData::terrain_patch(RESOLUTION, terrain, PatchId::new(CubeFace::Front, 2, 1, 2));
        let right =
            MeshData::terrain_patch(RESOLUTION, terrain, PatchId::new(CubeFace::Front, 2, 2, 2));
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
        let vertices_per_edge = (RESOLUTION + 1) as usize;

        for (face_index, mesh) in meshes.iter().enumerate() {
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
                        .filter(|(other_face_index, _)| *other_face_index != face_index)
                        .flat_map(|(_, mesh)| &mesh.vertices)
                        .find(|other| {
                            Vec3::from_array(other.position)
                                .normalize()
                                .abs_diff_eq(direction, 1.0e-6)
                        })
                        .expect("each face-boundary vertex must belong to another face");

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
