//! Event-handler action helpers for the native terminal app.

use winit::event::Ime;
use winit::event_loop::ActiveEventLoop;

use super::super::{NativeAppAction, NativeGlyphFrameError, NativeTerminalApp};
use crate::clipboard::NativeClipboard;
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
        if let Err(error) = self.runtime.pump_pty_output() {
            self.startup_error = Some(error.to_string());
            event_loop.exit();
            return;
        }
        let mut frame_presented = false;
        match self.present_redraw_frame() {
            Ok(report) => {
                frame_presented = report.glyph_frame_presented || report.clear_presented;
                self.lifecycle.record_glyph_frame_presentation(report);
            }
            Err(error) => match error {
                NativeGlyphFrameError::Surface(
                    surface_error @ (SurfaceFrameError::Timeout | SurfaceFrameError::Occluded),
                ) => {
                    self.lifecycle.record_surface_frame_skip(surface_error);
                }
                NativeGlyphFrameError::Surface(
                    SurfaceFrameError::Outdated
                    | SurfaceFrameError::Lost
                    | SurfaceFrameError::Validation
                    | SurfaceFrameError::InvalidFrame(_),
                )
                | NativeGlyphFrameError::Font(_)
                | NativeGlyphFrameError::Renderer(_)
                | NativeGlyphFrameError::Snapshot(_) => {
                    self.startup_error = Some(error.to_string());
                    event_loop.exit();
                }
            },
        }
        match self.lifecycle.on_redraw_attempt_finished(frame_presented) {
            NativeAppAction::RequestRedraw => {
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            NativeAppAction::Exit => event_loop.exit(),
            NativeAppAction::None | NativeAppAction::CreateWindow => {}
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
        } else if matches!(
            super::super::native_tmux_assist_action(
                &logical_key,
                Some(physical_key),
                self.modifiers
            ),
            Some(super::super::NativeTmuxAssistAction::ToggleManager)
        ) {
            self.runtime.toggle_tmux_manager_panel_with_workspaces(
                read_tmux_manager_snapshot(),
                self.lifecycle.config().tmux_workspaces.clone(),
            );
            if let Some(window) = &self.window {
                window.request_redraw();
            }
            Ok(())
        } else {
            let tmux_outcome = self
                .runtime
                .handle_tmux_manager_key(&logical_key, self.modifiers);
            if !matches!(tmux_outcome, super::super::TmuxManagerKeyOutcome::Ignored) {
                self.runtime.dispatch_tmux_manager_action(
                    tmux_outcome,
                    &crate::tmux::SystemTmuxCommandRunner,
                );
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
                Ok(())
            } else {
                self.runtime
                    .send_native_key_event_input(
                        &logical_key,
                        Some(physical_key),
                        self.modifiers,
                        self.ime_preedit_active,
                    )
                    .map(|_| ())
            }
        };
        if let Err(error) = result {
            self.startup_error = Some(error.to_string());
            event_loop.exit();
        }
    }

    pub(super) fn handle_ime_event(&mut self, event_loop: &ActiveEventLoop, event: Ime) {
        match event {
            Ime::Enabled => {
                self.ime_preedit_active = false;
            }
            Ime::Preedit(text, _) => {
                self.ime_preedit_active = !text.is_empty();
            }
            Ime::Commit(text) => {
                self.ime_preedit_active = false;
                if let Err(error) = self.runtime.send_committed_text(&text) {
                    self.startup_error = Some(error.to_string());
                    event_loop.exit();
                }
            }
            Ime::Disabled => {
                self.ime_preedit_active = false;
            }
        }
    }
}

fn read_tmux_manager_snapshot() -> crate::tmux::TmuxManagerSnapshot {
    crate::tmux::TmuxManager::new(crate::tmux::SystemTmuxCommandRunner)
        .snapshot()
        .unwrap_or(crate::tmux::TmuxManagerSnapshot {
            state: crate::tmux::TmuxState::default(),
            current: None,
        })
}
