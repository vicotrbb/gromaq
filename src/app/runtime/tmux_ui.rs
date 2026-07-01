mod action_dispatch;

use crate::tmux::TmuxManagerSnapshot;

use super::NativeTerminalRuntime;
use crate::app::{
    TmuxManagerKeyOutcome, TmuxManagerMouseOutcome, TmuxManagerPanelState, TmuxUiSnapshot,
    TmuxWorkspaceUiPreset,
};
use crate::mouse::MouseEvent;

impl<S> NativeTerminalRuntime<S> {
    /// Retain a tmux status snapshot for the normal native render path.
    pub fn set_tmux_status_snapshot(&mut self, snapshot: TmuxUiSnapshot) {
        self.tmux_status_snapshot = Some(snapshot);
        self.terminal.invalidate_viewport();
    }

    /// Configure whether retained tmux status is rendered as a persistent strip.
    pub fn set_tmux_status_strip_enabled(&mut self, enabled: bool) {
        self.tmux_status_strip_enabled = enabled;
        self.terminal.invalidate_viewport();
    }

    /// Clear the retained tmux status snapshot from the normal native render path.
    pub fn clear_tmux_status_snapshot(&mut self) {
        self.tmux_status_snapshot = None;
        self.terminal.invalidate_viewport();
    }

    /// Return whether the last rendered frame applied the native tmux status strip.
    pub fn last_rendered_tmux_status_strip(&self) -> bool {
        self.last_rendered_tmux_status_strip
    }

    /// Return whether the last rendered frame applied the native tmux manager panel.
    pub fn last_rendered_tmux_manager_panel(&self) -> bool {
        self.last_rendered_tmux_manager_panel
    }

    /// Let the rendered tmux manager panel handle a grid-relative mouse event.
    pub fn handle_tmux_manager_mouse_event(
        &mut self,
        event: MouseEvent,
    ) -> TmuxManagerMouseOutcome {
        let Some(region) = self.last_rendered_tmux_manager_region else {
            return TmuxManagerMouseOutcome::Ignored;
        };
        if event.row < region.row
            || event.row >= region.row.saturating_add(region.rows)
            || event.col < region.col
            || event.col >= region.col.saturating_add(region.cols)
        {
            return TmuxManagerMouseOutcome::Ignored;
        }
        let (Some(snapshot), Some(panel)) = (
            self.tmux_manager_snapshot.as_ref(),
            self.tmux_manager_panel.as_mut(),
        ) else {
            return TmuxManagerMouseOutcome::Ignored;
        };
        let mut panel_event = event;
        panel_event.row = panel_event.row.saturating_sub(region.row);
        panel_event.col = panel_event.col.saturating_sub(region.col);
        let outcome = panel.handle_mouse_event(panel_event, snapshot);
        if matches!(outcome, TmuxManagerMouseOutcome::Consumed) {
            self.sync_tmux_status_feedback_from_panel();
            self.terminal.invalidate_viewport();
        }
        outcome
    }

    /// Toggle the native tmux manager panel using a freshly read snapshot.
    pub fn toggle_tmux_manager_panel(&mut self, snapshot: TmuxManagerSnapshot) {
        self.toggle_tmux_manager_panel_with_workspaces(snapshot, Vec::new());
    }

    /// Toggle the native tmux manager panel with configured workspace presets.
    pub fn toggle_tmux_manager_panel_with_workspaces(
        &mut self,
        snapshot: TmuxManagerSnapshot,
        workspace_presets: Vec<TmuxWorkspaceUiPreset>,
    ) {
        if self.tmux_manager_panel_is_open() {
            self.tmux_manager_panel = None;
            self.tmux_manager_snapshot = None;
        } else {
            self.tmux_status_snapshot = Some(TmuxUiSnapshot::from_manager_snapshot(&snapshot));
            self.tmux_manager_panel =
                Some(TmuxManagerPanelState::open_for_snapshot_with_workspaces(
                    &snapshot,
                    workspace_presets,
                ));
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

    /// Refresh the open tmux manager panel with a newly read snapshot.
    pub fn refresh_tmux_manager_panel(&mut self, snapshot: TmuxManagerSnapshot) {
        let Some((workspace_presets, action_feedback, pending_feedback, confirmation_feedback)) =
            self.tmux_manager_panel.as_ref().and_then(|panel| {
                if !panel.is_open() {
                    return None;
                }
                let last_feedback = panel
                    .last_action_feedback()
                    .or_else(|| panel.workspace_feedback())
                    .map(str::to_owned);
                Some((
                    panel.workspace_presets().to_vec(),
                    last_feedback.clone(),
                    last_feedback.or_else(|| panel.pending_action().map(str::to_owned)),
                    panel.confirmation_message().map(str::to_owned),
                ))
            })
        else {
            return;
        };
        let mut panel =
            TmuxManagerPanelState::open_for_snapshot_with_workspaces(&snapshot, workspace_presets);
        if let Some(feedback) = action_feedback {
            panel.record_action_feedback(feedback);
        }
        self.tmux_manager_panel = Some(panel);
        self.tmux_manager_snapshot = Some(snapshot);
        if let Some(snapshot) = self.tmux_manager_snapshot.as_ref() {
            let mut status = TmuxUiSnapshot::from_manager_snapshot(snapshot);
            status.pending_feedback = pending_feedback;
            status.confirmation_feedback = confirmation_feedback;
            self.tmux_status_snapshot = Some(status);
        }
        self.terminal.invalidate_viewport();
    }

    /// Let the open tmux manager panel handle a native key before shell input.
    pub fn handle_tmux_manager_key(
        &mut self,
        key: &winit::keyboard::Key,
        modifiers: winit::keyboard::ModifiersState,
    ) -> TmuxManagerKeyOutcome {
        let (outcome, panel_open) = {
            let (Some(snapshot), Some(panel)) = (
                self.tmux_manager_snapshot.as_ref(),
                self.tmux_manager_panel.as_mut(),
            ) else {
                return TmuxManagerKeyOutcome::Ignored;
            };
            let outcome = panel.handle_key(key, modifiers, snapshot);
            (outcome, panel.is_open())
        };
        if !matches!(outcome, TmuxManagerKeyOutcome::Ignored) {
            self.sync_tmux_status_feedback_from_panel();
            self.terminal.invalidate_viewport();
        }
        if !panel_open {
            self.tmux_manager_panel = None;
            self.tmux_manager_snapshot = None;
        }
        outcome
    }

    pub(super) fn sync_tmux_status_feedback_from_panel(&mut self) {
        let (Some(panel), Some(status)) = (
            self.tmux_manager_panel.as_ref(),
            self.tmux_status_snapshot.as_mut(),
        ) else {
            return;
        };
        status.pending_feedback = panel
            .last_action_feedback()
            .or_else(|| panel.workspace_feedback())
            .or_else(|| panel.pending_action())
            .map(str::to_owned);
        status.confirmation_feedback = panel.confirmation_message().map(str::to_owned);
    }
}
