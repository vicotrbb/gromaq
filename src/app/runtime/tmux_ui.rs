use crate::tmux::TmuxManagerSnapshot;

use super::NativeTerminalRuntime;
use crate::app::{TmuxManagerKeyOutcome, TmuxManagerPanelState};

impl<S> NativeTerminalRuntime<S> {
    /// Toggle the native tmux manager panel using a freshly read snapshot.
    pub fn toggle_tmux_manager_panel(&mut self, snapshot: TmuxManagerSnapshot) {
        if self.tmux_manager_panel_is_open() {
            self.tmux_manager_panel = None;
            self.tmux_manager_snapshot = None;
        } else {
            self.tmux_manager_panel = Some(TmuxManagerPanelState::open_for_snapshot(&snapshot));
            self.tmux_manager_snapshot = Some(snapshot);
        }
        self.terminal.invalidate_viewport();
    }

    /// Return whether the native tmux manager panel is open.
    pub fn tmux_manager_panel_is_open(&self) -> bool {
        self.tmux_manager_panel
            .as_ref()
            .is_some_and(TmuxManagerPanelState::is_open)
    }

    /// Let the open tmux manager panel handle a native key before shell input.
    pub fn handle_tmux_manager_key(
        &mut self,
        key: &winit::keyboard::Key,
        modifiers: winit::keyboard::ModifiersState,
    ) -> TmuxManagerKeyOutcome {
        let (Some(snapshot), Some(panel)) = (
            self.tmux_manager_snapshot.as_ref(),
            self.tmux_manager_panel.as_mut(),
        ) else {
            return TmuxManagerKeyOutcome::Ignored;
        };
        let outcome = panel.handle_key(key, modifiers, snapshot);
        if !matches!(outcome, TmuxManagerKeyOutcome::Ignored) {
            self.terminal.invalidate_viewport();
        }
        if !panel.is_open() {
            self.tmux_manager_panel = None;
            self.tmux_manager_snapshot = None;
        }
        outcome
    }
}
