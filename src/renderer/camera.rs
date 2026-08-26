use std::f32::consts::FRAC_PI_4;

use bytemuck::{Pod, Zeroable};
use glam::{
    Mat4, Vec3,
    camera::rh::{proj::directx::perspective, view::look_at_mat4},
};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, Buffer, BufferBindingType, BufferSize, BufferUsages, Device,
    Queue, ShaderStages,
    util::{BufferInitDescriptor, DeviceExt},
};
use winit::dpi::PhysicalSize;

const DEFAULT_ASPECT_RATIO: f32 = 1.0;
const NEAR_PLANE: f32 = 0.1;
const FAR_PLANE: f32 = 100.0;

struct Camera {
    eye: Vec3,
    target: Vec3,
    up: Vec3,
    aspect_ratio: f32,
    field_of_view_y: f32,
    near_plane: f32,
    far_plane: f32,
}

impl Camera {
    fn new(size: PhysicalSize<u32>) -> Self {
        Self {
            eye: Vec3::new(1.5, 1.0, 2.0),
            target: Vec3::ZERO,
            up: Vec3::Y,
            aspect_ratio: aspect_ratio(size).unwrap_or(DEFAULT_ASPECT_RATIO),
            field_of_view_y: FRAC_PI_4,
            near_plane: NEAR_PLANE,
            far_plane: FAR_PLANE,
        }
    }

    fn resize(&mut self, size: PhysicalSize<u32>) -> bool {
        let Some(aspect_ratio) = aspect_ratio(size) else {
            return false;
        };

        self.aspect_ratio = aspect_ratio;
        true
    }

    fn view_projection(&self) -> Mat4 {
        let view = look_at_mat4(self.eye, self.target, self.up);
        let projection = perspective(
            self.field_of_view_y,
            self.aspect_ratio,
            self.near_plane,
            self.far_plane,
        );

        projection * view
    }
}

fn aspect_ratio(size: PhysicalSize<u32>) -> Option<f32> {
    (size.width > 0 && size.height > 0).then(|| size.width as f32 / size.height as f32)
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct CameraUniform {
    view_projection: [[f32; 4]; 4],
}

impl CameraUniform {
    fn new(camera: &Camera) -> Self {
        Self {
            view_projection: camera.view_projection().to_cols_array_2d(),
        }
    }
}

pub(super) struct CameraResources {
    camera: Camera,
    uniform_buffer: Buffer,
    bind_group_layout: BindGroupLayout,
    bind_group: BindGroup,
}

impl CameraResources {
    pub(super) fn new(device: &Device, size: PhysicalSize<u32>) -> Self {
        let camera = Camera::new(size);
        let uniform = CameraUniform::new(&camera);
        let uniform_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Orbis camera uniform buffer"),
            contents: bytemuck::bytes_of(&uniform),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Orbis camera bind group layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: BufferSize::new(size_of::<CameraUniform>() as u64),
                },
                count: None,
            }],
        });
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Orbis camera bind group"),
            layout: &bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        Self {
            camera,
            uniform_buffer,
            bind_group_layout,
            bind_group,
        }
    }

    pub(super) fn resize(&mut self, queue: &Queue, size: PhysicalSize<u32>) {
        if self.camera.resize(size) {
            let uniform = CameraUniform::new(&self.camera);
            queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniform));
        }
    }

    pub(super) fn bind_group_layout(&self) -> &BindGroupLayout {
        &self.bind_group_layout
    }

    pub(super) fn bind_group(&self) -> &BindGroup {
        &self.bind_group
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_projection_updates_for_valid_sizes() {
        let mut camera = Camera::new(PhysicalSize::new(1280, 720));

        assert_eq!(camera.aspect_ratio, 1280.0 / 720.0);
        assert!(camera.resize(PhysicalSize::new(800, 800)));
        assert_eq!(camera.aspect_ratio, 1.0);
        assert!(!camera.resize(PhysicalSize::new(0, 800)));
        assert_eq!(camera.aspect_ratio, 1.0);
    }

    #[test]
    fn camera_uniform_matches_wgsl_contract() {
        let camera = Camera::new(PhysicalSize::new(1280, 720));
        let uniform = CameraUniform::new(&camera);

        assert_eq!(size_of::<CameraUniform>(), 64);
        assert!(
            uniform
                .view_projection
                .iter()
                .flatten()
                .all(|value| value.is_finite())
        );
    }
}
