use std::time::Instant;

use tracing::trace;
use winit::keyboard::{Key, ModifiersState, PhysicalKey};

use crate::app::native_input::{
    NativeMouseGridMapper, NativeRenderedGridMetrics, NativeWindowMouseInput,
    ScrollbackKeyDirection, clamp_u32_to_u16, native_scrollback_key_direction,
};
use crate::app::perf::add_usize_counter;
use crate::app::{NativeAppError, NativePtySessionIo};
use crate::clipboard::HostClipboard;
use crate::mouse::{MouseButton, MouseEvent, MouseEventKind};

use super::NativeTerminalRuntime;

impl<S> NativeTerminalRuntime<S>
where
    S: NativePtySessionIo,
{
    /// Write encoded terminal input bytes to the PTY session.
    pub fn send_pty_input(&mut self, bytes: &[u8]) -> Result<(), NativeAppError> {
        if bytes.is_empty() {
            return Ok(());
        }
        let Some(session) = self.shell_session.as_mut() else {
            return Ok(());
        };
        session
            .write_input(bytes)
            .map_err(|error| NativeAppError::Runtime(error.to_string()))?;
        self.perf.pty_input_writes += 1;
        add_usize_counter(&mut self.perf.pty_input_bytes, bytes.len());
        trace!(
            input_bytes = bytes.len(),
            input_writes = self.perf.pty_input_writes,
            total_input_bytes = self.perf.pty_input_bytes,
            "wrote native PTY input"
        );
        if self.pending_input_to_render_started.is_none() {
            self.pending_input_to_render_started = Some(Instant::now());
        }
        Ok(())
    }

    /// Encode a native logical key and write it to the PTY when it maps to terminal input.
    pub fn send_winit_key_input(
        &mut self,
        key: &Key,
        modifiers: ModifiersState,
    ) -> Result<bool, NativeAppError> {
        self.send_winit_key_event_input(key, None, modifiers)
    }

    /// Encode a native key event and write it to the PTY when it maps to terminal input.
    pub fn send_winit_key_event_input(
        &mut self,
        key: &Key,
        physical_key: Option<PhysicalKey>,
        modifiers: ModifiersState,
    ) -> Result<bool, NativeAppError> {
        if let Some(direction) = native_scrollback_key_direction(key, modifiers) {
            let alternate_screen_active = self.terminal.is_alternate_screen_active();
            let rows = self.terminal.dump_grid().rows.saturating_sub(1).max(1);
            if match direction {
                ScrollbackKeyDirection::Up => self.terminal.scroll_display_up(rows),
                ScrollbackKeyDirection::Down => self.terminal.scroll_display_down(rows),
            } {
                return Ok(true);
            }
            if !alternate_screen_active {
                return Ok(false);
            }
        }

        let Some(bytes) = self
            .terminal
            .encode_winit_key_event_input(key, physical_key, modifiers)
        else {
            return Ok(false);
        };
        let had_session = self.shell_session.is_some();
        self.send_pty_input(&bytes)?;
        if had_session {
            self.perf.native_key_inputs += 1;
        }
        Ok(true)
    }

    /// Encode a terminal mouse event and write it to the PTY when reporting is enabled.
    pub fn send_mouse_input(&mut self, event: MouseEvent) -> Result<bool, NativeAppError> {
        let Some(bytes) = self.terminal.encode_mouse_event(event) else {
            return Ok(false);
        };
        let had_session = self.shell_session.is_some();
        self.send_pty_input(&bytes)?;
        if had_session {
            self.perf.mouse_inputs += 1;
        }
        Ok(true)
    }

    /// Encode a terminal focus event and write it to the PTY when reporting is enabled.
    pub fn send_focus_event(&mut self, focused: bool) -> Result<bool, NativeAppError> {
        let Some(bytes) = self.terminal.encode_focus_event(focused) else {
            return Ok(false);
        };
        let had_session = self.shell_session.is_some();
        self.send_pty_input(&bytes)?;
        if had_session {
            self.perf.focus_inputs += 1;
        }
        Ok(true)
    }

    /// Map a native window mouse position to a terminal event and write its report to the PTY.
    pub fn send_window_mouse_input(
        &mut self,
        x: f64,
        y: f64,
        window_width_px: u32,
        window_height_px: u32,
        kind: MouseEventKind,
        button: MouseButton,
    ) -> Result<bool, NativeAppError> {
        self.send_window_mouse_input_event(NativeWindowMouseInput {
            x,
            y,
            window_width_px,
            window_height_px,
            cell_width_px: inferred_cell_size_px(window_width_px, self.terminal.dump_grid().cols),
            line_height_px: inferred_cell_size_px(window_height_px, self.terminal.dump_grid().rows),
            surface_padding_px: 0,
            cell_spacing_px: 0,
            kind,
            button,
            modifiers: ModifiersState::empty(),
        })
    }

    /// Map native window mouse input to a terminal event and write its report.
    pub fn send_window_mouse_input_event(
        &mut self,
        input: NativeWindowMouseInput,
    ) -> Result<bool, NativeAppError> {
        let grid = self.terminal.dump_grid();
        let Some(mapper) = NativeMouseGridMapper::new(
            input.window_width_px,
            input.window_height_px,
            NativeRenderedGridMetrics {
                cell_width_px: input.cell_width_px,
                line_height_px: input.line_height_px,
                surface_padding_px: input.surface_padding_px,
                cell_spacing_px: input.cell_spacing_px,
                cols: grid.cols,
                rows: grid.rows,
            },
        ) else {
            return Ok(false);
        };
        let Some(event) = mapper.mouse_event_at_with_modifiers(
            input.x,
            input.y,
            input.kind,
            input.button,
            input.modifiers,
        ) else {
            return Ok(false);
        };
        if self.send_mouse_input(event)? {
            return Ok(true);
        }
        Ok(match (input.kind, input.button) {
            (MouseEventKind::Press, MouseButton::WheelUp) => self.terminal.scroll_display_up(1),
            (MouseEventKind::Press, MouseButton::WheelDown) => self.terminal.scroll_display_down(1),
            _ => false,
        })
    }

    /// Encode pasted text according to terminal mode and write it to the PTY.
    pub fn send_paste_text(&mut self, text: &str) -> Result<(), NativeAppError> {
        let bytes = self.terminal.encode_paste_text(text);
        let had_session = self.shell_session.is_some();
        self.send_pty_input(&bytes)?;
        if had_session {
            add_usize_counter(&mut self.perf.paste_bytes, text.len());
        }
        Ok(())
    }

    /// Read text from a host clipboard and write it to the PTY as a terminal paste.
    pub fn send_clipboard_paste<C>(&mut self, clipboard: &C) -> Result<bool, NativeAppError>
    where
        C: HostClipboard,
    {
        if self.shell_session.is_none() {
            return Ok(false);
        }
        let Some(text) = clipboard.read_text().filter(|text| !text.is_empty()) else {
            return Ok(false);
        };
        self.send_paste_text(&text)?;
        self.perf.clipboard_pastes += 1;
        Ok(true)
    }

    /// Write committed platform text input to the PTY as typed UTF-8 text.
    pub fn send_committed_text(&mut self, text: &str) -> Result<(), NativeAppError> {
        let had_session = self.shell_session.is_some();
        self.send_pty_input(text.as_bytes())?;
        if had_session {
            add_usize_counter(&mut self.perf.committed_text_bytes, text.len());
        }
        Ok(())
    }
}

fn inferred_cell_size_px(window_px: u32, cells: u16) -> u16 {
    if cells == 0 {
        return 0;
    }
    clamp_u32_to_u16((window_px / u32::from(cells)).max(1))
}
