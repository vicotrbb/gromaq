//! Help catalog proof helper for the native tmux UI smoke.

use winit::keyboard::{Key, ModifiersState};

use crate::app::{NativeTerminalRuntimeConfig, TmuxManagerKeyOutcome};
use crate::renderer::WgpuRenderer;

use super::pty::TmuxUiSmokePtySpawner;
use super::render::last_plan_text;

pub(super) fn drive_help_catalog(runtime: &mut super::SmokeRuntime) -> bool {
    matches!(
        runtime.handle_tmux_manager_key(&Key::Character("?".into()), ModifiersState::empty()),
        TmuxManagerKeyOutcome::Consumed
    )
}

pub(super) fn render_help_catalog(
    snapshot: &crate::tmux::TmuxManagerSnapshot,
    renderer: &mut WgpuRenderer,
) -> bool {
    render_help_catalog_with(
        snapshot,
        renderer,
        &["split-pane-righttmux", "refreshsnapshot"],
    )
}

pub(super) fn render_help_catalog_action_coverage(
    snapshot: &crate::tmux::TmuxManagerSnapshot,
    renderer: &mut WgpuRenderer,
) -> bool {
    render_help_catalog_with(
        snapshot,
        renderer,
        &[
            "start-sessiontmuxnew-session",
            "attach-sessiontmuxattach-session",
            "detach-sessiontmuxdetach-client",
            "split-pane-righttmuxsplit-window-hCtrl-b%",
            "split-pane-downtmuxsplit-window-vCtrl-b",
            "new-windowtmuxnew-windowCtrl-bc",
            "rename-sessiontmuxrename-session",
            "rename-windowtmuxrename-window",
            "next-windowtmuxnext-windowCtrl-bn",
            "previous-windowtmuxprevious-windowCtrl-bp",
            "zoom-panetmuxresize-pane-ZCtrl-bz",
            "select-panetmuxselect-pane",
            "kill-panetmuxkill-pane-t<pane>Ctrl-bx",
            "kill-windowtmuxkill-window-t<window>Ctrl-b&",
            "kill-sessiontmuxkill-session",
        ],
    )
}

fn render_help_catalog_with(
    snapshot: &crate::tmux::TmuxManagerSnapshot,
    renderer: &mut WgpuRenderer,
    required_fragments: &[&str],
) -> bool {
    let Ok(mut runtime) = super::SmokeRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 2400,
        terminal_rows: 10,
        ..NativeTerminalRuntimeConfig::default()
    }) else {
        return false;
    };
    if runtime
        .write_startup_text("gromaq tmux ui help smoke\r\n> ")
        .is_err()
    {
        return false;
    }
    if runtime.start_shell(&TmuxUiSmokePtySpawner).is_err() {
        return false;
    }
    runtime.open_tmux_manager_panel_with_workspaces(snapshot.clone(), Vec::new());
    if !drive_help_catalog(&mut runtime) {
        return false;
    }
    runtime.invalidate_terminal_frame();
    let Ok(rendered) = runtime.render_terminal_frame_with_status_overlay(renderer, None) else {
        return false;
    };
    if !rendered {
        return false;
    }
    let text = last_plan_text(renderer);
    text.contains("tmuxhelp")
        && required_fragments
            .iter()
            .all(|fragment| text.contains(fragment))
}
