//! Empty-state hint render checks for the native tmux UI smoke.

use crate::renderer::WgpuRenderer;

use super::fixtures::{detached_snapshot, no_server_snapshot};
use super::render_manager_panel_contains;

pub(in crate::cli::runtime_tmux_smoke::ui) fn render_no_server_start_hint(
    renderer: &mut WgpuRenderer,
) -> bool {
    let Ok(mut runtime) = super::super::smoke_runtime() else {
        return false;
    };
    runtime.open_tmux_manager_panel_with_workspaces(no_server_snapshot(), Vec::new());
    render_manager_panel_contains(&mut runtime, renderer, "Enterstart-session")
        && render_manager_panel_contains(&mut runtime, renderer, "tmuxnew-session-d-s<session>")
        && render_manager_panel_contains(&mut runtime, renderer, "Enterstart-sessiontocreate")
        && render_manager_panel_contains(&mut runtime, renderer, "rrefresh")
        && render_manager_panel_contains(&mut runtime, renderer, "?help")
}

pub(in crate::cli::runtime_tmux_smoke::ui) fn render_outside_attach_hint(
    renderer: &mut WgpuRenderer,
) -> bool {
    let Ok(mut runtime) = super::super::smoke_runtime() else {
        return false;
    };
    runtime.open_tmux_manager_panel_with_workspaces(detached_snapshot(), Vec::new());
    render_manager_panel_contains(&mut runtime, renderer, "Outsidetmux")
        && render_manager_panel_contains(&mut runtime, renderer, "tmuxattach-session-t<session>")
        && render_manager_panel_contains(&mut runtime, renderer, "rrefresh")
        && render_manager_panel_contains(&mut runtime, renderer, "?help")
}

pub(in crate::cli::runtime_tmux_smoke::ui) fn render_outside_attach_target(
    renderer: &mut WgpuRenderer,
) -> bool {
    let Ok(mut runtime) = super::super::smoke_runtime() else {
        return false;
    };
    runtime.open_tmux_manager_panel_with_workspaces(detached_snapshot(), Vec::new());
    render_manager_panel_contains(
        &mut runtime,
        renderer,
        "Enterattach-sessiongromaq-runtime-detached",
    )
}
