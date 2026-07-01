use std::cell::RefCell;

use crate::{MockBackend, run_with_backend};

#[test]
fn runtime_tmux_smoke_cli_reports_isolated_tmux_proof_or_clean_skip() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--runtime-tmux-smoke"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("runtime tmux smoke: ok"));
    assert!(exit.stdout.contains("tmux available:"));
    if exit.stdout.contains("tmux available: true") {
        assert!(exit.stdout.contains("created session: true"));
        assert!(exit.stdout.contains("split pane action: success"));
        assert!(exit.stdout.contains("new window action: success"));
        assert!(exit.stdout.contains("state reader observed session: true"));
        assert!(exit.stdout.contains("state windows: 2"));
        assert!(exit.stdout.contains("state panes: 3"));
        assert!(exit.stdout.contains("cleanup killed session: true"));
    } else {
        assert!(exit.stdout.contains("skipped: tmux not found on PATH"));
    }
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
}

#[test]
fn runtime_tmux_ui_smoke_cli_reports_native_manager_ui_proof_or_clean_skip() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--runtime-tmux-ui-smoke"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("runtime tmux ui smoke: ok"));
    assert!(exit.stdout.contains("tmux available:"));
    if exit.stdout.contains("tmux available: true") {
        assert!(exit.stdout.contains("manager panel opened: true"));
        assert!(exit.stdout.contains("status strip rendered: true"));
        assert!(exit.stdout.contains("status pane command checked: true"));
        assert!(exit.stdout.contains("status feedback checked: true"));
        assert!(exit.stdout.contains("manager panel rendered: true"));
        assert!(exit.stdout.contains("manager state checked: true"));
        assert!(exit.stdout.contains("target pane detail checked: true"));
        assert!(exit.stdout.contains("current pane marker checked: true"));
        assert!(
            exit.stdout
                .contains("current target row markers checked: true")
        );
        assert!(
            exit.stdout
                .contains("startup manager after shell prompt checked: true")
        );
        assert!(exit.stdout.contains("confirmation path checked: true"));
        assert!(exit.stdout.contains("cancellation feedback checked: true"));
        assert!(exit.stdout.contains("destructive shortcut checked: true"));
        assert!(exit.stdout.contains("unavailable shortcut blocked: true"));
        assert!(exit.stdout.contains("no-server start hint checked: true"));
        assert!(exit.stdout.contains("outside attach hint checked: true"));
        assert!(exit.stdout.contains("mouse focus checked: true"));
        assert!(exit.stdout.contains("mouse action selection checked: true"));
        assert!(
            exit.stdout
                .contains("mouse workspace selection checked: true")
        );
        assert!(exit.stdout.contains("refresh shortcut requested: true"));
        assert!(exit.stdout.contains("refresh focus preserved: true"));
        assert!(exit.stdout.contains("help catalog checked: true"));
        assert!(
            exit.stdout
                .contains("help catalog action coverage checked: true")
        );
        assert!(exit.stdout.contains("new window shortcut checked: true"));
        assert!(exit.stdout.contains("window cycle shortcuts checked: true"));
        assert!(exit.stdout.contains("zoom shortcut checked: true"));
        assert!(exit.stdout.contains("zoom toggle shortcut checked: true"));
        assert!(exit.stdout.contains("select pane shortcut checked: true"));
        assert!(exit.stdout.contains("split right shortcut checked: true"));
        assert!(exit.stdout.contains("split down shortcut checked: true"));
        assert!(exit.stdout.contains("safe action dispatched: true"));
        assert!(
            exit.stdout
                .contains("attach session pty handoff checked: true")
        );
        assert!(
            exit.stdout
                .contains("skipped pty handoffs checked: attach=true start=true workspace=true")
        );
        assert!(
            exit.stdout
                .contains("detach session failure feedback checked: true")
        );
        assert!(
            exit.stdout
                .contains("detach session shortcut checked: true")
        );
        assert!(exit.stdout.contains("tmux missing feedback checked: true"));
        assert!(
            exit.stdout
                .contains("rename window action dispatched: true")
        );
        assert!(exit.stdout.contains("rename window feedback checked: true"));
        assert!(
            exit.stdout
                .contains("rename session action dispatched: true")
        );
        assert!(
            exit.stdout
                .contains("rename session feedback checked: true")
        );
        assert!(
            exit.stdout
                .contains("kill pane confirmation dispatched: true")
        );
        assert!(
            exit.stdout
                .contains("kill window confirmation dispatched: true")
        );
        assert!(
            exit.stdout
                .contains("kill session confirmation dispatched: true")
        );
        assert!(exit.stdout.contains("name entry action dispatched: true"));
        assert!(
            exit.stdout
                .contains("start session pty handoff checked: true")
        );
        assert!(exit.stdout.contains("start session feedback checked: true"));
        assert!(exit.stdout.contains("workspace launch: started"));
        assert!(exit.stdout.contains("workspace feedback checked: true"));
        assert!(
            exit.stdout
                .contains("workspace command hints checked: true")
        );
        assert!(
            exit.stdout
                .contains("workspace existing attach checked: true")
        );
        assert!(
            exit.stdout
                .contains("workspace failure feedback checked: true")
        );
        assert!(
            exit.stdout
                .contains("workspace invalid preflight checked: true")
        );
        assert!(exit.stdout.contains("workspace duplicate prevented: true"));
        assert!(exit.stdout.contains("state reader observed session: true"));
        assert!(exit.stdout.contains("cleanup killed session: true"));
    } else {
        assert!(exit.stdout.contains("skipped: tmux not found on PATH"));
    }
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
}

