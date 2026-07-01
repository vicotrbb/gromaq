//! Native tmux manager UI runtime smoke.

mod cleanup;
mod confirmation;
mod destructive;
mod help;
mod mouse;
mod pty;
mod render;
mod report;
mod shortcuts;
mod skipped_handoffs;
mod workspace;

use super::availability::tmux_missing_skip_exit;
use crate::app::{NativeTerminalRuntime, NativeTerminalRuntimeConfig};
use crate::cli::CliExit;
use crate::renderer::{RendererConfig, WgpuRenderer};
use crate::tmux::{
    SocketTmuxCommandRunner, SystemTmuxCommandRunner, TmuxCommandRunner, TmuxManager, TmuxProbe,
    TmuxStateReader,
};
use cleanup::TmuxUiSmokeCleanup;
use confirmation::{drive_confirmation_cancel, drive_safe_action};
use destructive::drive_kill_session_confirmation;
use help::render_help_catalog;
use mouse::{drive_mouse_action_selection, drive_mouse_focus, drive_mouse_workspace_selection};
use pty::{TmuxUiSmokePtySession, TmuxUiSmokePtySpawner};
use render::{
    render_current_pane_marker, render_current_target_pane_detail, render_manager_panel,
    render_manager_panel_contains, render_manager_state, render_no_server_start_hint,
    render_outside_attach_hint, render_startup_manager_after_shell_prompt, render_status_feedback,
    render_status_pane_command, render_status_strip,
};
use report::runtime_tmux_ui_smoke_result;
use shortcuts::{
    drive_attach_session_handoff, drive_destructive_shortcut_confirmation,
    drive_detach_session_failure, drive_kill_pane_confirmation, drive_kill_window_confirmation,
    drive_missing_tmux_feedback, drive_name_entry_action, drive_new_window_shortcut,
    drive_refresh_preserves_action_focus, drive_refresh_shortcut, drive_rename_session_action,
    drive_rename_window_action, drive_select_pane_shortcut, drive_split_down_shortcut,
    drive_split_right_shortcut, drive_start_session_feedback, drive_start_session_pty_handoff,
    drive_unavailable_shortcut_block, drive_window_cycle_shortcuts, drive_zoom_shortcut,
    drive_zoom_toggle_shortcut,
};
use workspace::{docs_workspace_preset, run_workspace_proof, workspace_preset};

const UI_SESSION: &str = "gromaq-runtime-tmux-ui";

