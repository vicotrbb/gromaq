//! Native tmux manager panel rendering.

use super::enter_action::enter_action_id;
use super::hints::{action_hint, hint_row};
use super::input::panel_actions;
use super::selection::{selected_panes, selected_windows, window_label};
use super::state::{TmuxManagerFocus, TmuxManagerPanelState};
use super::workspaces::workspace_row;
use crate::tmux::{TmuxAction, TmuxManagerSnapshot, TmuxPane};
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
    let start_row = first_blank_row(grid)?;
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
    let target = snapshot
        .current
        .as_ref()
        .map(|current| {
            format!(
                "{}:{}:{}",
                current.session_name, current.window_index, current.pane_id
            )
        })
        .unwrap_or_else(|| "none".to_owned());
    let mut lines = vec![
        format!(
            "tmux manager | focus {} | target {target}",
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

fn focus_label(focus: TmuxManagerFocus) -> &'static str {
    match focus {
        TmuxManagerFocus::Sessions => "sessions",
        TmuxManagerFocus::Windows => "windows",
        TmuxManagerFocus::Panes => "panes",
        TmuxManagerFocus::Workspaces => "workspaces",
        TmuxManagerFocus::Actions => "actions",
    }
}

fn session_row(snapshot: &TmuxManagerSnapshot, panel: &TmuxManagerPanelState) -> String {
    if snapshot.state.sessions.is_empty() {
        return "none".to_owned();
    }
    snapshot
        .state
        .sessions
        .iter()
        .enumerate()
        .map(|(index, session)| selected_label(&session.name, index == panel.selected_session))
        .collect::<Vec<_>>()
        .join(" ")
}

fn window_row(snapshot: &TmuxManagerSnapshot, panel: &TmuxManagerPanelState) -> String {
    let windows = selected_windows(snapshot, panel.selected_session);
    if windows.is_empty() {
        return "none".to_owned();
    }
    windows
        .iter()
        .enumerate()
        .map(|(index, window)| {
            selected_label(&window_label(window), index == panel.selected_window)
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn pane_row(snapshot: &TmuxManagerSnapshot, panel: &TmuxManagerPanelState) -> String {
    let panes = selected_panes(snapshot, panel.selected_session, panel.selected_window);
    if panes.is_empty() {
        return "none".to_owned();
    }
    panes
        .iter()
        .enumerate()
        .map(|(index, pane)| pane_label(pane, index == panel.selected_pane))
        .collect::<Vec<_>>()
        .join(" ")
}

fn action_row(snapshot: &TmuxManagerSnapshot, panel: &TmuxManagerPanelState) -> String {
    let selected_action_id = enter_action_id(snapshot, panel);
    let selected_action =
        TmuxAction::by_id(selected_action_id).expect("panel action is registered");
    let actions = panel_actions()
        .iter()
        .enumerate()
        .filter_map(|(index, action_id)| {
            TmuxAction::by_id(*action_id)
                .map(|action| selected_label(action.stable_id, index == panel.selected_action))
        })
        .collect::<Vec<_>>()
        .join(" ");
    format!(
        "Actions | Enter {} | {} | choices {actions} | Esc close",
        selected_action.stable_id,
        action_hint(selected_action)
    )
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

fn selected_label(label: &str, selected: bool) -> String {
    if selected {
        format!("{label}*")
    } else {
        label.to_owned()
    }
}

fn first_blank_row(grid: &GridSnapshot) -> Option<u16> {
    (0..grid.rows).find(|row| grid.line_text(*row).is_empty())
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
