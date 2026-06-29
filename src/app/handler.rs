use std::time::Instant;

use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::window::WindowId;

use super::{NativeAppEvent, NativeTerminalApp, native_mouse_button, wheel_mouse_button};
use crate::mouse::MouseEventKind;

mod actions;
mod mouse;
mod resume;
mod surface;
mod window_metadata;

impl ApplicationHandler<NativeAppEvent> for NativeTerminalApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.handle_resumed(event_loop);
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        match self.reload_config_if_changed() {
            Ok(true) => {
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            Ok(false) => {}
            Err(error) => {
                self.startup_error = Some(error.to_string());
                event_loop.exit();
                return;
            }
        }
        self.pump_output_and_apply_action(event_loop);
        if let Some(deadline) = self.lifecycle.next_pty_pump_deadline(Instant::now()) {
            event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: NativeAppEvent) {
        match event {
            NativeAppEvent::PtyOutputReady => self.pump_output_and_apply_action(event_loop),
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if Some(window_id) != self.window_id {
            return;
        }
        match event {
            WindowEvent::CloseRequested => {
                self.lifecycle.on_close_requested();
                event_loop.exit();
            }
            WindowEvent::Destroyed => {
                self.lifecycle.on_destroyed();
                self.surface = None;
                self.gpu_context = None;
                self.window = None;
                self.window_id = None;
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => self.handle_redraw_requested(event_loop),
            WindowEvent::Resized(size) => {
                self.handle_window_resized(event_loop, size.width, size.height);
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state.is_pressed() {
                    self.handle_key_press(event_loop, event.logical_key, event.physical_key);
                }
            }
            WindowEvent::Ime(event) => self.handle_ime_event(event_loop, event),
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_position = Some(position);
                let (kind, button) = self.mouse_buttons.cursor_move_event();
                if let Err(error) = self.send_current_mouse_input_and_request_redraw(kind, button) {
                    self.startup_error = Some(error.to_string());
                    event_loop.exit();
                }
            }
            WindowEvent::Focused(focused) => {
                if let Err(error) = self.runtime.send_focus_event(focused).map(|_| ()) {
                    self.startup_error = Some(error.to_string());
                    event_loop.exit();
                }
            }
            WindowEvent::Occluded(false) => {
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::Occluded(true) => {}
            WindowEvent::MouseInput { state, button, .. } => {
                if let Some(button) = native_mouse_button(button) {
                    let kind = if state == ElementState::Pressed {
                        self.mouse_buttons.set_pressed(button, true);
                        MouseEventKind::Press
                    } else {
                        MouseEventKind::Release
                    };
                    if let Err(error) =
                        self.send_current_mouse_input_and_request_redraw(kind, button)
                    {
                        self.startup_error = Some(error.to_string());
                        event_loop.exit();
                    }
                    if state == ElementState::Released {
                        self.mouse_buttons.set_pressed(button, false);
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                if let Some(action) = super::native_wheel_text_zoom_action(&delta, self.modifiers) {
                    if let Err(error) = self.apply_text_zoom_action(action).map(|changed| {
                        if changed && let Some(window) = &self.window {
                            window.request_redraw();
                        }
                    }) {
                        self.startup_error = Some(error.to_string());
                        event_loop.exit();
                    }
                } else if let Some(button) = wheel_mouse_button(&delta)
                    && let Err(error) = self
                        .send_current_mouse_input_and_request_redraw(MouseEventKind::Press, button)
                {
                    self.startup_error = Some(error.to_string());
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }
}
