use std::sync::Arc;

use super::RendererError;
use wgpu::{
    CommandBuffer, CurrentSurfaceTexture, Device, DeviceDescriptor, Instance, Queue,
    RequestAdapterOptions, Surface, SurfaceConfiguration, SurfaceTexture, Texture,
};
use winit::{dpi::PhysicalSize, window::Window};

const FRAME_ACQUISITION_ATTEMPTS: usize = 2;

pub(super) struct SurfaceState {
    window: Arc<Window>,
    instance: Instance,
    surface: Surface<'static>,
    adapter: wgpu::Adapter,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    size: PhysicalSize<u32>,
}

impl SurfaceState {
    pub(super) async fn new(window: Arc<Window>) -> Result<Self, RendererError> {
        let size = window.inner_size();
        let instance = Instance::default();
        let surface = instance
            .create_surface(window.clone())
            .map_err(RendererError::create_surface)?;
        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .map_err(RendererError::request_adapter)?;
        let (device, queue) = adapter
            .request_device(&DeviceDescriptor {
                label: Some("Orbis device"),
                ..Default::default()
            })
            .await
            .map_err(RendererError::request_device)?;
        let config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .ok_or_else(RendererError::unsupported_surface)?;

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

    pub(super) fn device(&self) -> &Device {
        &self.device
    }

    pub(super) fn format(&self) -> wgpu::TextureFormat {
        self.config.format
    }

    pub(super) fn resize(&mut self, size: PhysicalSize<u32>) {
        self.size = size;

        if self.is_minimized() {
            return;
        }

        self.config.width = size.width;
        self.config.height = size.height;
        self.configure();
    }

    pub(super) fn acquire_frame(&mut self) -> Result<FrameAcquisition, RendererError> {
        if self.is_minimized() {
            return Ok(FrameAcquisition::Wait);
        }

        for _ in 0..FRAME_ACQUISITION_ATTEMPTS {
            match self.surface.get_current_texture() {
                CurrentSurfaceTexture::Success(texture) => {
                    return Ok(FrameAcquisition::Ready(SurfaceFrame::optimal(texture)));
                }
                CurrentSurfaceTexture::Suboptimal(texture) => {
                    return Ok(FrameAcquisition::Ready(SurfaceFrame::suboptimal(texture)));
                }
                CurrentSurfaceTexture::Timeout => return Ok(FrameAcquisition::Retry),
                CurrentSurfaceTexture::Occluded => {
                    return Ok(FrameAcquisition::Wait);
                }
                CurrentSurfaceTexture::Outdated => self.configure(),
                CurrentSurfaceTexture::Lost => self.recreate()?,
                CurrentSurfaceTexture::Validation => {
                    return Err(RendererError::surface_validation());
                }
            }
        }

        Ok(FrameAcquisition::Retry)
    }

    pub(super) fn present(&mut self, commands: CommandBuffer, frame: SurfaceFrame) {
        self.queue.submit([commands]);
        self.window.pre_present_notify();
        self.queue.present(frame.texture);

        if frame.reconfigure_after_present {
            self.configure();
        }
    }

    fn is_minimized(&self) -> bool {
        self.size.width == 0 || self.size.height == 0
    }

    fn configure(&self) {
        self.surface.configure(&self.device, &self.config);
    }

    fn recreate(&mut self) -> Result<(), RendererError> {
        let surface = self
            .instance
            .create_surface(self.window.clone())
            .map_err(RendererError::create_surface)?;
        let config = surface
            .get_default_config(&self.adapter, self.size.width, self.size.height)
            .ok_or_else(RendererError::unsupported_surface)?;

        surface.configure(&self.device, &config);
        self.surface = surface;
        self.config = config;

        Ok(())
    }
}

pub(super) enum FrameAcquisition {
    Ready(SurfaceFrame),
    Retry,
    Wait,
}

pub(super) struct SurfaceFrame {
    texture: SurfaceTexture,
    reconfigure_after_present: bool,
}

impl SurfaceFrame {
    fn optimal(texture: SurfaceTexture) -> Self {
        Self {
            texture,
            reconfigure_after_present: false,
        }
    }

    fn suboptimal(texture: SurfaceTexture) -> Self {
        Self {
            texture,
            reconfigure_after_present: true,
        }
    }

    pub(super) fn texture(&self) -> &Texture {
        &self.texture.texture
    }
}
