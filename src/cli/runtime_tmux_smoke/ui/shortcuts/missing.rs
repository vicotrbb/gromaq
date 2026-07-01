//! Missing-tmux shortcut feedback proof helper.

use winit::keyboard::{Key, ModifiersState};

use crate::tmux::{
    ActionId, TmuxActionResult, TmuxCommandOutput, TmuxCommandRunner, TmuxError,
    TmuxManagerSnapshot,
};

use super::super::render::render_manager_panel_contains;

pub(in crate::cli::runtime_tmux_smoke::ui) fn drive_missing_tmux_feedback(
    snapshot: &TmuxManagerSnapshot,
    renderer: &mut crate::renderer::WgpuRenderer,
) -> bool {
    let Ok(mut runtime) = super::super::smoke_runtime() else {
        return false;
    };
    runtime.open_tmux_manager_panel_with_workspaces(snapshot.clone(), Vec::new());
    let requested =
        runtime.handle_tmux_manager_key(&Key::Character("s".into()), ModifiersState::empty());
    matches!(
        runtime.dispatch_tmux_manager_action(requested, &MissingTmuxRunner),
        Some(TmuxActionResult::TmuxMissing {
            action_id: ActionId::SplitPaneRight,
            ..
        })
    ) && render_manager_panel_contains(&mut runtime, renderer, "split-pane-rightfailed:tmuxmissing")
}

#[derive(Debug, Clone, Copy)]
struct MissingTmuxRunner;

impl TmuxCommandRunner for MissingTmuxRunner {
    fn run_tmux(&self, _args: &[&str]) -> Result<TmuxCommandOutput, TmuxError> {
        Err(TmuxError::Missing)
    }
}
