//! Current-target marker proof for the native tmux UI smoke.

use winit::keyboard::{Key, ModifiersState, NamedKey};

use crate::app::TmuxManagerPanelState;
use crate::renderer::WgpuRenderer;

use super::fixtures::current_target_snapshot;
use super::last_plan_text;

pub(in crate::cli::runtime_tmux_smoke::ui) fn render_current_pane_marker(
    renderer: &mut WgpuRenderer,
) -> bool {
    let Ok(mut runtime) = super::super::smoke_runtime() else {
        return false;
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
    runtime.invalidate_terminal_frame();
    runtime
        .render_terminal_frame_with_tmux_manager_panel(renderer, &snapshot, &panel)
        .is_ok_and(|rendered| rendered)
        && last_plan_text(renderer).contains("shell:zsh*100x30")
        && last_plan_text(renderer).contains("editor:nvim100x30@")
}
