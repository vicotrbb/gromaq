use std::process::Command;
use std::time::Duration;

use crate::support::*;

#[test]
fn pty_session_runs_tmux_prefix_split_pane_when_available() {
    let Some(_program) = find_program("tmux") else {
        eprintln!("skipping tmux prefix PTY workflow test because tmux is not on PATH");
        return;
    };
    let socket_name = format!("gromaq-pty-prefix-{}", std::process::id());
    let _guard = TmuxServerGuard::new(socket_name.clone());
    let command = format!(
        "TERM=xterm-256color tmux -L {} new-session -s gromaq-pty-prefix",
        shell_quote(&socket_name)
    );
    let mut session = spawn_shell_pty_command(command);
    session.start_output_reader().unwrap();
    drain_until_any_output(&mut session, 50, Duration::from_millis(20));

    assert_eq!(
        tmux_window_pane_count(&socket_name).trim(),
        "1",
        "tmux prefix workflow must start with one pane"
    );

    session.write_all(b"\x02%").unwrap();
    let pane_count =
        wait_for_tmux_window_pane_count(&socket_name, "2", 100, Duration::from_millis(20));

    assert_eq!(
        pane_count.trim(),
        "2",
        "tmux should consume the prefix split command from the PTY"
    );
    let kill = Command::new("tmux")
        .args([
            "-L",
            &socket_name,
            "kill-session",
            "-t",
            "gromaq-pty-prefix",
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
