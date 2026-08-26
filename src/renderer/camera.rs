use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI, TAU};

use bytemuck::{Pod, Zeroable};
use glam::{
    Mat4, Vec2, Vec3,
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
const ORBIT_SENSITIVITY: f32 = 0.005;
const ZOOM_SENSITIVITY: f32 = 0.2;
const PITCH_LIMIT: f32 = FRAC_PI_2 - 0.01;
const MIN_DISTANCE: f32 = 1.25;
const MAX_DISTANCE: f32 = 25.0;

struct Camera {
    target: Vec3,
    yaw: f32,
    pitch: f32,
    distance: f32,
    aspect_ratio: f32,
    field_of_view_y: f32,
    near_plane: f32,
    far_plane: f32,
}

impl Camera {
    fn new(size: PhysicalSize<u32>) -> Self {
        let target = Vec3::ZERO;
        let initial_eye = Vec3::new(1.5, 1.0, 2.0);
        let offset = initial_eye - target;
        let distance = offset.length();

        Self {
            target,
            yaw: offset.x.atan2(offset.z),
            pitch: (offset.y / distance).asin(),
            distance,
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

    fn orbit(&mut self, cursor_delta: Vec2) -> bool {
        if !cursor_delta.is_finite() || cursor_delta == Vec2::ZERO {
            return false;
        }

        let previous_yaw = self.yaw;
        let previous_pitch = self.pitch;
        self.yaw = wrap_angle(self.yaw - cursor_delta.x * ORBIT_SENSITIVITY);
        self.pitch =
            (self.pitch + cursor_delta.y * ORBIT_SENSITIVITY).clamp(-PITCH_LIMIT, PITCH_LIMIT);

        self.yaw != previous_yaw || self.pitch != previous_pitch
    }

    fn zoom(&mut self, scroll_amount: f32) -> bool {
        if !scroll_amount.is_finite() || scroll_amount == 0.0 {
            return false;
        }

        let previous_distance = self.distance;
        self.distance = (self.distance * (-scroll_amount * ZOOM_SENSITIVITY).exp())
            .clamp(MIN_DISTANCE, MAX_DISTANCE);

        self.distance != previous_distance
    }

    fn eye(&self) -> Vec3 {
        let horizontal_distance = self.pitch.cos() * self.distance;

        self.target
            + Vec3::new(
                self.yaw.sin() * horizontal_distance,
                self.pitch.sin() * self.distance,
                self.yaw.cos() * horizontal_distance,
            )
    }

    fn view_projection(&self) -> Mat4 {
        let view = look_at_mat4(self.eye(), self.target, Vec3::Y);
        let projection = perspective(
            self.field_of_view_y,
            self.aspect_ratio,
            self.near_plane,
            self.far_plane,
        );

        projection * view
    }
}

fn wrap_angle(angle: f32) -> f32 {
    (angle + PI).rem_euclid(TAU) - PI
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
            self.write_uniform(queue);
        }
    }

    pub(super) fn orbit(&mut self, queue: &Queue, cursor_delta: Vec2) -> bool {
        if !self.camera.orbit(cursor_delta) {
            return false;
        }

        self.write_uniform(queue);
        true
    }

    pub(super) fn zoom(&mut self, queue: &Queue, scroll_amount: f32) -> bool {
        if !self.camera.zoom(scroll_amount) {
            return false;
        }

        self.write_uniform(queue);
        true
    }

    pub(super) fn bind_group_layout(&self) -> &BindGroupLayout {
        &self.bind_group_layout
    }

    pub(super) fn bind_group(&self) -> &BindGroup {
        &self.bind_group
    }

    fn write_uniform(&self, queue: &Queue) {
        let uniform = CameraUniform::new(&self.camera);
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniform));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_vec3_close(actual: Vec3, expected: Vec3) {
        assert!(
            actual.abs_diff_eq(expected, 1.0e-6),
            "expected {expected:?}, got {actual:?}"
        );
    }

    #[test]
    fn orbit_camera_preserves_initial_viewpoint() {
        let camera = Camera::new(PhysicalSize::new(1280, 720));

        assert_vec3_close(camera.eye(), Vec3::new(1.5, 1.0, 2.0));
        assert_eq!(camera.target, Vec3::ZERO);
    }

    #[test]
    fn orbit_camera_preserves_target_and_distance() {
        let mut camera = Camera::new(PhysicalSize::new(1280, 720));
        let initial_eye = camera.eye();
        let initial_distance = camera.distance;

        assert!(camera.orbit(Vec2::new(20.0, -10.0)));

        assert_eq!(camera.target, Vec3::ZERO);
        assert_eq!(camera.distance, initial_distance);
        assert_ne!(camera.eye(), initial_eye);
        assert!(camera.eye().is_finite());
    }

    #[test]
    fn orbit_camera_wraps_yaw_and_clamps_pitch() {
        let mut camera = Camera::new(PhysicalSize::new(1280, 720));

        assert!(camera.orbit(Vec2::new(-10_000.0, 10_000.0)));

        assert!((-PI..PI).contains(&camera.yaw));
        assert_eq!(camera.pitch, PITCH_LIMIT);
        assert!(camera.eye().is_finite());
    }

    #[test]
    fn orbit_camera_zoom_is_bounded() {
        let mut camera = Camera::new(PhysicalSize::new(1280, 720));
        let initial_distance = camera.distance;

        assert!(camera.zoom(1.0));
        assert!(camera.distance < initial_distance);
        assert!(camera.zoom(f32::MAX));
        assert_eq!(camera.distance, MIN_DISTANCE);
        assert!(camera.zoom(-f32::MAX));
        assert_eq!(camera.distance, MAX_DISTANCE);
    }

    #[test]
    fn orbit_camera_ignores_invalid_input() {
        let mut camera = Camera::new(PhysicalSize::new(1280, 720));
        let initial_eye = camera.eye();

        assert!(!camera.orbit(Vec2::ZERO));
        assert!(!camera.orbit(Vec2::splat(f32::NAN)));
        assert!(!camera.zoom(0.0));
        assert!(!camera.zoom(f32::NAN));
        assert_eq!(camera.eye(), initial_eye);
    }

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
