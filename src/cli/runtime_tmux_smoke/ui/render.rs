//! Render-plan proof helpers for the native tmux UI smoke.

use crate::app::{NativeTerminalRuntime, TmuxStatusKind, TmuxUiSnapshot};
use crate::renderer::WgpuRenderer;
use crate::tmux::TmuxManagerSnapshot;

pub(super) fn render_status_strip(
    runtime: &mut NativeTerminalRuntime<()>,
    renderer: &mut WgpuRenderer,
    snapshot: &TmuxManagerSnapshot,
) -> bool {
    let strip = TmuxUiSnapshot {
        status: tmux_status(snapshot),
        current_session: snapshot
            .current
            .as_ref()
            .map(|current| current.session_name.clone()),
        current_window: snapshot
            .current
            .as_ref()
            .map(|current| current.window_index.to_string()),
        visible_windows: snapshot
            .state
            .windows
            .iter()
            .map(|window| format!("{}:{}", window.index, window.name))
            .collect(),
        pane_count: Some(snapshot.state.panes.len()),
        active_pane_id: snapshot
            .current
            .as_ref()
            .map(|current| current.pane_id.clone()),
        active_pane_command: None,
        pending_feedback: None,
        confirmation_feedback: None,
    };
    runtime
        .render_terminal_frame_with_tmux_status_strip(renderer, &strip)
        .is_ok_and(|rendered| rendered)
        && last_plan_text(renderer).contains("tmux:")
}

pub(super) fn render_manager_panel(
    runtime: &mut NativeTerminalRuntime<()>,
    renderer: &mut WgpuRenderer,
) -> bool {
    runtime.invalidate_terminal_frame();
    runtime
        .render_terminal_frame_with_status_overlay(renderer, None)
        .is_ok_and(|rendered| rendered)
        && last_plan_text(renderer).contains("tmuxmanager")
}

fn tmux_status(snapshot: &TmuxManagerSnapshot) -> TmuxStatusKind {
    if snapshot.state.sessions.is_empty() {
        TmuxStatusKind::NoServer
    } else {
        TmuxStatusKind::Detached
    }
}

fn last_plan_text(renderer: &WgpuRenderer) -> String {
    renderer
        .last_plan()
        .map(|plan| {
            plan.glyphs
                .iter()
                .map(|glyph| glyph.text.as_str())
                .collect::<String>()
        })
        .unwrap_or_default()
}
