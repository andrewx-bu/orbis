mod surface;

use std::{error::Error, fmt, sync::Arc};

use self::surface::SurfaceState;
use wgpu::{
    Color, CommandEncoderDescriptor, LoadOp, Operations, RenderPassColorAttachment,
    RenderPassDescriptor, StoreOp, TextureViewDescriptor,
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
}

impl Renderer {
    pub async fn new(window: Arc<Window>) -> Result<Self, RendererError> {
        let surface = SurfaceState::new(window).await?;

        Ok(Self { surface })
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        self.surface.resize(size);
    }

    pub fn render(&mut self) -> Result<(), RendererError> {
        let Some(frame) = self.surface.acquire_frame()? else {
            return Ok(());
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
            let _render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Orbis clear pass"),
                color_attachments: &[Some(color_attachment)],
                ..Default::default()
            });
        }

        self.surface.present(encoder.finish(), frame);

        Ok(())
    }
}

#[derive(Debug)]
pub enum RendererError {
    CreateSurface(wgpu::CreateSurfaceError),
    RequestAdapter(wgpu::RequestAdapterError),
    RequestDevice(wgpu::RequestDeviceError),
    UnsupportedSurface,
    SurfaceValidation,
}

impl fmt::Display for RendererError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CreateSurface(error) => write!(formatter, "failed to create surface: {error}"),
            Self::RequestAdapter(error) => write!(formatter, "failed to request adapter: {error}"),
            Self::RequestDevice(error) => write!(formatter, "failed to request device: {error}"),
            Self::UnsupportedSurface => formatter.write_str("surface is not supported by adapter"),
            Self::SurfaceValidation => formatter.write_str("surface validation failed"),
        }
    }
}

impl Error for RendererError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::CreateSurface(error) => Some(error),
            Self::RequestAdapter(error) => Some(error),
            Self::RequestDevice(error) => Some(error),
            Self::UnsupportedSurface | Self::SurfaceValidation => None,
        }
    }
}
