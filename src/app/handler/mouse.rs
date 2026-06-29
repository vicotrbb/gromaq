//! Mouse event helpers for the native app event handler.

use super::super::{
    NativeAppError, NativeTerminalApp, NativeWindowMouseInput, NativeWindowMouseInputResult,
};
use crate::mouse::{MouseButton, MouseEventKind};

impl NativeTerminalApp {
    pub(super) fn send_current_mouse_input_and_request_redraw(
        &mut self,
        kind: MouseEventKind,
        button: MouseButton,
    ) -> Result<(), NativeAppError> {
        let result = self.send_current_mouse_input(kind, button)?;
        if result.needs_redraw
            && let Some(window) = &self.window
        {
            window.request_redraw();
        }
        Ok(())
    }

    pub(super) fn send_current_mouse_input(
        &mut self,
        kind: MouseEventKind,
        button: MouseButton,
    ) -> Result<NativeWindowMouseInputResult, NativeAppError> {
        let (Some(position), Some(window)) = (self.cursor_position, self.window.as_ref()) else {
            return Ok(NativeWindowMouseInputResult::default());
        };
        let size = window.inner_size();
        self.runtime
            .send_window_mouse_input_event_result(NativeWindowMouseInput {
                x: position.x,
                y: position.y,
                window_width_px: size.width,
                window_height_px: size.height,
                cell_width_px: self.renderer.config().cell_width_px,
                line_height_px: self.renderer.config().line_height_px,
                surface_padding_px: self.renderer.config().surface_padding_px,
                cell_spacing_px: self.renderer.config().cell_spacing_px,
                kind,
                button,
                modifiers: self.modifiers,
            })
    }
}
