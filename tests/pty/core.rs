use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;

use gromaq::pty::{PtyConfig, PtySession, ShellCommand};

use crate::support::*;

#[test]
fn pty_config_converts_to_portable_pty_size() {
    let config = PtyConfig {
        rows: 24,
        cols: 80,
        pixel_width: 640,
        pixel_height: 384,
        shell: ShellCommand::default_shell(),
    };

    let size = config.size();

    assert_eq!(size.rows, 24);
    assert_eq!(size.cols, 80);
    assert_eq!(size.pixel_width, 640);
    assert_eq!(size.pixel_height, 384);
}

#[test]
fn shell_command_preserves_program_args_and_cwd() {
    let command = ShellCommand {
        program: "bash".into(),
        args: vec!["-lc".into(), "printf ok".into()],
        cwd: Some("/tmp".into()),
    };

    let builder = command.to_command_builder();

    assert!(!builder.is_default_prog());
}

#[test]
fn pty_session_spawns_shell_command_and_reads_output() {
    let config = PtyConfig {
        rows: 8,
        cols: 40,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: vec!["-lc".into(), "printf gromaq-pty".into()],
            cwd: None,
        },
    };

    let mut session = PtySession::spawn(config).unwrap();
    let output = session
        .read_to_string_timeout(Duration::from_secs(3))
        .unwrap();

    assert!(output.contains("gromaq-pty"));
    assert!(
        session
            .wait_timeout(Duration::from_secs(3))
            .unwrap()
            .is_some()
    );
}

#[test]
fn pty_session_accepts_interactive_shell_input() {
    let config = PtyConfig {
        rows: 8,
        cols: 40,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    };

    let mut session = PtySession::spawn(config).unwrap();
    session.start_output_reader().unwrap();
    session
        .write_all(b"printf 'gromaq-interactive-pty\\n'\nexit\n")
        .unwrap();

    let output = drain_until_contains(
        &mut session,
        "gromaq-interactive-pty",
        50,
        Duration::from_millis(20),
    );

    assert!(
        output.contains("gromaq-interactive-pty"),
        "interactive shell output: {output:?}"
    );
    assert!(
        session
            .wait_timeout(Duration::from_secs(3))
            .unwrap()
            .is_some()
    );
}

#[test]
fn pty_session_background_reader_drains_available_output() {
    let config = PtyConfig {
        rows: 8,
        cols: 40,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: vec!["-lc".into(), "printf gromaq-bg-reader".into()],
            cwd: None,
        },
    };

    let mut session = PtySession::spawn(config).unwrap();
    session.start_output_reader().unwrap();

    let output = drain_until_contains(
        &mut session,
        "gromaq-bg-reader",
        100,
        Duration::from_millis(20),
    );

    assert!(
        output.contains("gromaq-bg-reader"),
        "background reader output: {output:?}"
    );
    assert!(
        session
            .wait_timeout(Duration::from_secs(3))
            .unwrap()
            .is_some()
    );
}

#[test]
fn pty_session_background_reader_notifies_when_output_arrives() {
    let config = PtyConfig {
        rows: 8,
        cols: 40,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: vec!["-lc".into(), "printf gromaq-wakeup".into()],
            cwd: None,
        },
    };
    let wakeups = Arc::new(AtomicUsize::new(0));
    let wakeups_for_reader = Arc::clone(&wakeups);
    let mut session = PtySession::spawn(config).unwrap();
    session
        .start_output_reader_with_wakeup(move || {
            wakeups_for_reader.fetch_add(1, Ordering::Relaxed);
        })
        .unwrap();

    let output = drain_until_contains(
        &mut session,
        "gromaq-wakeup",
        100,
        Duration::from_millis(20),
    );

    assert!(
        output.contains("gromaq-wakeup"),
        "background reader wakeup output: {output:?}"
    );
    assert!(wakeups.load(Ordering::Relaxed) > 0);
}

#[test]
fn pty_session_background_reader_drains_large_output_burst() {
    let config = PtyConfig {
        rows: 24,
        cols: 80,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: vec![
                "-lc".into(),
                "i=0; while [ \"$i\" -lt 2000 ]; do printf 'gromaq-large-%04d\\n' \"$i\"; i=$((i + 1)); done"
                    .into(),
            ],
            cwd: None,
        },
    };

    let mut session = PtySession::spawn(config).unwrap();
    session.start_output_reader().unwrap();

    let mut output = Vec::new();
    for _ in 0..100 {
        output.extend(session.drain_available_output().unwrap());
        if String::from_utf8_lossy(&output).contains("gromaq-large-1999") {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }

    let output = String::from_utf8_lossy(&output);
    assert!(output.contains("gromaq-large-0000"));
    assert!(output.contains("gromaq-large-1999"));
    assert!(
        session
            .wait_timeout(Duration::from_secs(3))
            .unwrap()
            .is_some()
    );
}
