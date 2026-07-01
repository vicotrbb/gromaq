//! Native tmux manager panel mouse handling.

use super::super::enter_action::enter_action_id;
use super::super::hints::{action_choice_label, action_hint, enter_action_label};
use super::super::selection::{selected_panes, selected_windows, window_label};
use super::super::state::{TmuxManagerFocus, TmuxManagerPanelState};
use super::super::workspaces::workspace_summary;
use crate::tmux::{TmuxAction, TmuxManagerSnapshot, TmuxPane};
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
    pub fn handle_mouse_event(
        &mut self,
        event: MouseEvent,
        snapshot: &TmuxManagerSnapshot,
    ) -> TmuxManagerMouseOutcome {
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
        self.select_clicked_item(focus, event.col, snapshot);
        TmuxManagerMouseOutcome::Consumed
    }

    fn select_clicked_item(
        &mut self,
        focus: TmuxManagerFocus,
        col: u16,
        snapshot: &TmuxManagerSnapshot,
    ) {
        match focus {
            TmuxManagerFocus::Sessions => {
                if let Some(index) = hit_label_index(
                    col,
                    "Sessions ".len(),
                    snapshot
                        .state
                        .sessions
                        .iter()
                        .enumerate()
                        .map(|(index, session)| {
                            selected_label(&session.name, index == self.selected_session)
                        }),
                ) {
                    self.selected_session = index;
                    self.selected_window = 0;
                    self.selected_pane = 0;
                }
            }
            TmuxManagerFocus::Windows => {
                if let Some(index) = hit_label_index(
                    col,
                    "Windows ".len(),
                    selected_windows(snapshot, self.selected_session)
                        .iter()
                        .enumerate()
                        .map(|(index, window)| {
                            selected_label(&window_label(window), index == self.selected_window)
                        }),
                ) {
                    self.selected_window = index;
                    self.selected_pane = 0;
                }
            }
            TmuxManagerFocus::Panes => {
                if let Some(index) = hit_label_index(
                    col,
                    "Panes ".len(),
                    selected_panes(snapshot, self.selected_session, self.selected_window)
                        .iter()
                        .enumerate()
                        .map(|(index, pane)| pane_label(pane, index == self.selected_pane)),
                ) {
                    self.selected_pane = index;
                }
            }
            TmuxManagerFocus::Actions => {
                if let Some(index) = hit_label_index(
                    col,
                    action_choices_start_col(snapshot, self),
                    super::PANEL_ACTIONS.iter().filter_map(|action_id| {
                        TmuxAction::by_id(*action_id).map(|action| {
                            action_choice_label(
                                action,
                                action.id
                                    == super::PANEL_ACTIONS
                                        .get(self.selected_action)
                                        .copied()
                                        .unwrap_or(super::PANEL_ACTIONS[0]),
                                snapshot,
                                self,
                            )
                        })
                    }),
                ) {
                    self.selected_action = index;
                }
            }
            TmuxManagerFocus::Workspaces => {
                if let Some(index) = hit_label_index(
                    col,
                    "Workspaces ".len(),
                    self.workspace_presets
                        .iter()
                        .enumerate()
                        .map(|(index, preset)| {
                            workspace_summary(preset, index == self.selected_workspace)
                        }),
                ) {
                    self.selected_workspace = index;
                }
            }
        }
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

fn hit_label_index(
    col: u16,
    prefix_width: usize,
    labels: impl Iterator<Item = String>,
) -> Option<usize> {
    let col = usize::from(col);
    let mut start = prefix_width;
    for (index, label) in labels.enumerate() {
        let end = start + label.chars().count();
        if col >= start && col < end {
            return Some(index);
        }
        start = end.saturating_add(1);
    }
    None
}

fn action_choices_start_col(
    snapshot: &TmuxManagerSnapshot,
    panel: &TmuxManagerPanelState,
) -> usize {
    let selected_action =
        TmuxAction::by_id(enter_action_id(snapshot, panel)).expect("panel action is registered");
    format!(
        "Actions | Enter {} | {} | ",
        enter_action_label(selected_action, snapshot, panel),
        action_hint(selected_action)
    )
    .chars()
    .count()
}

fn selected_label(label: &str, selected: bool) -> String {
    if selected {
        format!("{label}*")
    } else {
        label.to_owned()
    }
}

fn pane_label(pane: &TmuxPane, selected: bool) -> String {
    let mut command = if pane.title.is_empty() || pane.title == pane.current_command {
        pane.current_command.clone()
    } else {
        format!("{}:{}", pane.title, pane.current_command)
    };
    if selected {
        command.push('*');
    }
    let dimensions = match (pane.width, pane.height) {
        (Some(width), Some(height)) => format!(" {width}x{height}"),
        _ => String::new(),
    };
    format!("{} {}{}", pane.id, command, dimensions)
}
