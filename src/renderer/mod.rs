mod mesh;
mod surface;

use std::{error::Error, fmt, sync::Arc};

use self::{
    mesh::{GpuMesh, Vertex},
    surface::{FrameAcquisition, SurfaceState},
};
use wgpu::{
    Color, ColorTargetState, CommandEncoderDescriptor, Device, FragmentState, LoadOp, Operations,
    PipelineLayoutDescriptor, PrimitiveState, RenderPassColorAttachment, RenderPassDescriptor,
    RenderPipeline, RenderPipelineDescriptor, ShaderModuleDescriptor, ShaderSource, StoreOp,
    TextureFormat, TextureViewDescriptor, VertexState,
};
use winit::{dpi::PhysicalSize, window::Window};

const CLEAR_COLOR: Color = Color {
    r: 0.05,
    g: 0.15,
    b: 0.4,
    a: 1.0,
};

pub struct Renderer {
    surface: SurfaceState,
    render_pipeline: RenderPipeline,
    mesh: GpuMesh,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderOutcome {
    Presented,
    Retry,
    Skipped,
}

impl Renderer {
    pub fn new(window: Arc<Window>) -> Result<Self, RendererError> {
        let surface = pollster::block_on(SurfaceState::new(window))?;
        let render_pipeline = create_render_pipeline(surface.device(), surface.format());
        let mesh = GpuMesh::quad(surface.device());

        Ok(Self {
            surface,
            render_pipeline,
            mesh,
        })
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        self.surface.resize(size);
    }

    pub fn render(&mut self) -> Result<RenderOutcome, RendererError> {
        let frame = match self.surface.acquire_frame()? {
            FrameAcquisition::Ready(frame) => frame,
            FrameAcquisition::Retry => return Ok(RenderOutcome::Retry),
            FrameAcquisition::Wait => return Ok(RenderOutcome::Skipped),
        };
        let view = frame
            .texture()
            .create_view(&TextureViewDescriptor::default());
        let mut encoder = self
            .surface
            .device()
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("Orbis render encoder"),
            });

        {
            let color_attachment = RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(CLEAR_COLOR),
                    store: StoreOp::Store,
                },
            };
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Orbis render pass"),
                color_attachments: &[Some(color_attachment)],
                ..Default::default()
            });
            render_pass.set_pipeline(&self.render_pipeline);
            self.mesh.draw(&mut render_pass);
        }

        self.surface.present(encoder.finish(), frame);

        Ok(RenderOutcome::Presented)
    }
}

fn create_render_pipeline(device: &Device, surface_format: TextureFormat) -> RenderPipeline {
    let shader = device.create_shader_module(ShaderModuleDescriptor {
        label: Some("Orbis geometry shader"),
        source: ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
    });
    let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("Orbis render pipeline layout"),
        bind_group_layouts: &[],
        immediate_size: 0,
    });

    device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("Orbis render pipeline"),
        layout: Some(&layout),
        vertex: VertexState {
            module: &shader,
            entry_point: Some("vertex_main"),
            buffers: &[Some(Vertex::layout())],
            compilation_options: Default::default(),
        },
        primitive: PrimitiveState::default(),
        depth_stencil: None,
        multisample: Default::default(),
        fragment: Some(FragmentState {
            module: &shader,
            entry_point: Some("fragment_main"),
            targets: &[Some(ColorTargetState {
                format: surface_format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        multiview_mask: None,
        cache: None,
    })
}

#[derive(Debug)]
pub enum RendererError {
    CreateSurface(Box<dyn Error>),
    RequestAdapter(Box<dyn Error>),
    RequestDevice(Box<dyn Error>),
    UnsupportedSurface,
    SurfaceValidation,
}

impl RendererError {
    fn create_surface(source: impl Error + 'static) -> Self {
        Self::CreateSurface(Box::new(source))
    }

    fn request_adapter(source: impl Error + 'static) -> Self {
        Self::RequestAdapter(Box::new(source))
    }

    fn request_device(source: impl Error + 'static) -> Self {
        Self::RequestDevice(Box::new(source))
    }

    fn unsupported_surface() -> Self {
        Self::UnsupportedSurface
    }

    fn surface_validation() -> Self {
        Self::SurfaceValidation
    }
}

impl fmt::Display for RendererError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CreateSurface(source) => {
                write!(formatter, "failed to create surface: {source}")
            }
            Self::RequestAdapter(source) => {
                write!(formatter, "failed to request adapter: {source}")
            }
            Self::RequestDevice(source) => {
                write!(formatter, "failed to request device: {source}")
            }
            Self::UnsupportedSurface => formatter.write_str("surface is not supported by adapter"),
            Self::SurfaceValidation => formatter.write_str("surface validation failed"),
        }
    }
}

impl Error for RendererError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::CreateSurface(source)
            | Self::RequestAdapter(source)
            | Self::RequestDevice(source) => Some(source.as_ref()),
            Self::UnsupportedSurface | Self::SurfaceValidation => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct TestError(&'static str);

    impl fmt::Display for TestError {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str(self.0)
        }
    }

    impl Error for TestError {}

    #[test]
    fn sourced_errors_include_context_and_preserve_their_source() {
        let errors = [
            (
                RendererError::create_surface(TestError("test source")),
                "failed to create surface: test source",
            ),
            (
                RendererError::request_adapter(TestError("test source")),
                "failed to request adapter: test source",
            ),
            (
                RendererError::request_device(TestError("test source")),
                "failed to request device: test source",
            ),
        ];

        for (error, expected_message) in errors {
            assert_eq!(error.to_string(), expected_message);
            assert_eq!(
                error.source().map(ToString::to_string).as_deref(),
                Some("test source")
            );
        }
    }

    #[test]
    fn unsupported_surface_has_no_source() {
        let error = RendererError::unsupported_surface();

        assert_eq!(error.to_string(), "surface is not supported by adapter");
        assert!(error.source().is_none());
    }

    #[test]
    fn surface_validation_has_no_source() {
        let error = RendererError::surface_validation();

        assert_eq!(error.to_string(), "surface validation failed");
        assert!(error.source().is_none());
    }
}
