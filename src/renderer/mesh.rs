use bytemuck::{Pod, Zeroable};
use wgpu::{
    Buffer, BufferAddress, BufferUsages, Device, IndexFormat, RenderPass, VertexAttribute,
    VertexBufferLayout, VertexStepMode,
    util::{BufferInitDescriptor, DeviceExt},
};

const QUAD_VERTICES: &[Vertex] = &[
    Vertex::new([-0.6, 0.6], [0.9, 0.2, 0.2]),
    Vertex::new([-0.6, -0.6], [0.2, 0.9, 0.3]),
    Vertex::new([0.6, -0.6], [0.2, 0.4, 1.0]),
    Vertex::new([0.6, 0.6], [0.9, 0.8, 0.2]),
];
const QUAD_INDICES: &[u16] = &[0, 1, 2, 0, 2, 3];

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(super) struct Vertex {
    position: [f32; 2],
    color: [f32; 3],
}

impl Vertex {
    const ATTRIBUTES: [VertexAttribute; 2] = wgpu::vertex_attr_array![
        0 => Float32x2,
        1 => Float32x3,
    ];

    const fn new(position: [f32; 2], color: [f32; 3]) -> Self {
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
