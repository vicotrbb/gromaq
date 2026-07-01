//! Header status proof for the native tmux UI smoke.

use crate::renderer::WgpuRenderer;
use crate::tmux::TmuxManagerSnapshot;

use super::fixtures::{current_target_snapshot, detached_snapshot, no_server_snapshot};
use super::render_manager_panel_contains;

pub(in crate::cli::runtime_tmux_smoke::ui) fn render_manager_header_status(
    renderer: &mut WgpuRenderer,
) -> bool {
    let Ok(mut runtime) = super::super::smoke_runtime() else {
        return false;
    };
    render_header_status(
        &mut runtime,
        renderer,
        current_target_snapshot(),
        "statusattached",
    ) && render_header_status(
        &mut runtime,
        renderer,
        detached_snapshot(),
        "statusdetached",
    ) && render_header_status(
        &mut runtime,
        renderer,
        no_server_snapshot(),
        "statusnoserver",
    ) && render_header_status(
        &mut runtime,
        renderer,
        TmuxManagerSnapshot::missing(),
        "statusmissing",
    )
}

fn render_header_status(
    runtime: &mut super::super::SmokeRuntime,
    renderer: &mut WgpuRenderer,
    snapshot: TmuxManagerSnapshot,
    expected: &str,
) -> bool {
    runtime.open_tmux_manager_panel_with_workspaces(snapshot, Vec::new());
    render_manager_panel_contains(runtime, renderer, expected)
}
