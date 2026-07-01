//! Native tmux manager panel rendering.

use super::enter_action::enter_action_id;
use super::hints::{action_choice_label, action_hint, enter_action_label, hint_row};
use super::input::panel_actions;
use super::rows::{pane_row, session_row, window_row};
use super::state::{TmuxManagerFocus, TmuxManagerPanelState};
use super::target::current_target_label;
use super::workspaces::workspace_row;
use crate::tmux::{TmuxAction, TmuxManagerSnapshot, TmuxManagerStatus};
use crate::{CellSnapshot, Color, DirtyRegion, GridSnapshot, Style};

/// Apply a compact tmux manager panel to a cloned grid snapshot.
pub fn apply_tmux_manager_panel(
    grid: &mut GridSnapshot,
    snapshot: &TmuxManagerSnapshot,
    panel: &TmuxManagerPanelState,
) -> Option<DirtyRegion> {
    if !panel.is_open() || grid.cols == 0 || grid.rows == 0 {
        return None;
    }
    let lines = panel_lines(snapshot, panel);
    let start_row = panel_start_row(grid, lines.len());
    let available_rows = usize::from(grid.rows.saturating_sub(start_row));
    let rows = lines.len().min(available_rows);
    for (offset, line) in lines.into_iter().take(rows).enumerate() {
        write_panel_line(grid, start_row + u16::try_from(offset).ok()?, &line);
    }
    Some(DirtyRegion {
        row: start_row,
        col: 0,
        rows: u16::try_from(rows).ok()?,
        cols: grid.cols,
    })
}

fn panel_lines(snapshot: &TmuxManagerSnapshot, panel: &TmuxManagerPanelState) -> Vec<String> {
    let target = current_target_label(snapshot);
    let mut lines = vec![
        format!(
            "tmux manager | status {} | focus {} | target {target}",
            status_label(snapshot),
            focus_label(panel.focus)
        ),
        format!("Sessions {}", session_row(snapshot, panel)),
        format!("Windows {}", window_row(snapshot, panel)),
        format!("Panes {}", pane_row(snapshot, panel)),
    ];
    if let Some(workspace_row) = workspace_row(panel) {
        lines.push(workspace_row);
    }
    lines.push(action_row(snapshot, panel));
    lines.push(
        panel
            .action_input_prompt()
            .or_else(|| panel.confirmation.clone())
            .or_else(|| panel.workspace_feedback.clone())
            .or_else(|| panel.last_action_feedback.clone())
            .unwrap_or_else(|| hint_row(snapshot)),
    );
    lines
}

fn status_label(snapshot: &TmuxManagerSnapshot) -> &'static str {
    match snapshot.status {
        TmuxManagerStatus::Missing => "missing",
        TmuxManagerStatus::NoServer => "no server",
        TmuxManagerStatus::Available if snapshot.current.is_some() => "attached",
        TmuxManagerStatus::Available if snapshot.state.sessions.is_empty() => "no server",
        TmuxManagerStatus::Available => "detached",
    }
}

fn focus_label(focus: TmuxManagerFocus) -> &'static str {
    match focus {
        TmuxManagerFocus::Sessions => "sessions",
        TmuxManagerFocus::Windows => "windows",
        TmuxManagerFocus::Panes => "panes",
        TmuxManagerFocus::Workspaces => "workspaces",
        TmuxManagerFocus::Actions => "actions",
    }
}

fn action_row(snapshot: &TmuxManagerSnapshot, panel: &TmuxManagerPanelState) -> String {
    let selected_action_id = enter_action_id(snapshot, panel);
    let selected_action =
        TmuxAction::by_id(selected_action_id).expect("panel action is registered");
    let actions = panel_actions()
        .iter()
        .enumerate()
        .filter_map(|(index, action_id)| {
            TmuxAction::by_id(*action_id).map(|action| {
                action_choice_label(action, index == panel.selected_action, snapshot, panel)
            })
        })
        .collect::<Vec<_>>()
        .join(" ");
    format!(
        "Actions | Enter {} | {} | {actions} | Esc close",
        enter_action_label(selected_action, snapshot, panel),
        action_hint(selected_action)
    )
}

fn panel_start_row(grid: &GridSnapshot, panel_rows: usize) -> u16 {
    let row = (0..grid.rows)
        .find(|row| grid.line_text(*row).is_empty())
        .unwrap_or(0);
    let available_rows = usize::from(grid.rows.saturating_sub(row));
    if available_rows >= panel_rows { row } else { 0 }
}

fn write_panel_line(grid: &mut GridSnapshot, row: u16, line: &str) {
    let line = fit_panel_line(line, usize::from(grid.cols));
    let style = panel_style();
    for col in 0..grid.cols {
        let text = line
            .chars()
            .nth(usize::from(col))
            .map(|ch| ch.to_string())
            .unwrap_or_else(|| " ".to_owned());
        let index = usize::from(row) * usize::from(grid.cols) + usize::from(col);
        grid.cells[index] = CellSnapshot {
            text,
            style,
            hyperlink_id: 0,
            is_wide_leading: false,
            is_wide_trailing: false,
        };
    }
}

fn fit_panel_line(line: &str, cols: usize) -> String {
    let width = line.chars().count();
    if width <= cols {
        return line.to_owned();
    }
    if cols <= 3 {
        return ".".repeat(cols);
    }
    let mut output = line.chars().take(cols - 3).collect::<String>();
    output.push_str("...");
    output
}

fn panel_style() -> Style {
    Style {
        foreground: Color::Ansi(14),
        background: Color::Ansi(0),
        ..Style::default()
    }
}
