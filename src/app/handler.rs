use std::sync::Arc;
use std::time::Instant;

use winit::application::ApplicationHandler;
use winit::event::{ElementState, Ime, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::window::{Window, WindowId};

use super::{
    NativeAppAction, NativeAppError, NativeAppEvent, NativeGlyphFrameError, NativeTerminalApp,
    NativeWindowMouseInput, native_mouse_button, wheel_mouse_button,
};
use crate::clipboard::NativeClipboard;
use crate::mouse::{MouseButton, MouseEventKind};
use crate::native_gpu::{GpuBootstrap, GpuBootstrapConfig};
use crate::renderer::SurfaceFrameError;

impl ApplicationHandler<NativeAppEvent> for NativeTerminalApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.lifecycle.on_resumed() != NativeAppAction::CreateWindow {
            return;
        }
        match event_loop.create_window(self.lifecycle.config().window_attributes()) {
            Ok(window) => {
                let window = Arc::new(window);
                let size = window.inner_size();
                self.window_id = Some(window.id());
                if let Err(error) =
                    self.create_surface_for_window(Arc::clone(&window), size.width, size.height)
                {
                    self.startup_error = Some(error.to_string());
                    event_loop.exit();
                    return;
                }
                self.window = Some(window);
                self.lifecycle.on_window_created();
                if let Err(error) = self.runtime.start_shell(&self.pty_spawner) {
                    self.startup_error = Some(error.to_string());
                    event_loop.exit();
                }
            }
            Err(error) => {
                self.startup_error = Some(error.to_string());
                event_loop.exit();
            }
        }
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
        if let Some(deadline) = self.lifecycle.next_pty_pump_deadline(Instant::now()) {
            event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
        }
        self.pump_output_and_apply_action(event_loop);
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
            WindowEvent::Ime(Ime::Commit(text)) => {
                if let Err(error) = self.runtime.send_committed_text(&text) {
                    self.startup_error = Some(error.to_string());
                    event_loop.exit();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_position = Some(position);
                let (kind, button) = self.mouse_buttons.cursor_move_event();
                if let Err(error) = self.send_current_mouse_input(kind, button) {
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
            WindowEvent::MouseInput { state, button, .. } => {
                if let Some(button) = native_mouse_button(button) {
                    let kind = if state == ElementState::Pressed {
                        self.mouse_buttons.set_pressed(button, true);
                        MouseEventKind::Press
                    } else {
                        MouseEventKind::Release
                    };
                    if let Err(error) = self.send_current_mouse_input(kind, button) {
                        self.startup_error = Some(error.to_string());
                        event_loop.exit();
                    }
                    if state == ElementState::Released {
                        self.mouse_buttons.set_pressed(button, false);
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                if let Some(button) = wheel_mouse_button(delta)
                    && let Err(error) = self.send_current_mouse_input(MouseEventKind::Press, button)
                {
                    self.startup_error = Some(error.to_string());
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }
}

impl NativeTerminalApp {
    fn pump_output_and_apply_action(&mut self, event_loop: &ActiveEventLoop) {
        let mut clipboard = NativeClipboard::new();
        let action = self
            .runtime
            .pump_output_sync_clipboard_and_schedule_redraw(&mut self.lifecycle, &mut clipboard)
            .unwrap_or_else(|error| {
                self.startup_error = Some(error.to_string());
                NativeAppAction::Exit
            });
        match action {
            NativeAppAction::RequestRedraw => {
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            NativeAppAction::Exit => event_loop.exit(),
            NativeAppAction::None | NativeAppAction::CreateWindow => {}
        }
    }

    fn handle_redraw_requested(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.pre_present_notify();
        }
        if let Err(error) = self.present_redraw_frame() {
            match error {
                NativeGlyphFrameError::Surface(
                    SurfaceFrameError::Timeout | SurfaceFrameError::Occluded,
                ) => {}
                NativeGlyphFrameError::Surface(
                    SurfaceFrameError::Outdated
                    | SurfaceFrameError::Lost
                    | SurfaceFrameError::Validation
                    | SurfaceFrameError::InvalidFrame(_),
                )
                | NativeGlyphFrameError::Font(_)
                | NativeGlyphFrameError::Renderer(_) => {
                    self.startup_error = Some(error.to_string());
                    event_loop.exit();
                }
            }
        }
        match self.lifecycle.on_redraw_requested() {
            NativeAppAction::RequestRedraw => {
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            NativeAppAction::Exit => event_loop.exit(),
            NativeAppAction::None | NativeAppAction::CreateWindow => {}
        }
    }

    fn handle_window_resized(&mut self, event_loop: &ActiveEventLoop, width: u32, height: u32) {
        if let Some(surface) = &mut self.surface
            && let Err(error) = surface.resize(width, height)
        {
            self.startup_error = Some(error.to_string());
            event_loop.exit();
            return;
        }
        if let Some(resize) = self.resize_mapper.resize_for_window(width, height)
            && let Err(error) = self.runtime.resize_terminal(resize)
        {
            self.startup_error = Some(error.to_string());
            event_loop.exit();
        }
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn handle_key_press(
        &mut self,
        event_loop: &ActiveEventLoop,
        logical_key: winit::keyboard::Key,
        physical_key: winit::keyboard::PhysicalKey,
    ) {
        let result = if super::is_native_copy_shortcut(&logical_key, self.modifiers) {
            let mut clipboard = NativeClipboard::new();
            self.runtime.copy_selection_to_clipboard(&mut clipboard);
            Ok(())
        } else if super::is_native_paste_shortcut(&logical_key, self.modifiers) {
            let clipboard = NativeClipboard::new();
            self.runtime.send_clipboard_paste(&clipboard).map(|_| ())
        } else {
            self.runtime
                .send_winit_key_event_input(&logical_key, Some(physical_key), self.modifiers)
                .map(|_| ())
        };
        if let Err(error) = result {
            self.startup_error = Some(error.to_string());
            event_loop.exit();
        }
    }

    fn create_surface_for_window(
        &mut self,
        window: Arc<Window>,
        width: u32,
        height: u32,
    ) -> Result<(), NativeAppError> {
        let context = GpuBootstrap::new(GpuBootstrapConfig::native_default())
            .initialize_native()
            .map_err(NativeAppError::from)?;
        let gpu_surface = context
            .create_window_surface(window)
            .map_err(NativeAppError::from)?;
        let surface = super::NativeWindowSurface::from_gpu_surface(gpu_surface, width, height)
            .map_err(NativeAppError::from)?;
        self.gpu_context = Some(context);
        self.surface = Some(surface);
        Ok(())
    }

    fn send_current_mouse_input(
        &mut self,
        kind: MouseEventKind,
        button: MouseButton,
    ) -> Result<(), NativeAppError> {
        let (Some(position), Some(window)) = (self.cursor_position, self.window.as_ref()) else {
            return Ok(());
        };
        let size = window.inner_size();
        self.runtime
            .send_window_mouse_input_event(NativeWindowMouseInput {
                x: position.x,
                y: position.y,
                window_width_px: size.width,
                window_height_px: size.height,
                cell_width_px: self.renderer.config().cell_width_px,
                line_height_px: self.renderer.config().line_height_px,
                surface_padding_px: self.renderer.config().surface_padding_px,
                kind,
                button,
                modifiers: self.modifiers,
            })
            .map(|_| ())
    }
}