#[test]
fn tmux_assist_cli_lists_actions_with_commands_keys_and_safety() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--tmux-assist"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("tmux assist"));
    assert!(exit.stdout.contains("tmux installed:"));
    assert!(exit.stdout.contains("inside tmux:"));
    assert!(exit.stdout.contains("Split pane right"));
    assert!(exit.stdout.contains("tmux command: tmux split-window -h"));
    assert!(exit.stdout.contains("tmux key: Ctrl-b %"));
    assert!(exit.stdout.contains("Kill session"));
    assert!(exit.stdout.contains("confirmation required: true"));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
}

#[test]
fn tmux_action_cli_requires_confirmation_before_destructive_execution() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(
        ["gromaq", "--tmux-action", "kill-session", "alpha"],
        &backend,
    );

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("tmux action: confirmation required"));
    assert!(exit.stdout.contains("action: kill-session"));
    assert!(
        exit.stdout
            .contains("tmux command: tmux kill-session -t <session>")
    );
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
}

#[test]
fn tmux_action_cli_rejects_unknown_action_id() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--tmux-action", "wat"], &backend);

    assert_eq!(exit.code, 2);
    assert!(exit.stdout.is_empty());
    assert!(exit.stderr.contains("unknown tmux action: wat"));
    assert!(backend.requests.borrow().is_empty());
}

#[test]
fn tmux_manager_cli_reports_state_or_clean_absence() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--tmux-manager"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("tmux manager"));
    assert!(exit.stdout.contains("tmux installed:"));
    assert!(exit.stdout.contains("sessions:"));
    assert!(exit.stdout.contains("windows:"));
    assert!(exit.stdout.contains("panes:"));
    assert!(exit.stdout.contains("manager action: attach-session"));
    assert!(exit.stdout.contains("manager action: kill-session"));
    assert!(exit.stdout.contains("run: gromaq --tmux-action"));
    assert!(exit.stdout.contains("tmux command: tmux split-window -h"));
    assert!(exit.stdout.contains("tmux key: Ctrl-b %"));
    assert!(exit.stdout.contains("destructive: true"));
    assert!(exit.stdout.contains("confirmation required: true"));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
}
