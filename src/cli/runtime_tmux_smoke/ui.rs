//! Native tmux manager UI runtime smoke.

mod cleanup;
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
use render::{render_manager_panel, render_status_strip};
use shortcuts::{
    drive_destructive_shortcut_confirmation, drive_refresh_shortcut, drive_shortcut_action,
};
use workspace::{run_workspace_proof, workspace_preset};

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
    runtime.toggle_tmux_manager_panel_with_workspaces(
        snapshot.clone(),
        vec![workspace_preset.clone()],
    );
    let manager_opened = runtime.tmux_manager_panel_is_open();
    let manager_rendered = render_manager_panel(&mut runtime, &mut renderer);
    let confirmation_checked = drive_confirmation_cancel(&mut runtime);
    let destructive_shortcut_checked = drive_destructive_shortcut_confirmation(&mut runtime);
    let refresh_shortcut_requested = drive_refresh_shortcut(&mut runtime);
    let shortcut_action_dispatched = drive_shortcut_action(&mut runtime, &runner);
    let safe_action_dispatched = drive_safe_action(&mut runtime, &runner);
    let name_entry_dispatched = drive_name_entry_action(&mut runtime, &runner);
    let workspace_result = run_workspace_proof(&snapshot, &workspace_preset, &runner);
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
            "runtime tmux ui smoke: ok\ntmux available: true\nsocket: {socket}\nsession: {UI_SESSION}\nmanager panel opened: {manager_opened}\nstatus strip rendered: {status_rendered}\nmanager panel rendered: {manager_rendered}\nconfirmation path checked: {confirmation_checked}\ndestructive shortcut checked: {destructive_shortcut_checked}\nrefresh shortcut requested: {refresh_shortcut_requested}\nshortcut action dispatched: {shortcut_action_dispatched}\nsafe action dispatched: {safe_action_dispatched}\nname entry action dispatched: {name_entry_dispatched}\nworkspace launch: {workspace_launch}\nworkspace duplicate prevented: {}\nstate reader observed session: {observed_session}\nstate sessions: {}\nstate windows: {}\nstate panes: {}\ncleanup killed session: {cleanup_ok}\n",
            workspace_result.duplicate_prevented,
            state.sessions.len(),
            state.windows.len(),
            state.panes.len()
        ),
        stderr: String::new(),
    }
}

fn smoke_runtime() -> Result<NativeTerminalRuntime<()>, String> {
    let mut runtime = NativeTerminalRuntime::<()>::new(NativeTerminalRuntimeConfig {
        terminal_cols: 96,
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

fn drive_confirmation_cancel(runtime: &mut NativeTerminalRuntime<()>) -> bool {
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

fn drive_safe_action(
    runtime: &mut NativeTerminalRuntime<()>,
    runner: &SocketTmuxCommandRunner,
) -> bool {
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

fn drive_name_entry_action(
    runtime: &mut NativeTerminalRuntime<()>,
    runner: &SocketTmuxCommandRunner,
) -> bool {
    for _ in 0..3 {
        if matches!(
            runtime
                .handle_tmux_manager_key(&Key::Named(NamedKey::ArrowDown), ModifiersState::empty()),
            TmuxManagerKeyOutcome::Ignored
        ) {
            return false;
        }
    }
    if !matches!(
        runtime.handle_tmux_manager_key(&Key::Named(NamedKey::Enter), ModifiersState::empty()),
        TmuxManagerKeyOutcome::Consumed
    ) {
        return false;
    }
    for character in "gromaq-runtime-tmux-ui-name".chars() {
        if !matches!(
            runtime.handle_tmux_manager_key(
                &Key::Character(character.to_string().into()),
                ModifiersState::empty()
            ),
            TmuxManagerKeyOutcome::Consumed
        ) {
            return false;
        }
    }
    let requested =
        runtime.handle_tmux_manager_key(&Key::Named(NamedKey::Enter), ModifiersState::empty());
    matches!(
        runtime.dispatch_tmux_manager_action(requested, runner),
        Some(TmuxActionResult::Success {
            action_id: ActionId::StartSession,
            ..
        })
    )
}
