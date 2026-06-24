//! Event-handler action helpers for the native terminal app.

use std::sync::Arc;

use winit::event_loop::ActiveEventLoop;
use winit::window::Window;

use super::super::{
    NativeAppAction, NativeAppError, NativeGlyphFrameError, NativeTerminalApp,
    NativeWindowMouseInput,
};
use crate::clipboard::NativeClipboard;
use crate::mouse::{MouseButton, MouseEventKind};
use crate::native_gpu::{GpuBootstrap, GpuBootstrapConfig};
use crate::renderer::SurfaceFrameError;

impl NativeTerminalApp {
    pub(super) fn pump_output_and_apply_action(&mut self, event_loop: &ActiveEventLoop) {
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

    pub(super) fn handle_redraw_requested(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.pre_present_notify();
        }
        match self.present_redraw_frame() {
            Ok(report) => self.lifecycle.record_glyph_frame_presentation(report),
            Err(error) => match error {
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
            },
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

    pub(super) fn handle_window_resized(
        &mut self,
        event_loop: &ActiveEventLoop,
        width: u32,
        height: u32,
    ) {
        if let Some(surface) = &mut self.surface
            && let Err(error) = surface.resize(width, height)
        {
            self.startup_error = Some(error.to_string());
            event_loop.exit();
            return;
        }
        if let Err(error) = self.resize_runtime_to_window_pixels(width, height) {
            self.startup_error = Some(error.to_string());
            event_loop.exit();
        }
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    pub(super) fn handle_key_press(
        &mut self,
        event_loop: &ActiveEventLoop,
        logical_key: winit::keyboard::Key,
        physical_key: winit::keyboard::PhysicalKey,
    ) {
        let result = if let Some(action) =
            super::super::native_text_zoom_action(&logical_key, self.modifiers)
        {
            self.apply_text_zoom_action(action).map(|changed| {
                if changed && let Some(window) = &self.window {
                    window.request_redraw();
                }
            })
        } else if super::super::is_native_copy_shortcut(&logical_key, self.modifiers) {
            let mut clipboard = NativeClipboard::new();
            self.runtime.copy_selection_to_clipboard(&mut clipboard);
            Ok(())
        } else if super::super::is_native_paste_shortcut(&logical_key, self.modifiers) {
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

    pub(super) fn create_surface_for_window(
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
        let surface =
            super::super::NativeWindowSurface::from_gpu_surface(gpu_surface, width, height)
                .map_err(NativeAppError::from)?;
        self.gpu_context = Some(context);
        self.surface = Some(surface);
        Ok(())
    }

    pub(super) fn send_current_mouse_input(
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
