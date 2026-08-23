use std::error::Error;

use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    error::OsError,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

const WINDOW_TITLE: &str = "Orbis";
const INITIAL_WIDTH: f64 = 1280.0;
const INITIAL_HEIGHT: f64 = 720.0;

#[derive(Default)]
struct App {
    window: Option<Window>,
    startup_error: Option<OsError>,
}

impl App {
    fn create_window(&mut self, event_loop: &ActiveEventLoop) {
        let attributes = Window::default_attributes()
            .with_title(WINDOW_TITLE)
            .with_inner_size(LogicalSize::new(INITIAL_WIDTH, INITIAL_HEIGHT));

        match event_loop.create_window(attributes) {
            Ok(window) => self.window = Some(window),
            Err(error) => {
                self.startup_error = Some(error);
                event_loop.exit();
            }
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() && self.startup_error.is_none() {
            self.create_window(event_loop);
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

        if let WindowEvent::CloseRequested = event {
            event_loop.exit();
        }
    }
}

pub fn run() -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = App::default();
    event_loop.run_app(&mut app)?;

    match app.startup_error {
        Some(error) => Err(Box::new(error)),
        None => Ok(()),
    }
}
