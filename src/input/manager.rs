use smithay::{
    input::{
        keyboard::FilterResult,
        pointer::{ButtonEvent, MotionEvent},
        Seat,
    },
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    utils::{Point, SERIAL_COUNTER},
};

use crate::compositor::CompositorStateData;

pub struct InputManager {
    pub focused_surface: Option<WlSurface>,
}

impl InputManager {
    pub fn new() -> Self {
        Self {
            focused_surface: None,
        }
    }

    pub fn set_focus(
        &mut self,
        seat: &Seat<CompositorStateData>,
        state: &mut CompositorStateData,
        surface: Option<&WlSurface>,
    ) {
        if self.focused_surface.as_ref() == surface {
            return;
        }

        self.focused_surface = surface.cloned();

        let serial = SERIAL_COUNTER.next_serial();
        if let Some(keyboard) = seat.get_keyboard() {
            keyboard.set_focus(state, surface.cloned(), serial);
        }
    }

    pub fn send_pointer_motion(
        &mut self,
        seat: &Seat<CompositorStateData>,
        state: &mut CompositorStateData,
        surface: &WlSurface,
        local_pos: Point<f64, smithay::utils::Logical>,
        time: u32,
    ) {
        if let Some(pointer) = seat.get_pointer() {
            let serial = SERIAL_COUNTER.next_serial();
            pointer.motion(
                state,
                Some((surface.clone(), local_pos)),
                &MotionEvent {
                    location: local_pos,
                    serial,
                    time,
                },
            );
            pointer.frame(state);
        }
    }

    pub fn send_pointer_button(
        &mut self,
        seat: &Seat<CompositorStateData>,
        state: &mut CompositorStateData,
        button: u32,
        is_pressed: bool,
        time: u32,
    ) {
        if let Some(pointer) = seat.get_pointer() {
            let serial = SERIAL_COUNTER.next_serial();
            let button_state = if is_pressed {
                smithay::backend::input::ButtonState::Pressed
            } else {
                smithay::backend::input::ButtonState::Released
            };

            pointer.button(
                state,
                &ButtonEvent {
                    serial,
                    time,
                    button,
                    state: button_state,
                },
            );
            pointer.frame(state);
        }
    }

    pub fn send_key(
        &mut self,
        seat: &Seat<CompositorStateData>,
        state: &mut CompositorStateData,
        key_code: u32,
        is_pressed: bool,
        time: u32,
    ) {
        if let Some(keyboard) = seat.get_keyboard() {
            let serial = SERIAL_COUNTER.next_serial();
            let key_state = if is_pressed {
                smithay::backend::input::KeyState::Pressed
            } else {
                smithay::backend::input::KeyState::Released
            };

            keyboard.input::<(), _>(
                state,
                key_code.into(),
                key_state,
                serial,
                time,
                |_, _, _| FilterResult::Forward,
            );
        }
    }
}
