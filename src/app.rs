use std::{error::Error, sync::Arc};

use crate::renderer::Renderer;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

const WINDOW_TITLE: &str = "Orbis";
const INITIAL_WIDTH: f64 = 1280.0;
const INITIAL_HEIGHT: f64 = 720.0;

#[derive(Default)]
struct App {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    error: Option<Box<dyn Error>>,
    initial_redraw_pending: bool,
}

impl App {
    fn create_window(&mut self, event_loop: &ActiveEventLoop) {
        let attributes = Window::default_attributes()
            .with_title(WINDOW_TITLE)
            .with_inner_size(LogicalSize::new(INITIAL_WIDTH, INITIAL_HEIGHT));
        let window = match event_loop.create_window(attributes) {
            Ok(window) => Arc::new(window),
            Err(error) => return self.exit_with_error(event_loop, error),
        };
        let renderer = match pollster::block_on(Renderer::new(window.clone())) {
            Ok(renderer) => renderer,
            Err(error) => return self.exit_with_error(event_loop, error),
        };

        self.window = Some(window);
        self.renderer = Some(renderer);
        self.initial_redraw_pending = true;
    }

    fn exit_with_error(&mut self, event_loop: &ActiveEventLoop, error: impl Error + 'static) {
        self.error = Some(Box::new(error));
        event_loop.exit();
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() && self.error.is_none() {
            self.create_window(event_loop);
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if !self.initial_redraw_pending {
            return;
        }

        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
            self.initial_redraw_pending = false;
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(window) = self.window.as_ref() else {
            return;
        };

        if window.id() != window_id {
            return;
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Focused(true) | WindowEvent::Occluded(false) => {
                window.request_redraw();
            }
            WindowEvent::Resized(size) => {
                let Some(renderer) = self.renderer.as_mut() else {
                    return;
                };

                renderer.resize(size);
                window.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                let Some(renderer) = self.renderer.as_mut() else {
                    return;
                };

                if let Err(error) = renderer.render() {
                    self.exit_with_error(event_loop, error);
                }
            }
            _ => {}
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
