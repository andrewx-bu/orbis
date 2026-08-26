mod input;

use std::{error::Error, sync::Arc};

use self::input::{OrbitInput, scroll_amount};
use crate::renderer::{RenderOutcome, Renderer, RendererError};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalSize},
    event::{DeviceEvent, DeviceId, ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

const WINDOW_TITLE: &str = "Orbis";
const INITIAL_WIDTH: f64 = 1280.0;
const INITIAL_HEIGHT: f64 = 720.0;

#[derive(Default)]
struct App {
    window_state: Option<WindowState>,
    error: Option<Box<dyn Error>>,
}

struct WindowState {
    window: Arc<Window>,
    renderer: Renderer,
    orbit_input: OrbitInput,
    initial_redraw_pending: bool,
}

impl WindowState {
    fn new(event_loop: &ActiveEventLoop) -> Result<Self, Box<dyn Error>> {
        let attributes = Window::default_attributes()
            .with_title(WINDOW_TITLE)
            .with_inner_size(LogicalSize::new(INITIAL_WIDTH, INITIAL_HEIGHT));
        let window = Arc::new(event_loop.create_window(attributes)?);
        let renderer = Renderer::new(window.clone())?;

        Ok(Self {
            window,
            renderer,
            orbit_input: OrbitInput::default(),
            initial_redraw_pending: true,
        })
    }

    fn id(&self) -> WindowId {
        self.window.id()
    }

    fn request_redraw(&self) {
        self.window.request_redraw();
    }

    fn request_initial_redraw(&mut self) {
        if self.initial_redraw_pending {
            self.request_redraw();
            self.initial_redraw_pending = false;
        }
    }

    fn resize(&mut self, size: PhysicalSize<u32>) {
        self.renderer.resize(size);
        self.request_redraw();
    }

    fn mouse_button(&mut self, state: ElementState, button: MouseButton) {
        self.orbit_input.mouse_button(state, button);
    }

    fn mouse_motion(&mut self, delta: (f64, f64)) {
        let Some((delta_x, delta_y)) = self.orbit_input.mouse_motion(delta) else {
            return;
        };

        if self.renderer.orbit_camera(delta_x, delta_y) {
            self.request_redraw();
        }
    }

    fn mouse_wheel(&mut self, delta: MouseScrollDelta) {
        if self
            .renderer
            .zoom_camera(scroll_amount(delta, self.window.scale_factor()))
        {
            self.request_redraw();
        }
    }

    fn reset_orbit_input(&mut self) {
        self.orbit_input.reset();
    }

    fn render(&mut self) -> Result<(), RendererError> {
        if self.renderer.render()? == RenderOutcome::Retry {
            self.request_redraw();
        }

        Ok(())
    }
}

impl App {
    fn create_window(&mut self, event_loop: &ActiveEventLoop) {
        let window_state = match WindowState::new(event_loop) {
            Ok(window_state) => window_state,
            Err(error) => return self.exit_with_error(event_loop, error),
        };

        self.window_state = Some(window_state);
    }

    fn exit_with_error(&mut self, event_loop: &ActiveEventLoop, error: impl Into<Box<dyn Error>>) {
        self.error = Some(error.into());
        event_loop.exit();
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window_state.is_none() && self.error.is_none() {
            self.create_window(event_loop);
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window_state) = self.window_state.as_mut() {
            window_state.request_initial_redraw();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(window_state) = self.window_state.as_mut() else {
            return;
        };

        if window_state.id() != window_id {
            return;
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Focused(true) | WindowEvent::Occluded(false) => {
                window_state.request_redraw();
            }
            WindowEvent::Focused(false) | WindowEvent::CursorLeft { .. } => {
                window_state.reset_orbit_input();
            }
            WindowEvent::MouseInput { state, button, .. } => {
                window_state.mouse_button(state, button);
            }
            WindowEvent::MouseWheel { delta, .. } => window_state.mouse_wheel(delta),
            WindowEvent::Resized(size) => window_state.resize(size),
            WindowEvent::RedrawRequested => {
                if let Err(error) = window_state.render() {
                    self.exit_with_error(event_loop, error);
                }
            }
            _ => {}
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        let Some(window_state) = self.window_state.as_mut() else {
            return;
        };

        if let DeviceEvent::MouseMotion { delta } = event {
            window_state.mouse_motion(delta);
        }
    }
}

pub fn run() -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = App::default();
    event_loop.run_app(&mut app)?;

    match app.error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}
