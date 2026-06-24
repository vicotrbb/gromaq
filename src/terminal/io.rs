//! Terminal parser input and application input/output encoders.

use winit::keyboard::{Key, ModifiersState, PhysicalKey};

use crate::error::Result;
use crate::input::encode_winit_key_with_terminal_modes;
use crate::mouse::MouseEvent;

use super::Terminal;

impl Terminal {
    /// Feed UTF-8 text and escape sequences into the terminal parser.
    pub fn write_str(&mut self, input: &str) -> Result<()> {
        self.write_bytes(input.as_bytes())
    }

    /// Feed raw terminal bytes and escape sequences into the terminal parser.
    pub fn write_bytes(&mut self, input: &[u8]) -> Result<()> {
        if !input.is_empty() {
            self.scroll_display_to_bottom();
        }
        let mut parser = std::mem::take(&mut self.parser);
        parser.advance(self, input);
        self.parser = parser;
        self.flush_dirty_run();
        self.perf.parsed_bytes += input.len() as u64;
        Ok(())
    }

    /// Encode a mouse event for the running application when reporting is enabled.
    pub fn encode_mouse_event(&self, event: MouseEvent) -> Option<Vec<u8>> {
        self.mouse.encode(event)
    }

    /// Encode a native logical key according to terminal input modes.
    pub fn encode_winit_key_input(&self, key: &Key, modifiers: ModifiersState) -> Option<Vec<u8>> {
        self.encode_winit_key_event_input(key, None, modifiers)
    }

    /// Encode a native key event according to terminal input modes.
    pub fn encode_winit_key_event_input(
        &self,
        key: &Key,
        physical_key: Option<PhysicalKey>,
        modifiers: ModifiersState,
    ) -> Option<Vec<u8>> {
        encode_winit_key_with_terminal_modes(
            key,
            physical_key,
            modifiers,
            self.application_cursor_keys,
            self.application_keypad,
        )
    }

    /// Encode a terminal focus event when focus reporting mode is enabled.
    pub fn encode_focus_event(&self, focused: bool) -> Option<Vec<u8>> {
        if !self.focus_event_reporting {
            return None;
        }
        Some(if focused {
            b"\x1b[I".to_vec()
        } else {
            b"\x1b[O".to_vec()
        })
    }

    /// Return the current window title set by OSC 0 or OSC 2.
    pub fn dump_title(&self) -> Option<String> {
        self.title.clone()
    }

    /// Return clipboard text accepted from terminal control sequences.
    pub fn dump_clipboard_text(&self) -> Option<String> {
        self.clipboard_text.clone()
    }

    /// Encode pasted text for the running application.
    pub fn encode_paste_text(&self, text: &str) -> Vec<u8> {
        if self.bracketed_paste {
            let mut bytes = Vec::with_capacity(text.len() + b"\x1b[200~\x1b[201~".len());
            bytes.extend_from_slice(b"\x1b[200~");
            bytes.extend_from_slice(text.as_bytes());
            bytes.extend_from_slice(b"\x1b[201~");
            bytes
        } else {
            text.as_bytes().to_vec()
        }
    }

    /// Drain terminal-generated response bytes that should be written back to the PTY.
    pub fn take_pending_response_bytes(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.pending_response_bytes)
    }
}
