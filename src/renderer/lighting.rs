use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, BufferBindingType, BufferSize, BufferUsages, Device,
    ShaderStages,
    util::{BufferInitDescriptor, DeviceExt},
};

const DIRECTION_TO_LIGHT: Vec3 = Vec3::new(-0.5, 1.0, 0.75);
const AMBIENT_STRENGTH: f32 = 0.2;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct DirectionalLightUniform {
    direction_to_light: [f32; 3],
    ambient_strength: f32,
}

impl DirectionalLightUniform {
    fn new() -> Self {
        Self {
            direction_to_light: DIRECTION_TO_LIGHT.normalize().to_array(),
            ambient_strength: AMBIENT_STRENGTH,
        }
    }
}

pub(super) struct LightingResources {
    bind_group_layout: BindGroupLayout,
    bind_group: BindGroup,
}

impl LightingResources {
    pub(super) fn new(device: &Device) -> Self {
        let uniform = DirectionalLightUniform::new();
        let uniform_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Orbis directional light uniform buffer"),
            contents: bytemuck::bytes_of(&uniform),
            usage: BufferUsages::UNIFORM,
        });
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Orbis lighting bind group layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: BufferSize::new(size_of::<DirectionalLightUniform>() as u64),
                },
                count: None,
            }],
        });
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Orbis lighting bind group"),
            layout: &bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        Self {
            bind_group_layout,
            bind_group,
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
    fn directional_light_uniform_matches_wgsl_contract() {
        let uniform = DirectionalLightUniform::new();
        let direction = Vec3::from_array(uniform.direction_to_light);

        assert_eq!(size_of::<DirectionalLightUniform>(), 16);
        assert!(direction.is_finite());
        assert!(direction.is_normalized());
        assert!((0.0..=1.0).contains(&uniform.ambient_strength));
    }
}
