use bytemuck::{Pod, Zeroable};
use wgpu::{
    Buffer, BufferAddress, BufferUsages, Device, IndexFormat, RenderPass, VertexAttribute,
    VertexBufferLayout, VertexStepMode,
    util::{BufferInitDescriptor, DeviceExt},
};

const CUBE_VERTICES: &[Vertex] = &[
    // Front
    Vertex::new([-0.6, -0.6, 0.6], [0.9, 0.2, 0.2], [0.0, 0.0, 1.0]),
    Vertex::new([0.6, -0.6, 0.6], [0.2, 0.9, 0.3], [0.0, 0.0, 1.0]),
    Vertex::new([0.6, 0.6, 0.6], [0.2, 0.4, 1.0], [0.0, 0.0, 1.0]),
    Vertex::new([-0.6, 0.6, 0.6], [0.9, 0.8, 0.2], [0.0, 0.0, 1.0]),
    // Back
    Vertex::new([0.6, -0.6, -0.6], [0.2, 0.8, 0.9], [0.0, 0.0, -1.0]),
    Vertex::new([-0.6, -0.6, -0.6], [0.5, 0.2, 0.9], [0.0, 0.0, -1.0]),
    Vertex::new([-0.6, 0.6, -0.6], [0.4, 0.9, 0.2], [0.0, 0.0, -1.0]),
    Vertex::new([0.6, 0.6, -0.6], [0.9, 0.5, 0.2], [0.0, 0.0, -1.0]),
    // Left
    Vertex::new([-0.6, -0.6, -0.6], [0.5, 0.2, 0.9], [-1.0, 0.0, 0.0]),
    Vertex::new([-0.6, -0.6, 0.6], [0.9, 0.2, 0.2], [-1.0, 0.0, 0.0]),
    Vertex::new([-0.6, 0.6, 0.6], [0.9, 0.8, 0.2], [-1.0, 0.0, 0.0]),
    Vertex::new([-0.6, 0.6, -0.6], [0.4, 0.9, 0.2], [-1.0, 0.0, 0.0]),
    // Right
    Vertex::new([0.6, -0.6, 0.6], [0.2, 0.9, 0.3], [1.0, 0.0, 0.0]),
    Vertex::new([0.6, -0.6, -0.6], [0.2, 0.8, 0.9], [1.0, 0.0, 0.0]),
    Vertex::new([0.6, 0.6, -0.6], [0.9, 0.5, 0.2], [1.0, 0.0, 0.0]),
    Vertex::new([0.6, 0.6, 0.6], [0.2, 0.4, 1.0], [1.0, 0.0, 0.0]),
    // Top
    Vertex::new([-0.6, 0.6, 0.6], [0.9, 0.8, 0.2], [0.0, 1.0, 0.0]),
    Vertex::new([0.6, 0.6, 0.6], [0.2, 0.4, 1.0], [0.0, 1.0, 0.0]),
    Vertex::new([0.6, 0.6, -0.6], [0.9, 0.5, 0.2], [0.0, 1.0, 0.0]),
    Vertex::new([-0.6, 0.6, -0.6], [0.4, 0.9, 0.2], [0.0, 1.0, 0.0]),
    // Bottom
    Vertex::new([-0.6, -0.6, -0.6], [0.5, 0.2, 0.9], [0.0, -1.0, 0.0]),
    Vertex::new([0.6, -0.6, -0.6], [0.2, 0.8, 0.9], [0.0, -1.0, 0.0]),
    Vertex::new([0.6, -0.6, 0.6], [0.2, 0.9, 0.3], [0.0, -1.0, 0.0]),
    Vertex::new([-0.6, -0.6, 0.6], [0.9, 0.2, 0.2], [0.0, -1.0, 0.0]),
];
const CUBE_INDICES: &[u32] = &[
    0, 1, 2, 0, 2, 3, // Front
    4, 5, 6, 4, 6, 7, // Back
    8, 9, 10, 8, 10, 11, // Left
    12, 13, 14, 12, 14, 15, // Right
    16, 17, 18, 16, 18, 19, // Top
    20, 21, 22, 20, 22, 23, // Bottom
];

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

pub(super) struct GpuMesh {
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    index_count: u32,
}

impl GpuMesh {
    pub(super) fn cube(device: &Device) -> Self {
        Self::new(device, CUBE_VERTICES, CUBE_INDICES)
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
    use glam::Vec3;

    #[test]
    fn cube_mesh_matches_shader_contract() {
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

        assert_eq!(CUBE_VERTICES.len(), 24);
        assert_eq!(CUBE_INDICES.len(), 36);
        assert_eq!(CUBE_INDICES.len() % 3, 0);
        assert!(
            CUBE_INDICES
                .iter()
                .all(|&index| (index as usize) < CUBE_VERTICES.len())
        );
    }

    #[test]
    fn cube_normals_are_unit_length_and_match_triangle_winding() {
        assert!(CUBE_VERTICES.iter().all(|vertex| {
            let normal = Vec3::from_array(vertex.normal);
            normal.is_finite() && normal.is_normalized()
        }));

        let (triangles, remainder) = CUBE_INDICES.as_chunks::<3>();
        assert!(remainder.is_empty());

        for &[first, second, third] in triangles {
            let first = &CUBE_VERTICES[first as usize];
            let second = &CUBE_VERTICES[second as usize];
            let third = &CUBE_VERTICES[third as usize];
            let first_position = Vec3::from_array(first.position);
            let second_position = Vec3::from_array(second.position);
            let third_position = Vec3::from_array(third.position);
            let winding_normal = (second_position - first_position)
                .cross(third_position - first_position)
                .normalize();

            for vertex in [first, second, third] {
                let normal = Vec3::from_array(vertex.normal);
                assert!(
                    normal.abs_diff_eq(winding_normal, 1.0e-6),
                    "expected {winding_normal:?}, got {normal:?}"
                );
            }
        }
    }
}
