use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

use gromaq::pty::{PtyConfig, PtySession, ShellCommand};

pub(super) fn assert_shell_command_outputs(shell_name: &str, expected: &str) {
    let Some(program) = find_program(shell_name) else {
        eprintln!("skipping {shell_name} PTY workflow test because {shell_name} is not on PATH");
        return;
    };
    let config = PtyConfig {
        rows: 8,
        cols: 40,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program,
            args: vec!["-lc".into(), format!("printf {expected}").into()],
            cwd: None,
        },
    };

    let mut session = PtySession::spawn(config).unwrap();
    let output = session
        .read_to_string_timeout(Duration::from_secs(3))
        .unwrap();

    assert!(output.contains(expected), "{shell_name} output: {output:?}");
    assert!(
        session
            .wait_timeout(Duration::from_secs(3))
            .unwrap()
            .is_some()
    );
}

pub(super) fn assert_interactive_shell_outputs_when_available(
    shell_name: &str,
    input: &[u8],
    expected: &str,
) {
    let Some(program) = find_program(shell_name) else {
        eprintln!(
            "skipping {shell_name} interactive PTY workflow test because {shell_name} is not on PATH"
        );
        return;
    };
    let config = PtyConfig {
        rows: 24,
        cols: 80,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program,
            args: Vec::new(),
            cwd: Some(std::env::current_dir().unwrap()),
        },
    };

    let mut session = PtySession::spawn(config).unwrap();
    session.start_output_reader().unwrap();
    drain_until_any_output(&mut session, 50, Duration::from_millis(20));
    session.write_all(input).unwrap();
    let output =
        drain_until_contains_stripped(&mut session, expected, 100, Duration::from_millis(20));

    assert!(
        output.contains(expected),
        "{shell_name} interactive output: {output:?}"
    );
    assert!(
        session
            .wait_timeout(Duration::from_secs(5))
            .unwrap()
            .is_some()
    );
}

pub(super) fn assert_program_outputs_when_available(
    program_name: &str,
    args: &[&str],
    expected: &str,
) {
    assert_program_outputs_when_available_with_timeout(
        program_name,
        args,
        expected,
        Duration::from_secs(5),
    );
}

pub(super) fn assert_program_outputs_when_available_with_timeout(
    program_name: &str,
    args: &[&str],
    expected: &str,
    timeout: Duration,
) {
    let Some(program) = find_program(program_name) else {
        eprintln!(
            "skipping {program_name} PTY workflow test because {program_name} is not on PATH"
        );
        return;
    };
    let config = PtyConfig {
        rows: 24,
        cols: 80,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program,
            args: args.iter().map(OsString::from).collect(),
            cwd: Some(std::env::current_dir().unwrap()),
        },
    };

    let mut session = PtySession::spawn(config).unwrap();
    let output = session.read_to_string_timeout(timeout).unwrap();
    let normalized_output = strip_ansi_sequences(&output);

    assert!(
        normalized_output.contains(expected),
        "{program_name} output did not contain {expected:?}: {output:?}"
    );
    assert!(
        session
            .wait_timeout(Duration::from_secs(3))
            .unwrap()
            .is_some()
    );
}

pub(super) fn assert_shell_command_enters_and_leaves_alternate_screen_when_available(
    program_name: &str,
    command: String,
    exit_input: &[u8],
) {
    let Some(_program) = find_program(program_name) else {
        eprintln!(
            "skipping {program_name} alternate-screen PTY workflow test because {program_name} is not on PATH"
        );
        return;
    };
    let mut session = spawn_shell_pty_command(command);
    session.start_output_reader().unwrap();

    let enter_output =
        drain_until_contains(&mut session, "\x1b[?1049h", 100, Duration::from_millis(20));
    assert!(
        enter_output.contains("\x1b[?1049h"),
        "{program_name} did not enter alternate screen: {enter_output:?}"
    );

    session.write_all(exit_input).unwrap();
    let leave_output =
        drain_until_contains(&mut session, "\x1b[?1049l", 100, Duration::from_millis(20));
    assert!(
        leave_output.contains("\x1b[?1049l"),
        "{program_name} did not leave alternate screen: {leave_output:?}"
    );
    assert!(
        session
            .wait_timeout(Duration::from_secs(5))
            .unwrap()
            .is_some()
    );
}

