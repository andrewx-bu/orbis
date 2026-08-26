use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, MouseButton, MouseScrollDelta},
};

const PIXELS_PER_SCROLL_LINE: f64 = 100.0;

#[derive(Default)]
pub(super) struct OrbitInput {
    dragging: bool,
    cursor_position: Option<PhysicalPosition<f64>>,
}

impl OrbitInput {
    pub(super) fn mouse_button(&mut self, state: ElementState, button: MouseButton) {
        if button != MouseButton::Left {
            return;
        }

        self.dragging = state == ElementState::Pressed;
        self.cursor_position = None;
    }

    pub(super) fn cursor_moved(&mut self, position: PhysicalPosition<f64>) -> Option<(f32, f32)> {
        if !self.dragging {
            return None;
        }

        let previous_position = self.cursor_position.replace(position)?;
        Some((
            (position.x - previous_position.x) as f32,
            (position.y - previous_position.y) as f32,
        ))
    }

    pub(super) fn reset(&mut self) {
        self.dragging = false;
        self.cursor_position = None;
    }
}

pub(super) fn scroll_amount(delta: MouseScrollDelta) -> f32 {
    match delta {
        MouseScrollDelta::LineDelta(_, vertical) => vertical,
        MouseScrollDelta::PixelDelta(position) => (position.y / PIXELS_PER_SCROLL_LINE) as f32,
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
