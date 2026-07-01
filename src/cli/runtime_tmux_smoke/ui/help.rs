//! Help catalog proof helper for the native tmux UI smoke.

use winit::keyboard::{Key, ModifiersState};

use crate::app::{NativeTerminalRuntimeConfig, TmuxManagerKeyOutcome};
use crate::renderer::WgpuRenderer;

use super::pty::TmuxUiSmokePtySpawner;
use super::render::render_manager_panel_contains;

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
    let Ok(mut runtime) = super::SmokeRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 900,
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
    drive_help_catalog(&mut runtime)
        && render_manager_panel_contains(&mut runtime, renderer, "tmuxhelp")
        && render_manager_panel_contains(&mut runtime, renderer, "split-pane-righttmux")
        && render_manager_panel_contains(&mut runtime, renderer, "refreshsnapshot")
}
