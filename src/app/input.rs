use winit::event::{ElementState, MouseButton, MouseScrollDelta};

const PIXELS_PER_SCROLL_LINE: f64 = 100.0;

#[derive(Default)]
pub(super) struct OrbitInput {
    dragging: bool,
}

impl OrbitInput {
    pub(super) fn mouse_button(&mut self, state: ElementState, button: MouseButton) {
        if button != MouseButton::Left {
            return;
        }

        self.dragging = state == ElementState::Pressed;
    }

    pub(super) fn mouse_motion(&self, delta: (f64, f64)) -> Option<(f32, f32)> {
        if !self.dragging {
            return None;
        }

        Some((delta.0 as f32, delta.1 as f32))
    }

    pub(super) fn reset(&mut self) {
        self.dragging = false;
    }
}

pub(super) fn scroll_amount(delta: MouseScrollDelta, scale_factor: f64) -> f32 {
    match delta {
        MouseScrollDelta::LineDelta(_, vertical) => vertical,
        MouseScrollDelta::PixelDelta(position) => {
            (position.to_logical::<f64>(scale_factor).y / PIXELS_PER_SCROLL_LINE) as f32
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use winit::dpi::PhysicalPosition;

    #[test]
    fn orbit_drag_reports_raw_movement_while_left_button_is_pressed() {
        let mut input = OrbitInput::default();

        assert_eq!(input.mouse_motion((5.0, -8.0)), None);
        input.mouse_button(ElementState::Pressed, MouseButton::Left);
        assert_eq!(input.mouse_motion((5.0, -8.0)), Some((5.0, -8.0)));
    }

    #[test]
    fn orbit_drag_stops_after_release_or_reset() {
        let mut input = OrbitInput::default();
        input.mouse_button(ElementState::Pressed, MouseButton::Left);

        input.mouse_button(ElementState::Released, MouseButton::Left);
        assert_eq!(input.mouse_motion((5.0, -8.0)), None);

        input.mouse_button(ElementState::Pressed, MouseButton::Left);
        input.reset();
        assert_eq!(input.mouse_motion((5.0, -8.0)), None);
    }

    #[test]
    fn scroll_input_normalizes_lines_and_pixels() {
        assert_eq!(
            scroll_amount(MouseScrollDelta::LineDelta(0.0, 2.0), 2.0),
            2.0
        );
        assert_eq!(
            scroll_amount(
                MouseScrollDelta::PixelDelta(PhysicalPosition::new(0.0, 150.0)),
                1.0
            ),
            1.5
        );
        assert_eq!(
            scroll_amount(
                MouseScrollDelta::PixelDelta(PhysicalPosition::new(0.0, 300.0)),
                2.0
            ),
            1.5
        );
    }
}
