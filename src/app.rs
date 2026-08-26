use std::{error::Error, sync::Arc};

use crate::renderer::{RenderOutcome, Renderer, RendererError};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

const WINDOW_TITLE: &str = "Orbis";
const INITIAL_WIDTH: f64 = 1280.0;
const INITIAL_HEIGHT: f64 = 720.0;
const PIXELS_PER_SCROLL_LINE: f64 = 100.0;

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

#[derive(Default)]
struct OrbitInput {
    dragging: bool,
    cursor_position: Option<PhysicalPosition<f64>>,
}

impl OrbitInput {
    fn mouse_button(&mut self, state: ElementState, button: MouseButton) {
        if button != MouseButton::Left {
            return;
        }

        self.dragging = state == ElementState::Pressed;
        self.cursor_position = None;
    }

    fn cursor_moved(&mut self, position: PhysicalPosition<f64>) -> Option<(f32, f32)> {
        if !self.dragging {
            return None;
        }

        let previous_position = self.cursor_position.replace(position)?;
        Some((
            (position.x - previous_position.x) as f32,
            (position.y - previous_position.y) as f32,
        ))
    }

    fn reset(&mut self) {
        self.dragging = false;
        self.cursor_position = None;
    }
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

    fn cursor_moved(&mut self, position: PhysicalPosition<f64>) {
        let Some((delta_x, delta_y)) = self.orbit_input.cursor_moved(position) else {
            return;
        };

        if self.renderer.orbit_camera(delta_x, delta_y) {
            self.request_redraw();
        }
    }

    fn mouse_wheel(&mut self, delta: MouseScrollDelta) {
        if self.renderer.zoom_camera(scroll_amount(delta)) {
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
            WindowEvent::CursorMoved { position, .. } => window_state.cursor_moved(position),
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
}

fn scroll_amount(delta: MouseScrollDelta) -> f32 {
    match delta {
        MouseScrollDelta::LineDelta(_, vertical) => vertical,
        MouseScrollDelta::PixelDelta(position) => (position.y / PIXELS_PER_SCROLL_LINE) as f32,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orbit_drag_anchors_before_reporting_movement() {
        let mut input = OrbitInput::default();

        assert_eq!(input.cursor_moved(PhysicalPosition::new(10.0, 20.0)), None);
        input.mouse_button(ElementState::Pressed, MouseButton::Left);
        assert_eq!(input.cursor_moved(PhysicalPosition::new(10.0, 20.0)), None);
        assert_eq!(
            input.cursor_moved(PhysicalPosition::new(15.0, 12.0)),
            Some((5.0, -8.0))
        );
    }

    #[test]
    fn orbit_drag_stops_after_release_or_reset() {
        let mut input = OrbitInput::default();
        input.mouse_button(ElementState::Pressed, MouseButton::Left);
        input.cursor_moved(PhysicalPosition::new(10.0, 20.0));

        input.mouse_button(ElementState::Released, MouseButton::Left);
        assert_eq!(input.cursor_moved(PhysicalPosition::new(15.0, 12.0)), None);

        input.mouse_button(ElementState::Pressed, MouseButton::Left);
        input.cursor_moved(PhysicalPosition::new(10.0, 20.0));
        input.reset();
        assert_eq!(input.cursor_moved(PhysicalPosition::new(15.0, 12.0)), None);
    }

    #[test]
    fn scroll_input_normalizes_lines_and_pixels() {
        assert_eq!(scroll_amount(MouseScrollDelta::LineDelta(0.0, 2.0)), 2.0);
        assert_eq!(
            scroll_amount(MouseScrollDelta::PixelDelta(PhysicalPosition::new(
                0.0, 150.0
            ))),
            1.5
        );
    }
}
