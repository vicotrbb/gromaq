//! Current-target marker proof for the native tmux UI smoke.

use winit::keyboard::{Key, ModifiersState, NamedKey};

use crate::app::TmuxManagerPanelState;
use crate::renderer::WgpuRenderer;

use super::fixtures::current_target_snapshot;
use super::last_plan_text;

pub(in crate::cli::runtime_tmux_smoke::ui) fn render_current_pane_marker(
    renderer: &mut WgpuRenderer,
) -> bool {
    render_current_pane_marker_text(renderer).contains("editor:nvim100x30@")
}

pub(in crate::cli::runtime_tmux_smoke::ui) fn render_current_target_row_markers(
    renderer: &mut WgpuRenderer,
) -> bool {
    render_current_session_marker_text(renderer).contains("Sessionsalpha@beta*")
        && render_current_window_marker_text(renderer).contains("Windows0:shell*1:code@")
        && render_current_pane_marker_text(renderer).contains("editor:nvim100x30@")
}

fn render_current_session_marker_text(renderer: &mut WgpuRenderer) -> String {
    let Ok(mut runtime) = super::super::smoke_runtime() else {
        return String::new();
    };
    let snapshot = current_target_snapshot();
    let mut panel = TmuxManagerPanelState::open_for_snapshot(&snapshot);
    panel.handle_key(
        &Key::Named(NamedKey::ArrowDown),
        ModifiersState::empty(),
        &snapshot,
    );
    render_marker_text(&mut runtime, renderer, &snapshot, &panel)
}

fn render_current_window_marker_text(renderer: &mut WgpuRenderer) -> String {
    let Ok(mut runtime) = super::super::smoke_runtime() else {
        return String::new();
    };
    let snapshot = current_target_snapshot();
    let mut panel = TmuxManagerPanelState::open_for_snapshot(&snapshot);
    panel.focus_next();
    panel.handle_key(
        &Key::Named(NamedKey::ArrowUp),
        ModifiersState::empty(),
        &snapshot,
    );
    render_marker_text(&mut runtime, renderer, &snapshot, &panel)
}

fn render_current_pane_marker_text(renderer: &mut WgpuRenderer) -> String {
    let Ok(mut runtime) = super::super::smoke_runtime() else {
        return String::new();
    };
    let snapshot = current_target_snapshot();
    let mut panel = TmuxManagerPanelState::open_for_snapshot(&snapshot);
    panel.focus_next();
    panel.focus_next();
    panel.handle_key(
        &Key::Named(NamedKey::ArrowUp),
        ModifiersState::empty(),
        &snapshot,
    );
    render_marker_text(&mut runtime, renderer, &snapshot, &panel)
}

fn render_marker_text(
    runtime: &mut super::super::SmokeRuntime,
    renderer: &mut WgpuRenderer,
    snapshot: &crate::tmux::TmuxManagerSnapshot,
    panel: &TmuxManagerPanelState,
) -> String {
    runtime.invalidate_terminal_frame();
    if runtime
        .render_terminal_frame_with_tmux_manager_panel(renderer, snapshot, panel)
        .is_ok_and(|rendered| rendered)
    {
        last_plan_text(renderer)
    } else {
        String::new()
    }
}
