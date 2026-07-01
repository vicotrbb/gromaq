//! Status-strip guidance proof for the native tmux UI smoke.

use crate::app::{TmuxStatusKind, TmuxUiSnapshot};
use crate::renderer::WgpuRenderer;

use super::last_plan_text;

pub(in crate::cli::runtime_tmux_smoke::ui) fn render_status_guidance(
    renderer: &mut WgpuRenderer,
) -> bool {
    render_guidance(renderer, TmuxStatusKind::Missing, "installtmux")
        && render_guidance(renderer, TmuxStatusKind::NoServer, "startsession")
        && render_guidance(renderer, TmuxStatusKind::Detached, "attachsession")
}

fn render_guidance(renderer: &mut WgpuRenderer, status: TmuxStatusKind, expected: &str) -> bool {
    let Ok(mut runtime) = super::super::smoke_runtime() else {
        return false;
    };
    let snapshot = TmuxUiSnapshot {
        status,
        current_session: None,
        current_window: None,
        visible_windows: Vec::new(),
        pane_count: None,
        active_pane_id: None,
        active_pane_command: None,
        pending_feedback: None,
        confirmation_feedback: None,
    };
    runtime.invalidate_terminal_frame();
    runtime
        .render_terminal_frame_with_tmux_status_strip(renderer, &snapshot)
        .is_ok_and(|rendered| rendered)
        && last_plan_text(renderer).contains(expected)
}
