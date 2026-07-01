//! Native tmux manager UI runtime smoke.

mod cleanup;
mod mouse;
mod pty;
mod render;
mod shortcuts;
mod workspace;

use winit::keyboard::{Key, ModifiersState, NamedKey};

use crate::app::{NativeTerminalRuntime, NativeTerminalRuntimeConfig, TmuxManagerKeyOutcome};
use crate::cli::CliExit;
use crate::renderer::{RendererConfig, WgpuRenderer};
use crate::tmux::{
    ActionId, SocketTmuxCommandRunner, SystemTmuxCommandRunner, TmuxActionResult,
    TmuxCommandRunner, TmuxManager, TmuxProbe, TmuxStateReader,
};
use cleanup::TmuxUiSmokeCleanup;
use mouse::{drive_mouse_action_selection, drive_mouse_focus, drive_mouse_workspace_selection};
use pty::TmuxUiSmokePtySession;
use render::{
    render_manager_panel, render_manager_panel_contains, render_startup_manager_after_shell_prompt,
    render_status_strip,
};
use shortcuts::{
    drive_destructive_shortcut_confirmation, drive_name_entry_action, drive_refresh_shortcut,
    drive_rename_window_action, drive_select_pane_shortcut, drive_shortcut_action,
    drive_split_down_shortcut, drive_unavailable_shortcut_block, drive_window_cycle_shortcuts,
    drive_zoom_shortcut,
};
use workspace::{docs_workspace_preset, run_workspace_proof, workspace_preset};

const UI_SESSION: &str = "gromaq-runtime-tmux-ui";

pub(in crate::cli) fn runtime_tmux_ui_smoke_exit() -> CliExit {
    let probe = match TmuxProbe::new(SystemTmuxCommandRunner).probe() {
        Ok(probe) => probe,
        Err(error) => return ui_failure(format!("tmux probe failed: {error:?}")),
    };
    if !probe.installed {
        return CliExit {
            code: 0,
            stdout: "runtime tmux ui smoke: ok\ntmux available: false\nskipped: tmux not found on PATH\n"
                .to_owned(),
            stderr: String::new(),
        };
    }

    let socket = format!("gromaq-runtime-tmux-ui-{}", std::process::id());
    let runner = SocketTmuxCommandRunner::new(socket.clone());
    let mut cleanup = TmuxUiSmokeCleanup::new(runner.clone());
    if let Err(error) = runner.run_tmux(&["new-session", "-d", "-s", UI_SESSION]) {
        return ui_failure(format!("create isolated tmux UI session failed: {error:?}"));
    }

    let snapshot = match TmuxManager::new(runner.clone()).snapshot() {
        Ok(snapshot) => snapshot,
        Err(error) => return ui_failure(format!("tmux manager snapshot failed: {error:?}")),
    };
    let mut runtime = match smoke_runtime() {
        Ok(runtime) => runtime,
        Err(error) => return ui_failure(error),
    };
    let mut renderer = match WgpuRenderer::new(RendererConfig::default()) {
        Ok(renderer) => renderer,
        Err(error) => return ui_failure(error.to_string()),
    };

    let status_rendered = render_status_strip(&mut runtime, &mut renderer, &snapshot);
    let workspace_preset = workspace_preset();
    let docs_workspace_preset = docs_workspace_preset();
    runtime.toggle_tmux_manager_panel_with_workspaces(
        snapshot.clone(),
        vec![workspace_preset.clone(), docs_workspace_preset],
    );
    let manager_opened = runtime.tmux_manager_panel_is_open();
    let manager_rendered = render_manager_panel(&mut runtime, &mut renderer);
    let startup_manager_after_shell_prompt_checked =
        render_startup_manager_after_shell_prompt(&mut renderer, &snapshot);
    let confirmation_checked = drive_confirmation_cancel(&mut runtime);
    let cancellation_feedback_checked = confirmation_checked
        && render_manager_panel_contains(&mut runtime, &mut renderer, "kill-windowcanceled");
    let destructive_shortcut_checked = drive_destructive_shortcut_confirmation(&mut runtime);
    let refresh_shortcut_requested = drive_refresh_shortcut(&mut runtime);
    let shortcut_action_dispatched = drive_shortcut_action(&mut runtime, &runner);
    let window_cycle_shortcuts_checked = drive_window_cycle_shortcuts(&mut runtime, &runner);
    let zoom_shortcut_checked = drive_zoom_shortcut(&mut runtime, &runner);
    let select_pane_shortcut_checked = drive_select_pane_shortcut(&mut runtime, &runner);
    let split_down_shortcut_checked = drive_split_down_shortcut(&mut runtime, &runner);
    let safe_action_dispatched = drive_safe_action(&mut runtime, &runner);
    let rename_window_dispatched = drive_rename_window_action(&mut runtime, &runner);
    let name_entry_dispatched = drive_name_entry_action(&mut runtime, &runner);
    let workspace_result = run_workspace_proof(&snapshot, &workspace_preset, &runner);
    let mouse_focus_checked = render_manager_panel(&mut runtime, &mut renderer)
        && drive_mouse_focus(&mut runtime)
        && render_manager_panel_contains(&mut runtime, &mut renderer, "focuswindows");
    let mouse_action_selection_checked = render_manager_panel(&mut runtime, &mut renderer)
        && drive_mouse_action_selection(&mut runtime)
        && render_manager_panel_contains(&mut runtime, &mut renderer, "Enterkill-window");
    let mouse_workspace_selection_checked = render_manager_panel(&mut runtime, &mut renderer)
        && drive_mouse_workspace_selection(&mut runtime)
        && render_manager_panel_contains(&mut runtime, &mut renderer, "docs-ui-smoke*");
    let unavailable_shortcut_blocked = drive_unavailable_shortcut_block(&mut runtime)
        && render_manager_panel_contains(
            &mut runtime,
            &mut renderer,
            "split-pane-rightneedsactivetmux",
        );
    let state = match TmuxStateReader::new(runner.clone()).read_state() {
        Ok(state) => state,
        Err(error) => return ui_failure(format!("tmux UI state reader failed: {error:?}")),
    };
    let observed_session = state
        .sessions
        .iter()
        .any(|session| session.name == UI_SESSION);
    let cleanup_ok = cleanup.kill_server();

    let workspace_launch = if workspace_result.started {
        "started"
    } else {
        "not-started"
    };
    CliExit {
        code: 0,
        stdout: format!(
            "runtime tmux ui smoke: ok\ntmux available: true\nsocket: {socket}\nsession: {UI_SESSION}\nmanager panel opened: {manager_opened}\nstatus strip rendered: {status_rendered}\nmanager panel rendered: {manager_rendered}\nstartup manager after shell prompt checked: {startup_manager_after_shell_prompt_checked}\nconfirmation path checked: {confirmation_checked}\ncancellation feedback checked: {cancellation_feedback_checked}\ndestructive shortcut checked: {destructive_shortcut_checked}\nunavailable shortcut blocked: {unavailable_shortcut_blocked}\nmouse focus checked: {mouse_focus_checked}\nmouse action selection checked: {mouse_action_selection_checked}\nmouse workspace selection checked: {mouse_workspace_selection_checked}\nrefresh shortcut requested: {refresh_shortcut_requested}\nshortcut action dispatched: {shortcut_action_dispatched}\nwindow cycle shortcuts checked: {window_cycle_shortcuts_checked}\nzoom shortcut checked: {zoom_shortcut_checked}\nselect pane shortcut checked: {select_pane_shortcut_checked}\nsplit down shortcut checked: {split_down_shortcut_checked}\nsafe action dispatched: {safe_action_dispatched}\nrename window action dispatched: {rename_window_dispatched}\nname entry action dispatched: {name_entry_dispatched}\nworkspace launch: {workspace_launch}\nworkspace feedback checked: {}\nworkspace duplicate prevented: {}\nstate reader observed session: {observed_session}\nstate sessions: {}\nstate windows: {}\nstate panes: {}\ncleanup killed session: {cleanup_ok}\n",
            workspace_result.feedback_checked,
            workspace_result.duplicate_prevented,
            state.sessions.len(),
            state.windows.len(),
            state.panes.len()
        ),
        stderr: String::new(),
    }
}

