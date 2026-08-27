use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use wgpu::{
    Buffer, BufferAddress, BufferUsages, Device, IndexFormat, RenderPass, VertexAttribute,
    VertexBufferLayout, VertexStepMode,
    util::{BufferInitDescriptor, DeviceExt},
};

#[cfg(test)]
use super::terrain::MAX_RELATIVE_HEIGHT;
use super::terrain::Terrain;

const CUBE_SPHERE_RESOLUTION: u32 = 16;
const CUBE_SPHERE_RADIUS: f32 = 0.6;
const CUBE_SPHERE_COLOR: [f32; 3] = [0.2, 0.7, 0.35];
const TERRAIN_SEED: u32 = 42;
const CUBE_FACES: [CubeFace; 6] = [
    CubeFace::new(Vec3::Z, Vec3::X, Vec3::Y),
    CubeFace::new(Vec3::NEG_Z, Vec3::NEG_X, Vec3::Y),
    CubeFace::new(Vec3::NEG_X, Vec3::Z, Vec3::Y),
    CubeFace::new(Vec3::X, Vec3::NEG_Z, Vec3::Y),
    CubeFace::new(Vec3::Y, Vec3::X, Vec3::NEG_Z),
    CubeFace::new(Vec3::NEG_Y, Vec3::X, Vec3::Z),
];

#[derive(Clone, Copy)]
struct CubeFace {
    normal: Vec3,
    horizontal: Vec3,
    vertical: Vec3,
}

impl CubeFace {
    const fn new(normal: Vec3, horizontal: Vec3, vertical: Vec3) -> Self {
        Self {
            normal,
            horizontal,
            vertical,
        }
    }
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
    fn cube_sphere(resolution: u32, radius: f32) -> Self {
        assert!(resolution > 0, "cube-sphere resolution must be positive");
        assert!(
            radius.is_finite() && radius > 0.0,
            "cube-sphere radius must be finite and positive"
        );

        let vertices_per_edge = resolution + 1;
        let vertices_per_face = vertices_per_edge * vertices_per_edge;
        let indices_per_face = resolution * resolution * 6;
        let mut vertices = Vec::with_capacity((vertices_per_face * 6) as usize);
        let mut indices = Vec::with_capacity((indices_per_face * 6) as usize);
        let terrain = Terrain::new(TERRAIN_SEED);

        for face in CUBE_FACES {
            let face_start = u32::try_from(vertices.len())
                .expect("cube-sphere vertex count exceeds the supported u32 range");

            for row in 0..=resolution {
                let vertical = -1.0 + 2.0 * row as f32 / resolution as f32;

                for column in 0..=resolution {
                    let horizontal = -1.0 + 2.0 * column as f32 / resolution as f32;
                    let unit_direction =
                        (face.normal + face.horizontal * horizontal + face.vertical * vertical)
                            .normalize();
                    vertices.push(Vertex::new(
                        terrain.surface_position(unit_direction, radius).to_array(),
                        CUBE_SPHERE_COLOR,
                        terrain.surface_normal(unit_direction, radius).to_array(),
                    ));
                }
            }

            for row in 0..resolution {
                for column in 0..resolution {
                    let first = face_start + row * vertices_per_edge + column;
                    let second = first + 1;
                    let fourth = first + vertices_per_edge;
                    let third = fourth + 1;
                    indices.extend_from_slice(&[first, second, third, first, third, fourth]);
                }
            }
        }

        Self { vertices, indices }
    }
}

pub(super) struct GpuMesh {
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    index_count: u32,
}

impl GpuMesh {
    pub(super) fn cube_sphere(device: &Device) -> Self {
        let mesh = MeshData::cube_sphere(CUBE_SPHERE_RESOLUTION, CUBE_SPHERE_RADIUS);
        Self::new(device, &mesh.vertices, &mesh.indices)
    }

    fn new(device: &Device, vertices: &[Vertex], indices: &[u32]) -> Self {
        let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Orbis mesh vertex buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Orbis mesh index buffer"),
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

    pub(super) fn draw<'pass>(&'pass self, render_pass: &mut RenderPass<'pass>) {
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), IndexFormat::Uint32);
        render_pass.draw_indexed(0..self.index_count, 0, 0..1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn cube_sphere_has_expected_face_geometry() {
        const RESOLUTION: u32 = 4;
        let mesh = MeshData::cube_sphere(RESOLUTION, 2.5);
        let vertices_per_face = ((RESOLUTION + 1) * (RESOLUTION + 1)) as usize;
        let indices_per_face = (RESOLUTION * RESOLUTION * 6) as usize;

        assert_eq!(mesh.vertices.len(), vertices_per_face * CUBE_FACES.len());
        assert_eq!(mesh.indices.len(), indices_per_face * CUBE_FACES.len());
        assert!(
            mesh.indices
                .iter()
                .all(|&index| (index as usize) < mesh.vertices.len())
        );

        for face_index in 0..CUBE_FACES.len() {
            let vertex_start = face_index * vertices_per_face;
            let vertex_end = vertex_start + vertices_per_face;
            let index_start = face_index * indices_per_face;
            let index_end = index_start + indices_per_face;

            assert!(mesh.indices[index_start..index_end].iter().all(|&index| {
                let index = index as usize;
                (vertex_start..vertex_end).contains(&index)
            }));
        }
    }

    #[test]
    fn cube_sphere_vertices_have_bounded_terrain_and_outward_normals() {
        const RADIUS: f32 = 2.5;
        let mesh = MeshData::cube_sphere(4, RADIUS);
        let minimum_radius = RADIUS * (1.0 - MAX_RELATIVE_HEIGHT);
        let maximum_radius = RADIUS * (1.0 + MAX_RELATIVE_HEIGHT);

        assert!(mesh.vertices.iter().all(|vertex| {
            let position = Vec3::from_array(vertex.position);
            let normal = Vec3::from_array(vertex.normal);
            position.is_finite()
                && (minimum_radius..=maximum_radius).contains(&position.length())
                && normal.is_finite()
                && normal.is_normalized()
                && normal.dot(position) > 0.0
        }));
    }

    #[test]
    fn cube_sphere_face_boundaries_have_matching_positions_and_normals() {
        const RESOLUTION: u32 = 4;
        let mesh = MeshData::cube_sphere(RESOLUTION, 2.5);
        let mut matching_pairs = 0;

        for (index, first) in mesh.vertices.iter().enumerate() {
            for second in &mesh.vertices[index + 1..] {
                let first_position = Vec3::from_array(first.position);
                let second_position = Vec3::from_array(second.position);

                if first_position.abs_diff_eq(second_position, 1.0e-6) {
                    matching_pairs += 1;
                    assert!(
                        Vec3::from_array(first.normal)
                            .abs_diff_eq(Vec3::from_array(second.normal), 1.0e-6)
                    );
                }
            }
        }

        let edge_pairs = 12 * (RESOLUTION - 1);
        let corner_pairs = 8 * 3;
        assert_eq!(matching_pairs, edge_pairs + corner_pairs);
    }

    #[test]
    fn cube_sphere_triangles_have_outward_winding() {
        let mesh = MeshData::cube_sphere(4, 2.5);

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
