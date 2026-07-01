//! Native tmux manager panel mouse handling.

use super::super::state::{TmuxManagerFocus, TmuxManagerPanelState};
use crate::{MouseButton, MouseEvent, MouseEventKind};

/// Result of handling a mouse event while the tmux manager panel may be open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TmuxManagerMouseOutcome {
    /// The panel did not use the mouse event.
    Ignored,
    /// The panel consumed the mouse event without requesting an action.
    Consumed,
}

impl TmuxManagerPanelState {
    /// Handle a mouse event relative to the rendered manager panel rows.
    pub fn handle_mouse_event(&mut self, event: MouseEvent) -> TmuxManagerMouseOutcome {
        if !self.open
            || event.kind != MouseEventKind::Press
            || event.button != MouseButton::Left
            || !event.modifiers.is_empty()
        {
            return TmuxManagerMouseOutcome::Ignored;
        }
        let Some(focus) = panel_row_focus(self, event.row) else {
            return TmuxManagerMouseOutcome::Ignored;
        };
        self.focus = focus;
        TmuxManagerMouseOutcome::Consumed
    }
}

fn panel_row_focus(panel: &TmuxManagerPanelState, row: u16) -> Option<TmuxManagerFocus> {
    match row {
        1 => Some(TmuxManagerFocus::Sessions),
        2 => Some(TmuxManagerFocus::Windows),
        3 => Some(TmuxManagerFocus::Panes),
        4 if panel.workspace_presets.is_empty() => Some(TmuxManagerFocus::Actions),
        4 => Some(TmuxManagerFocus::Workspaces),
        5 if !panel.workspace_presets.is_empty() => Some(TmuxManagerFocus::Actions),
        _ => None,
    }
}
