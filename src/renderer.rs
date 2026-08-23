use std::{error::Error, fmt, sync::Arc};

use wgpu::{
    Color, CommandEncoderDescriptor, CurrentSurfaceTexture, Device, DeviceDescriptor, Instance,
    LoadOp, Operations, Queue, RenderPassColorAttachment, RenderPassDescriptor,
    RequestAdapterOptions, StoreOp, Surface, SurfaceConfiguration, TextureViewDescriptor,
};
use winit::{dpi::PhysicalSize, window::Window};

const CLEAR_COLOR: Color = Color {
    r: 0.02,
    g: 0.03,
    b: 0.05,
    a: 1.0,
};

pub struct Renderer {
    window: Arc<Window>,
    instance: Instance,
    surface: Surface<'static>,
    adapter: wgpu::Adapter,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    size: PhysicalSize<u32>,
}

impl Renderer {
    pub async fn new(window: Arc<Window>) -> Result<Self, RendererError> {
        let size = window.inner_size();
        let instance = Instance::default();
        let surface = instance
            .create_surface(window.clone())
            .map_err(RendererError::CreateSurface)?;
        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .map_err(RendererError::RequestAdapter)?;
        let (device, queue) = adapter
            .request_device(&DeviceDescriptor {
                label: Some("Orbis device"),
                ..Default::default()
            })
            .await
            .map_err(RendererError::RequestDevice)?;
        let config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .ok_or(RendererError::UnsupportedSurface)?;

        surface.configure(&device, &config);

        Ok(Self {
            window,
            instance,
            surface,
            adapter,
            device,
            queue,
            config,
            size,
        })
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        self.size = size;

        if size.width == 0 || size.height == 0 {
            return;
        }

        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(&self.device, &self.config);
    }

    pub fn render(&mut self) -> Result<(), RendererError> {
        if self.size.width == 0 || self.size.height == 0 {
            return Ok(());
        }

        let (frame, should_reconfigure) = match self.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(frame) => (frame, false),
            CurrentSurfaceTexture::Suboptimal(frame) => (frame, true),
            CurrentSurfaceTexture::Timeout | CurrentSurfaceTexture::Occluded => return Ok(()),
            CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            CurrentSurfaceTexture::Lost => {
                self.recreate_surface()?;
                return Ok(());
            }
            CurrentSurfaceTexture::Validation => return Err(RendererError::SurfaceValidation),
        };
        let view = frame.texture.create_view(&TextureViewDescriptor::default());
        let mut encoder = self
            .device
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

        self.queue.submit([encoder.finish()]);
        self.queue.present(frame);

        if should_reconfigure {
            self.surface.configure(&self.device, &self.config);
        }

        Ok(())
    }

    fn recreate_surface(&mut self) -> Result<(), RendererError> {
        let surface = self
            .instance
            .create_surface(self.window.clone())
            .map_err(RendererError::CreateSurface)?;
        let config = surface
            .get_default_config(&self.adapter, self.size.width, self.size.height)
            .ok_or(RendererError::UnsupportedSurface)?;

        surface.configure(&self.device, &config);
        self.surface = surface;
        self.config = config;

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
