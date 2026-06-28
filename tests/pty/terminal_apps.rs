use std::fs;
use std::process::Command;
use std::time::Duration;

use crate::support::*;

#[test]
fn pty_session_runs_tmux_version_when_available() {
    assert_program_outputs_when_available("tmux", &["-V"], "tmux");
}

#[test]
fn pty_session_runs_tmux_interactive_pane_when_available() {
    let Some(_program) = find_program("tmux") else {
        eprintln!("skipping tmux interactive PTY workflow test because tmux is not on PATH");
        return;
    };
    let socket_name = format!("gromaq-pty-interactive-{}", std::process::id());
    let _guard = TmuxServerGuard::new(socket_name.clone());
    let command = format!(
        "TERM=xterm-256color tmux -L {} new-session -s gromaq-pty-interactive",
        shell_quote(&socket_name)
    );
    let mut session = spawn_shell_pty_command(command);
    session.start_output_reader().unwrap();
    drain_until_any_output(&mut session, 50, Duration::from_millis(20));

    session
        .write_all(b"printf 'gromaq-tmux-interactive\\n'\r")
        .unwrap();
    let output = drain_until_contains_stripped(
        &mut session,
        "gromaq-tmux-interactive",
        100,
        Duration::from_millis(20),
    );
    session.write_all(b"exit\r").unwrap();

    assert!(
        output.contains("gromaq-tmux-interactive"),
        "tmux interactive output: {output:?}"
    );
    let kill = Command::new("tmux")
        .args([
            "-L",
            &socket_name,
            "kill-session",
            "-t",
            "gromaq-pty-interactive",
        ])
        .status()
        .unwrap();
    assert!(kill.success(), "tmux kill-session failed: {kill:?}");
    assert!(
        session
            .wait_timeout(Duration::from_secs(5))
            .unwrap()
            .is_some()
    );
}

#[test]
fn pty_session_runs_tmux_mouse_pane_selection_when_available() {
    let Some(_program) = find_program("tmux") else {
        eprintln!("skipping tmux mouse PTY workflow test because tmux is not on PATH");
        return;
    };
    let socket_name = format!("gromaq-pty-mouse-{}", std::process::id());
    let _guard = TmuxServerGuard::new(socket_name.clone());
    let command = format!(
        "TERM=xterm-256color tmux -L {} new-session -d -s gromaq-pty-mouse 'sh' \\; split-window -h 'sh' \\; set-option -g mouse on \\; select-pane -t 1 \\; attach-session -t gromaq-pty-mouse",
        shell_quote(&socket_name)
    );
    let mut session = spawn_shell_pty_command(command);
    session.start_output_reader().unwrap();
    drain_until_any_output(&mut session, 50, Duration::from_millis(20));

    assert_eq!(
        tmux_active_pane_index(&socket_name).trim(),
        "1",
        "tmux mouse workflow must start with the right pane active"
    );

    session.write_all(b"\x1b[<0;2;2M\x1b[<0;2;2m").unwrap();
    let active_pane =
        wait_for_tmux_active_pane_index(&socket_name, "0", 100, Duration::from_millis(20));

    assert_eq!(
        active_pane.trim(),
        "0",
        "tmux should consume the SGR mouse click from the PTY and select the left pane"
    );
}

#[test]
fn pty_session_runs_less_version_when_available() {
    assert_program_outputs_when_available("less", &["--version"], "less");
}

#[test]
fn pty_session_runs_less_interactive_search_when_available() {
    let Some(_program) = find_program("less") else {
        eprintln!("skipping less interactive PTY workflow test because less is not on PATH");
        return;
    };
    let file = test_temp_path("less-interactive.txt");
    let lines = (0..80)
        .map(|index| format!("gromaq-less-line-{index:03}\n"))
        .collect::<String>();
    fs::write(&file, lines).unwrap();
    let command = format!("TERM=xterm-256color less -S {}", shell_quote_path(&file));
    let mut session = spawn_shell_pty_command(command);
    session.start_output_reader().unwrap();
    drain_until_any_output(&mut session, 50, Duration::from_millis(20));

    session.write_all(b"/gromaq-less-line-040\r").unwrap();
    let output = drain_until_contains_stripped(
        &mut session,
        "gromaq-less-line-040",
        100,
        Duration::from_millis(20),
    );
    session.write_all(b"q").unwrap();

    assert!(
        output.contains("gromaq-less-line-040"),
        "less interactive output: {output:?}"
    );
    assert!(
        session
            .wait_timeout(Duration::from_secs(5))
            .unwrap()
            .is_some()
    );
    let _ = fs::remove_file(file);
}

#[test]
fn pty_session_runs_less_paging_navigation_when_available() {
    let Some(_program) = find_program("less") else {
        eprintln!("skipping less paging PTY workflow test because less is not on PATH");
        return;
    };
    let file = test_temp_path("less-paging.txt");
    let lines = (0..120)
        .map(|index| format!("gromaq-less-page-line-{index:03}\n"))
        .collect::<String>();
    fs::write(&file, lines).unwrap();
    let command = format!(
        "TERM=xterm-256color LESS= less -S {}",
        shell_quote_path(&file)
    );
    let mut session = spawn_shell_pty_command(command);
    session.start_output_reader().unwrap();
    drain_until_any_output(&mut session, 50, Duration::from_millis(20));

    session.write_all(b"G").unwrap();
    let output = drain_until_contains_stripped(
        &mut session,
        "gromaq-less-page-line-119",
        100,
        Duration::from_millis(20),
    );
    session.write_all(b"q").unwrap();

    assert!(
        output.contains("gromaq-less-page-line-119"),
        "less paging output: {output:?}"
    );
    assert!(
        session
            .wait_timeout(Duration::from_secs(5))
            .unwrap()
            .is_some()
    );
    let _ = fs::remove_file(file);
}

#[test]
fn pty_session_runs_less_alternate_screen_enter_exit_when_available() {
    let Some(_program) = find_program("less") else {
        eprintln!("skipping less alternate-screen PTY workflow test because less is not on PATH");
        return;
    };
    let file = test_temp_path("less-alternate-screen.txt");
    fs::write(&file, "gromaq-less-alt-screen\nsecond line\n").unwrap();
    let command = format!(
        "TERM=xterm-256color LESS= less -S {}",
        shell_quote_path(&file)
    );
    let mut session = spawn_shell_pty_command(command);
    session.start_output_reader().unwrap();

    let enter_output =
        drain_until_contains(&mut session, "\x1b[?1049h", 100, Duration::from_millis(20));
    assert!(
        enter_output.contains("\x1b[?1049h"),
        "less did not enter alternate screen: {enter_output:?}"
    );

    session.write_all(b"q").unwrap();
    let leave_output =
        drain_until_contains(&mut session, "\x1b[?1049l", 100, Duration::from_millis(20));

    assert!(
        leave_output.contains("\x1b[?1049l"),
        "less did not leave alternate screen: {leave_output:?}"
    );
    assert!(
        session
            .wait_timeout(Duration::from_secs(5))
            .unwrap()
            .is_some()
    );
    let _ = fs::remove_file(file);
}

#[test]
fn pty_session_runs_top_snapshot_when_available() {
    assert_program_outputs_any_when_available("top", top_snapshot_args(), &["Processes", "Tasks"]);
}

#[test]
fn pty_session_runs_htop_version_when_available() {
    assert_program_outputs_when_available("htop", &["--version"], "htop");
}

#[test]
fn pty_session_runs_btop_version_when_available() {
    assert_program_outputs_when_available("btop", &["--version"], "btop");
}
