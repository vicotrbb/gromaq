//! Native tmux UI state and frame-only rendering helpers.

mod manager_panel;
mod status_snapshot;

use crate::tmux::shell_quote;
use crate::{CellSnapshot, Color, DirtyRegion, GridSnapshot, Style};

pub use manager_panel::{
    TmuxManagerFocus, TmuxManagerKeyOutcome, TmuxManagerMouseOutcome, TmuxManagerPanelState,
    TmuxWorkspaceUiPreset, apply_tmux_manager_panel,
};

/// High-level tmux status shown in native tmux UI surfaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TmuxStatusKind {
    /// tmux support is disabled for this frame.
    Disabled,
    /// The tmux binary is not available.
    Missing,
    /// tmux is installed but no server is reachable.
    NoServer,
    /// tmux has sessions but the current shell is outside an attached client.
    Detached,
    /// Gromaq is operating with an attached tmux client target.
    Attached,
}

/// Snapshot consumed by native tmux UI rendering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TmuxUiSnapshot {
    /// Current tmux availability/client status.
    pub status: TmuxStatusKind,
    /// Current session name when known.
    pub current_session: Option<String>,
    /// Current window label when known.
    pub current_window: Option<String>,
    /// Visible window labels for the selected session.
    pub visible_windows: Vec<String>,
    /// Pane count for the selected window or session.
    pub pane_count: Option<usize>,
    /// Active pane id when known.
    pub active_pane_id: Option<String>,
    /// Active pane command when known.
    pub active_pane_command: Option<String>,
    /// Last or pending non-destructive action feedback.
    pub pending_feedback: Option<String>,
    /// Inline confirmation prompt for destructive actions.
    pub confirmation_feedback: Option<String>,
}

impl TmuxUiSnapshot {
    fn status_label(&self) -> &'static str {
        match self.status {
            TmuxStatusKind::Disabled => "disabled",
            TmuxStatusKind::Missing => "missing",
            TmuxStatusKind::NoServer => "no server",
            TmuxStatusKind::Detached => "detached",
            TmuxStatusKind::Attached => "attached",
        }
    }

    fn status_line(&self, cols: usize) -> String {
        let full = self.status_line_with_current_window(true);
        if full.chars().count() <= cols {
            return full;
        }
        if self.current_window.is_some() && !self.visible_windows.is_empty() {
            let compact = self.status_line_with_current_window(false);
            if compact.chars().count() <= cols {
                return compact;
            }
        }
        full
    }

    fn status_line_with_current_window(&self, include_current_window: bool) -> String {
        let mut parts = vec![format!("tmux: {}", self.status_label())];
        parts.push("manager Cmd/Ctrl+Shift+T".to_owned());
        if let Some(guidance) = self.status_guidance() {
            parts.push(guidance);
        }
        push_optional(&mut parts, self.current_session.as_deref());
        if include_current_window {
            push_optional(&mut parts, self.current_window.as_deref());
        }
        if !self.visible_windows.is_empty() {
            parts.push(format!("windows {}", self.visible_windows.join(" ")));
        }
        if let Some(pane_count) = self.pane_count {
            parts.push(format!("panes {pane_count}"));
        }
        if let Some(pane_id) = self.active_pane_id.as_deref() {
            let mut pane = pane_id.to_owned();
            if let Some(command) = self.active_pane_command.as_deref() {
                pane.push(' ');
                pane.push_str(command);
            }
            parts.push(pane);
        }
        push_optional(&mut parts, self.pending_feedback.as_deref());
        if let Some(confirmation) = self.confirmation_feedback.as_deref() {
            parts.push(format!("confirm: {confirmation}"));
        }
        parts.join(" | ")
    }

    fn status_guidance(&self) -> Option<String> {
        match self.status {
            TmuxStatusKind::Missing => Some("install tmux".to_owned()),
            TmuxStatusKind::NoServer => Some("start session".to_owned()),
            TmuxStatusKind::Detached => Some(
                self.current_session
                    .as_deref()
                    .filter(|session| !session.is_empty())
                    .map(|session| format!("attach session {}", shell_quote(session)))
                    .unwrap_or_else(|| "attach session".to_owned()),
            ),
            TmuxStatusKind::Disabled | TmuxStatusKind::Attached => None,
        }
    }
}

/// Apply a persistent tmux status strip to a cloned grid snapshot.
pub fn apply_tmux_status_strip(
    grid: &mut GridSnapshot,
    snapshot: &TmuxUiSnapshot,
) -> Option<DirtyRegion> {
    if snapshot.status == TmuxStatusKind::Disabled || grid.cols == 0 || grid.rows == 0 {
        return None;
    }
    let row = (0..grid.rows)
        .rev()
        .find(|row| grid.line_text(*row).is_empty())
        .unwrap_or(grid.rows - 1);
    let cols = usize::from(grid.cols);
    let line = fit_status_line(&snapshot.status_line(cols), cols);
    let style = tmux_status_strip_style();
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
    Some(DirtyRegion {
        row,
        col: 0,
        rows: 1,
        cols: grid.cols,
    })
}

fn push_optional(parts: &mut Vec<String>, value: Option<&str>) {
    if let Some(value) = value
        && !value.is_empty()
    {
        parts.push(value.to_owned());
    }
}

fn fit_status_line(line: &str, cols: usize) -> String {
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

fn tmux_status_strip_style() -> Style {
    Style {
        foreground: Color::Ansi(14),
        background: Color::Ansi(0),
        bold: true,
        ..Style::default()
    }
}