pub(in crate::cli) fn runtime_tmux_ui_smoke_exit() -> CliExit {
    let probe = match TmuxProbe::new(SystemTmuxCommandRunner).probe() {
        Ok(probe) => probe,
        Err(error) => return ui_failure(format!("tmux probe failed: {error:?}")),
    };
    if !probe.installed {
        return tmux_missing_skip_exit("runtime tmux ui smoke");
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
    let status_pane_command_checked =
        render_status_pane_command(&mut runtime, &mut renderer, &snapshot);
    let status_feedback_checked = render_status_feedback(&mut runtime, &mut renderer, &snapshot);
    let workspace_preset = workspace_preset();
    let docs_workspace_preset = docs_workspace_preset();
    runtime.toggle_tmux_manager_panel_with_workspaces(
        snapshot.clone(),
        vec![workspace_preset.clone(), docs_workspace_preset],
    );
    let manager_opened = runtime.tmux_manager_panel_is_open();
    let manager_rendered = render_manager_panel(&mut runtime, &mut renderer);
    let manager_state_checked = render_manager_state(&mut renderer, &snapshot, UI_SESSION);
    let target_pane_detail_checked = render_current_target_pane_detail(&mut renderer);
    let current_pane_marker_checked = render_current_pane_marker(&mut renderer);
    let startup_manager_after_shell_prompt_checked =
        render_startup_manager_after_shell_prompt(&mut renderer, &snapshot);
    let confirmation_checked = drive_confirmation_cancel(&mut runtime);
    let cancellation_feedback_checked = confirmation_checked
        && render_manager_panel_contains(&mut runtime, &mut renderer, "kill-windowcanceled");
    let destructive_shortcut_checked = drive_destructive_shortcut_confirmation(&mut runtime);
    let refresh_shortcut_requested = drive_refresh_shortcut(&mut runtime);
    let refresh_focus_preserved = drive_refresh_preserves_action_focus(&snapshot);
    let no_server_start_hint_checked = render_no_server_start_hint(&mut renderer);
    let outside_attach_hint_checked = render_outside_attach_hint(&mut renderer);
    let help_catalog_checked = render_help_catalog(&snapshot, &mut renderer);
    let new_window_shortcut_checked = drive_new_window_shortcut(&mut runtime, &runner);
    let window_cycle_shortcuts_checked = drive_window_cycle_shortcuts(&mut runtime, &runner);
    let zoom_shortcut_checked = drive_zoom_shortcut(&mut runtime, &runner);
    let zoom_toggle_shortcut_checked = drive_zoom_toggle_shortcut(&mut runtime, &runner);
    let select_pane_shortcut_checked = drive_select_pane_shortcut(&mut runtime, &runner);
    let split_right_shortcut_checked = drive_split_right_shortcut(&mut runtime, &runner);
    let split_down_shortcut_checked = drive_split_down_shortcut(&mut runtime, &runner);
    let safe_action_dispatched = drive_safe_action(&mut runtime, &runner);
    let attach_session_pty_handoff_checked = drive_attach_session_handoff(&mut runtime);
    let detach_session_failure_feedback_checked =
        drive_detach_session_failure(&mut runtime, &runner)
            && render_manager_panel_contains(&mut runtime, &mut renderer, "detach-sessionfailed");
    let tmux_missing_feedback_checked = drive_missing_tmux_feedback(&snapshot, &mut renderer);
    let rename_window_dispatched = drive_rename_window_action(&mut runtime, &runner);
    let rename_window_feedback_checked = rename_window_dispatched
        && render_manager_panel_contains(&mut runtime, &mut renderer, "rename-windowsuccess");
    let rename_session_dispatched = drive_rename_session_action(
        &mut runtime,
        &runner,
        UI_SESSION,
        "gromaq-runtime-tmux-ui-rn",
    );
    let rename_session_feedback_checked = rename_session_dispatched
        && render_manager_panel_contains(&mut runtime, &mut renderer, "rename-sessionsuccess");
    let kill_pane_confirmation_dispatched = drive_kill_pane_confirmation(&mut runtime, &runner);
    let kill_window_confirmation_dispatched = drive_kill_window_confirmation(&mut runtime, &runner);
    let name_entry_dispatched = drive_name_entry_action(&mut runtime, &runner);
    let start_session_pty_handoff_checked = drive_start_session_pty_handoff(&runner);
    let start_session_feedback_checked = drive_start_session_feedback(&runner, &mut renderer);
    let workspace_result =
        run_workspace_proof(&snapshot, &workspace_preset, &runner, &mut renderer);
    let skipped_pty_handoffs = skipped_handoffs::drive(&snapshot, &runner, &mut renderer);
    let mouse_focus_checked = render_manager_panel(&mut runtime, &mut renderer)
        && drive_mouse_focus(&mut runtime)
        && render_manager_panel_contains(&mut runtime, &mut renderer, "focuswindows");
    let mouse_action_selection_checked = render_manager_panel(&mut runtime, &mut renderer)
        && drive_mouse_action_selection(&mut runtime)
        && render_manager_panel_contains(&mut runtime, &mut renderer, "Enterkill-window");
    let mouse_workspace_selection_checked = render_manager_panel(&mut runtime, &mut renderer)
        && drive_mouse_workspace_selection(&mut runtime)
        && render_manager_panel_contains(&mut runtime, &mut renderer, "docs-ui-smoke*");
    let kill_session_confirmation_dispatched =
        drive_kill_session_confirmation(&mut runtime, &runner)
            && render_manager_panel_contains(&mut runtime, &mut renderer, "kill-sessionsuccess");
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

    let workspace_launch = ["not-started", "started"][usize::from(workspace_result.started)];
    runtime_tmux_ui_smoke_result(format!(
        "runtime tmux ui smoke: ok\ntmux available: true\nsocket: {socket}\nsession: {UI_SESSION}\nmanager panel opened: {manager_opened}\nstatus strip rendered: {status_rendered}\nstatus pane command checked: {status_pane_command_checked}\nstatus feedback checked: {status_feedback_checked}\nmanager panel rendered: {manager_rendered}\nmanager state checked: {manager_state_checked}\ntarget pane detail checked: {target_pane_detail_checked}\ncurrent pane marker checked: {current_pane_marker_checked}\nstartup manager after shell prompt checked: {startup_manager_after_shell_prompt_checked}\nconfirmation path checked: {confirmation_checked}\ncancellation feedback checked: {cancellation_feedback_checked}\ndestructive shortcut checked: {destructive_shortcut_checked}\nunavailable shortcut blocked: {unavailable_shortcut_blocked}\nno-server start hint checked: {no_server_start_hint_checked}\noutside attach hint checked: {outside_attach_hint_checked}\nmouse focus checked: {mouse_focus_checked}\nmouse action selection checked: {mouse_action_selection_checked}\nmouse workspace selection checked: {mouse_workspace_selection_checked}\nrefresh shortcut requested: {refresh_shortcut_requested}\nrefresh focus preserved: {refresh_focus_preserved}\nhelp catalog checked: {help_catalog_checked}\nnew window shortcut checked: {new_window_shortcut_checked}\nwindow cycle shortcuts checked: {window_cycle_shortcuts_checked}\nzoom shortcut checked: {zoom_shortcut_checked}\nzoom toggle shortcut checked: {zoom_toggle_shortcut_checked}\nselect pane shortcut checked: {select_pane_shortcut_checked}\nsplit right shortcut checked: {split_right_shortcut_checked}\nsplit down shortcut checked: {split_down_shortcut_checked}\nsafe action dispatched: {safe_action_dispatched}\nattach session pty handoff checked: {attach_session_pty_handoff_checked}\nskipped pty handoffs checked: attach={} start={} workspace={}\ndetach session failure feedback checked: {detach_session_failure_feedback_checked}\ntmux missing feedback checked: {tmux_missing_feedback_checked}\nrename window action dispatched: {rename_window_dispatched}\nrename window feedback checked: {rename_window_feedback_checked}\nrename session action dispatched: {rename_session_dispatched}\nrename session feedback checked: {rename_session_feedback_checked}\nkill pane confirmation dispatched: {kill_pane_confirmation_dispatched}\nkill window confirmation dispatched: {kill_window_confirmation_dispatched}\nkill session confirmation dispatched: {kill_session_confirmation_dispatched}\nname entry action dispatched: {name_entry_dispatched}\nstart session pty handoff checked: {start_session_pty_handoff_checked}\nstart session feedback checked: {start_session_feedback_checked}\nworkspace launch: {workspace_launch}\nworkspace feedback checked: {}\nworkspace command hints checked: {}\nworkspace existing attach checked: {}\nworkspace failure feedback checked: {}\nworkspace invalid preflight checked: {}\nworkspace duplicate prevented: {}\nstate reader observed session: {observed_session}\nstate sessions: {}\nstate windows: {}\nstate panes: {}\ncleanup killed session: {cleanup_ok}\n",
        skipped_pty_handoffs.attach,
        skipped_pty_handoffs.start,
        skipped_pty_handoffs.workspace,
        workspace_result.feedback_checked,
        workspace_result.command_hints_checked,
        workspace_result.existing_attach_checked,
        workspace_result.failure_feedback_checked,
        workspace_result.invalid_preflight_checked,
        workspace_result.duplicate_prevented,
        state.sessions.len(),
        state.windows.len(),
        state.panes.len()
    ))
}

type SmokeRuntime = NativeTerminalRuntime<TmuxUiSmokePtySession>;

fn smoke_runtime() -> Result<SmokeRuntime, String> {
    let mut runtime = SmokeRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 320,
        terminal_rows: 10,
        ..NativeTerminalRuntimeConfig::default()
    })
    .map_err(|error| error.to_string())?;
    runtime
        .write_startup_text("gromaq tmux ui smoke\r\n> ")
        .map_err(|error| error.to_string())?;
    runtime
        .start_shell(&TmuxUiSmokePtySpawner)
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
