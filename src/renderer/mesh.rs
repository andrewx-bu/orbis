use bytemuck::{Pod, Zeroable};
use wgpu::{
    Buffer, BufferAddress, BufferUsages, Device, IndexFormat, RenderPass, VertexAttribute,
    VertexBufferLayout, VertexStepMode,
    util::{BufferInitDescriptor, DeviceExt},
};

const QUAD_VERTICES: &[Vertex] = &[
    Vertex::new([-0.6, 0.6, 0.0], [0.9, 0.2, 0.2]),
    Vertex::new([-0.6, -0.6, 0.0], [0.2, 0.9, 0.3]),
    Vertex::new([0.6, -0.6, 0.0], [0.2, 0.4, 1.0]),
    Vertex::new([0.6, 0.6, 0.0], [0.9, 0.8, 0.2]),
];
const QUAD_INDICES: &[u16] = &[0, 1, 2, 0, 2, 3];

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(super) struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}

impl Vertex {
    const ATTRIBUTES: [VertexAttribute; 2] = wgpu::vertex_attr_array![
        0 => Float32x3,
        1 => Float32x3,
    ];

    const fn new(position: [f32; 3], color: [f32; 3]) -> Self {
        Self { position, color }
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
    pub(super) fn quad(device: &Device) -> Self {
        Self::new(device, QUAD_VERTICES, QUAD_INDICES)
    }

    fn new(device: &Device, vertices: &[Vertex], indices: &[u16]) -> Self {
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
        render_pass.set_index_buffer(self.index_buffer.slice(..), IndexFormat::Uint16);
        render_pass.draw_indexed(0..self.index_count, 0, 0..1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quad_mesh_matches_shader_contract() {
        let layout = Vertex::layout();
        let [position, color] = layout.attributes else {
            panic!("vertex layout must contain position and color attributes");
        };

        assert_eq!(layout.array_stride, 24);
        assert_eq!(layout.step_mode, VertexStepMode::Vertex);
        assert_eq!(position.format, wgpu::VertexFormat::Float32x3);
        assert_eq!(position.offset, 0);
        assert_eq!(position.shader_location, 0);
        assert_eq!(color.format, wgpu::VertexFormat::Float32x3);
        assert_eq!(color.offset, 12);
        assert_eq!(color.shader_location, 1);

        assert_eq!(QUAD_VERTICES.len(), 4);
        assert_eq!(QUAD_INDICES.len(), 6);
        assert_eq!(QUAD_INDICES.len() % 3, 0);
        assert!(
            QUAD_INDICES
                .iter()
                .all(|&index| usize::from(index) < QUAD_VERTICES.len())
        );
    }
}
