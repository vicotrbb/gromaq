//! Render-plan proof helpers for the native tmux UI smoke.

mod current_marker;
mod fixtures;
mod header_status;
mod hints;
mod status_guidance;

pub(super) use current_marker::{render_current_pane_marker, render_current_target_row_markers};
pub(super) use header_status::render_manager_header_status;
pub(super) use hints::{
    render_no_server_start_hint, render_outside_attach_hint, render_outside_attach_target,
};
pub(super) use status_guidance::render_status_guidance;

use crate::app::{NativeTerminalRuntimeConfig, TmuxUiSnapshot};
use crate::renderer::WgpuRenderer;
use crate::tmux::TmuxManagerSnapshot;

use fixtures::{current_target_snapshot, full_prompt_grid};

pub(super) const STARTUP_MANAGER_SMALL_GRID_COLS: u16 = 69;
pub(super) const STARTUP_MANAGER_SMALL_GRID_ROWS: u16 = 17;

pub(super) fn render_status_strip(
    runtime: &mut super::SmokeRuntime,
    renderer: &mut WgpuRenderer,
    snapshot: &TmuxManagerSnapshot,
) -> bool {
    let strip = status_strip(snapshot);
    runtime
        .render_terminal_frame_with_tmux_status_strip(renderer, &strip)
        .is_ok_and(|rendered| rendered)
        && last_plan_text(renderer).contains("tmux:")
}

pub(super) fn render_status_pane_command(
    runtime: &mut super::SmokeRuntime,
    renderer: &mut WgpuRenderer,
    snapshot: &TmuxManagerSnapshot,
) -> bool {
    let strip = status_strip(snapshot);
    let Some(command) = strip.active_pane_command.as_deref() else {
        return false;
    };
    runtime.invalidate_terminal_frame();
    runtime
        .render_terminal_frame_with_tmux_status_strip(renderer, &strip)
        .is_ok_and(|rendered| rendered)
        && last_plan_text(renderer).contains(command)
}

pub(super) fn render_status_feedback(
    runtime: &mut super::SmokeRuntime,
    renderer: &mut WgpuRenderer,
    snapshot: &TmuxManagerSnapshot,
) -> bool {
    let mut strip = status_strip(snapshot);
    strip.pending_feedback = Some("split-pane-right success".to_owned());
    strip.confirmation_feedback = Some("confirm kill-window | Ctrl-b &".to_owned());
    runtime.invalidate_terminal_frame();
    runtime
        .render_terminal_frame_with_tmux_status_strip(renderer, &strip)
        .is_ok_and(|rendered| rendered)
        && last_plan_text(renderer).contains("split-pane-rightsuccess")
        && last_plan_text(renderer).contains("confirm:confirmkill-window|Ctrl-b&")
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

pub(super) fn render_manager_state(
    renderer: &mut WgpuRenderer,
    snapshot: &TmuxManagerSnapshot,
    session: &str,
) -> bool {
    let Ok(mut runtime) = super::smoke_runtime() else {
        return false;
    };
    runtime.open_tmux_manager_panel_with_workspaces(snapshot.clone(), Vec::new());
    if !render_manager_panel(&mut runtime, renderer) {
        return false;
    }
    let text = last_plan_text(renderer);
    let Some(window) = snapshot
        .state
        .windows
        .iter()
        .find(|window| window.session_name == session)
    else {
        return false;
    };
    let Some(pane) = snapshot
        .state
        .panes
        .iter()
        .find(|pane| pane.session_name == session && pane.window_index == window.index)
    else {
        return false;
    };
    text.contains("tmuxmanager")
        && target_rendered(&text, snapshot)
        && text.contains("Sessions")
        && text.contains(session)
        && text.contains("Windows")
        && text.contains(&format!("{}:{}", window.index, window.name))
        && text.contains("Panes")
        && text.contains(&pane.id)
        && text.contains("?help")
        && text.contains("rrefresh")
        && text.contains("Escclose")
        && (pane.current_command.is_empty() || text.contains(&pane.current_command))
}

pub(super) fn render_current_target_pane_detail(renderer: &mut WgpuRenderer) -> bool {
    let Ok(mut runtime) = super::smoke_runtime() else {
        return false;
    };
    runtime.open_tmux_manager_panel_with_workspaces(current_target_snapshot(), Vec::new());
    if !render_manager_panel(&mut runtime, renderer) {
        return false;
    }
    let text = last_plan_text(renderer);
    text.contains("targetalpha:1:%2") && text.contains("editor:nvim") && text.contains("100x30")
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
        terminal_cols: STARTUP_MANAGER_SMALL_GRID_COLS,
        terminal_rows: STARTUP_MANAGER_SMALL_GRID_ROWS,
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

fn status_strip(snapshot: &TmuxManagerSnapshot) -> TmuxUiSnapshot {
    TmuxUiSnapshot::from_manager_snapshot(snapshot)
}

pub(super) fn last_plan_text(renderer: &WgpuRenderer) -> String {
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

fn target_rendered(text: &str, snapshot: &TmuxManagerSnapshot) -> bool {
    snapshot.current.as_ref().map_or_else(
        || text.contains("targetnone"),
        |current| {
            text.contains(&format!(
                "target{}:{}:{}",
                current.session_name, current.window_index, current.pane_id
            ))
        },
    )
}
