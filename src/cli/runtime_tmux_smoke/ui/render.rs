//! Render-plan proof helpers for the native tmux UI smoke.

use crate::app::{NativeTerminalRuntimeConfig, TmuxStatusKind, TmuxUiSnapshot};
use crate::renderer::WgpuRenderer;
use crate::tmux::{TmuxManagerSnapshot, TmuxManagerStatus, TmuxSession, TmuxState};

pub(super) fn render_status_strip(
    runtime: &mut super::SmokeRuntime,
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
    runtime: &mut super::SmokeRuntime,
    renderer: &mut WgpuRenderer,
) -> bool {
    runtime.invalidate_terminal_frame();
    runtime
        .render_terminal_frame_with_status_overlay(renderer, None)
        .is_ok_and(|rendered| rendered)
        && last_plan_text(renderer).contains("tmuxmanager")
}

pub(super) fn render_manager_panel_contains(
    runtime: &mut super::SmokeRuntime,
    renderer: &mut WgpuRenderer,
    expected: &str,
) -> bool {
    runtime.invalidate_terminal_frame();
    runtime
        .render_terminal_frame_with_status_overlay(renderer, None)
        .is_ok_and(|rendered| rendered)
        && last_plan_text(renderer).contains(expected)
}

pub(super) fn render_startup_manager_after_shell_prompt(
    renderer: &mut WgpuRenderer,
    snapshot: &TmuxManagerSnapshot,
) -> bool {
    let Ok(mut runtime) = super::SmokeRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 69,
        terminal_rows: 17,
        ..NativeTerminalRuntimeConfig::default()
    }) else {
        return false;
    };
    if runtime.write_startup_text(&full_prompt_grid()).is_err() {
        return false;
    }
    runtime.toggle_tmux_manager_panel(snapshot.clone());
    render_manager_panel(&mut runtime, renderer)
}

pub(super) fn render_no_server_start_hint(renderer: &mut WgpuRenderer) -> bool {
    let Ok(mut runtime) = super::smoke_runtime() else {
        return false;
    };
    runtime.open_tmux_manager_panel_with_workspaces(no_server_snapshot(), Vec::new());
    render_manager_panel_contains(&mut runtime, renderer, "Enterstart-session")
        && render_manager_panel_contains(&mut runtime, renderer, "tmuxnew-session-d-s<session>")
        && render_manager_panel_contains(&mut runtime, renderer, "Enterstart-sessiontocreate")
}

pub(super) fn render_outside_attach_hint(renderer: &mut WgpuRenderer) -> bool {
    let Ok(mut runtime) = super::smoke_runtime() else {
        return false;
    };
    runtime.open_tmux_manager_panel_with_workspaces(detached_snapshot(), Vec::new());
    render_manager_panel_contains(&mut runtime, renderer, "Outsidetmux")
        && render_manager_panel_contains(&mut runtime, renderer, "tmuxattach-session-t<session>")
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

fn no_server_snapshot() -> TmuxManagerSnapshot {
    TmuxManagerSnapshot {
        status: TmuxManagerStatus::NoServer,
        state: TmuxState::default(),
        current: None,
    }
}

fn detached_snapshot() -> TmuxManagerSnapshot {
    TmuxManagerSnapshot {
        status: TmuxManagerStatus::Available,
        state: TmuxState {
            sessions: vec![TmuxSession {
                name: "gromaq-runtime-detached".to_owned(),
                attached: false,
            }],
            windows: Vec::new(),
            panes: Vec::new(),
        },
        current: None,
    }
}

fn full_prompt_grid() -> String {
    (0..16)
        .map(|row| format!("gromaq startup line {row:02}\r\n"))
        .chain(std::iter::once(
            "Now using node v20.20.0 (npm v10.8.2)\r\n~ ................................ rb 2.7.5 22:17:47\r\n> ".to_owned(),
        ))
        .collect()
}