pub(super) fn spawn_shell_pty_command(command: String) -> PtySession {
    let config = PtyConfig {
        rows: 24,
        cols: 80,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: vec!["-lc".into(), command.into()],
            cwd: Some(std::env::current_dir().unwrap()),
        },
    };

    PtySession::spawn(config).unwrap()
}

pub(super) fn drain_until_contains(
    session: &mut PtySession,
    expected: &str,
    attempts: usize,
    pause: Duration,
) -> String {
    let mut output = Vec::new();
    for _ in 0..attempts {
        output.extend(session.drain_available_output().unwrap());
        if String::from_utf8_lossy(&output).contains(expected) {
            break;
        }
        std::thread::sleep(pause);
    }
    String::from_utf8_lossy(&output).into_owned()
}

pub(super) fn drain_until_contains_stripped(
    session: &mut PtySession,
    expected: &str,
    attempts: usize,
    pause: Duration,
) -> String {
    let mut output = Vec::new();
    for _ in 0..attempts {
        output.extend(session.drain_available_output().unwrap());
        let normalized = strip_ansi_sequences(&String::from_utf8_lossy(&output));
        if normalized.contains(expected) {
            return normalized;
        }
        std::thread::sleep(pause);
    }
    strip_ansi_sequences(&String::from_utf8_lossy(&output))
}

pub(super) fn drain_until_any_output(
    session: &mut PtySession,
    attempts: usize,
    pause: Duration,
) -> Vec<u8> {
    let mut output = Vec::new();
    for _ in 0..attempts {
        output.extend(session.drain_available_output().unwrap());
        if !output.is_empty() {
            break;
        }
        std::thread::sleep(pause);
    }
    output
}

pub(super) fn top_snapshot_args() -> &'static [&'static str] {
    if cfg!(target_os = "macos") {
        &["-l", "1", "-n", "5"]
    } else {
        &["-b", "-n", "1"]
    }
}

pub(super) fn find_program(program: &str) -> Option<OsString> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|directory| directory.join(program))
        .find(|candidate| is_executable_file(candidate.as_path()))
        .map(OsString::from)
}

fn is_executable_file(path: &Path) -> bool {
    path.is_file()
}

pub(super) fn test_temp_path(name: &str) -> std::path::PathBuf {
    let directory = std::env::current_dir()
        .unwrap()
        .join("target")
        .join("gromaq-pty-tests");
    fs::create_dir_all(&directory).unwrap();
    directory.join(format!("{}-{name}", std::process::id()))
}

pub(super) struct TmuxServerGuard {
    socket_name: String,
}

impl TmuxServerGuard {
    pub(super) fn new(socket_name: String) -> Self {
        Self { socket_name }
    }
}

impl Drop for TmuxServerGuard {
    fn drop(&mut self) {
        let _ = Command::new("tmux")
            .args(["-L", &self.socket_name, "kill-server"])
            .output();
    }
}

pub(super) fn wait_for_tmux_active_pane_index(
    socket_name: &str,
    expected: &str,
    attempts: usize,
    pause: Duration,
) -> String {
    let mut active_pane = String::new();
    for _ in 0..attempts {
        active_pane = tmux_active_pane_index(socket_name);
        if active_pane.trim() == expected {
            break;
        }
        std::thread::sleep(pause);
    }
    active_pane
}

pub(super) fn tmux_active_pane_index(socket_name: &str) -> String {
    let output = Command::new("tmux")
        .args(["-L", socket_name, "display-message", "-p", "#{pane_index}"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "tmux display-message failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

pub(super) fn shell_quote_path(path: &Path) -> String {
    shell_quote(&path.to_string_lossy())
}

pub(super) fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn strip_ansi_sequences(output: &str) -> String {
    let bytes = output.as_bytes();
    let mut stripped = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] != 0x1b {
            stripped.push(bytes[index]);
            index += 1;
            continue;
        }

        index += 1;
        match bytes.get(index).copied() {
            Some(b'[') => {
                index += 1;
                while index < bytes.len() {
                    let byte = bytes[index];
                    index += 1;
                    if (0x40..=0x7e).contains(&byte) {
                        break;
                    }
                }
            }
            Some(b'(' | b')' | b'*' | b'+') => {
                index = (index + 2).min(bytes.len());
            }
            Some(_) => index += 1,
            None => {}
        }
    }

    String::from_utf8(stripped).unwrap()
}