type SmokeRuntime = NativeTerminalRuntime<TmuxUiSmokePtySession>;

fn smoke_runtime() -> Result<SmokeRuntime, String> {
    let mut runtime = SmokeRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 220,
        terminal_rows: 10,
        ..NativeTerminalRuntimeConfig::default()
    })
    .map_err(|error| error.to_string())?;
    runtime
        .write_startup_text("gromaq tmux ui smoke\r\n> ")
        .map_err(|error| error.to_string())?;
    Ok(runtime)
}

fn ui_failure(message: String) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime tmux ui smoke failed: {message}\n"),
    }
}

fn drive_confirmation_cancel(runtime: &mut SmokeRuntime) -> bool {
    for key in [
        Key::Named(NamedKey::ArrowRight),
        Key::Named(NamedKey::ArrowRight),
        Key::Named(NamedKey::ArrowRight),
        Key::Named(NamedKey::ArrowRight),
        Key::Named(NamedKey::ArrowDown),
    ] {
        if matches!(
            runtime.handle_tmux_manager_key(&key, ModifiersState::empty()),
            TmuxManagerKeyOutcome::Ignored
        ) {
            return false;
        }
    }
    let confirmation =
        runtime.handle_tmux_manager_key(&Key::Named(NamedKey::Enter), ModifiersState::empty());
    let canceled =
        runtime.handle_tmux_manager_key(&Key::Named(NamedKey::Escape), ModifiersState::empty());
    matches!(
        confirmation,
        TmuxManagerKeyOutcome::ConfirmationRequired(ActionId::KillWindow)
    ) && matches!(canceled, TmuxManagerKeyOutcome::Consumed)
}

fn drive_safe_action(runtime: &mut SmokeRuntime, runner: &SocketTmuxCommandRunner) -> bool {
    let previous_action =
        runtime.handle_tmux_manager_key(&Key::Named(NamedKey::ArrowUp), ModifiersState::empty());
    let requested =
        runtime.handle_tmux_manager_key(&Key::Named(NamedKey::Enter), ModifiersState::empty());
    let result = runtime.dispatch_tmux_manager_action(requested, runner);
    !matches!(previous_action, TmuxManagerKeyOutcome::Ignored)
        && matches!(
            result,
            Some(TmuxActionResult::Success {
                action_id: ActionId::SplitPaneRight,
                ..
            })
        )
}
