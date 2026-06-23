use std::ffi::OsString;
use std::path::Path;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;

use gromaq::pty::{PtyConfig, PtySession, ShellCommand};

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

    let mut output = Vec::new();
    for _ in 0..30 {
        output.extend(session.drain_available_output().unwrap());
        if String::from_utf8_lossy(&output).contains("gromaq-bg-reader") {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }

    assert!(String::from_utf8_lossy(&output).contains("gromaq-bg-reader"));
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

    let mut output = Vec::new();
    for _ in 0..30 {
        output.extend(session.drain_available_output().unwrap());
        if String::from_utf8_lossy(&output).contains("gromaq-wakeup") {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }

    assert!(String::from_utf8_lossy(&output).contains("gromaq-wakeup"));
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

#[test]
fn pty_session_runs_bash_command_when_available() {
    assert_shell_command_outputs("bash", "gromaq-bash");
}

#[test]
fn pty_session_runs_zsh_command_when_available() {
    assert_shell_command_outputs("zsh", "gromaq-zsh");
}

#[test]
fn pty_session_runs_fish_command_when_available() {
    assert_program_outputs_when_available("fish", &["-c", "printf gromaq-fish"], "gromaq-fish");
}

#[test]
fn pty_session_runs_nushell_command_when_available() {
    assert_program_outputs_when_available("nu", &["-c", "print gromaq-nushell"], "gromaq-nushell");
}

#[test]
fn pty_session_runs_vim_version_when_available() {
    assert_program_outputs_when_available("vim", &["--version"], "VIM");
}

#[test]
fn pty_session_runs_nvim_version_when_available() {
    assert_program_outputs_when_available("nvim", &["--version"], "NVIM");
}

#[test]
fn pty_session_runs_tmux_version_when_available() {
    assert_program_outputs_when_available("tmux", &["-V"], "tmux");
}

#[test]
fn pty_session_runs_less_version_when_available() {
    assert_program_outputs_when_available("less", &["--version"], "less");
}

#[test]
fn pty_session_runs_top_snapshot_when_available() {
    assert_program_outputs_when_available("top", top_snapshot_args(), "Processes");
}

#[test]
fn pty_session_runs_htop_version_when_available() {
    assert_program_outputs_when_available("htop", &["--version"], "htop");
}

#[test]
fn pty_session_runs_btop_version_when_available() {
    assert_program_outputs_when_available("btop", &["--version"], "btop");
}

#[test]
fn pty_session_runs_ssh_version_when_available() {
    assert_program_outputs_when_available("ssh", &["-V"], "OpenSSH");
}

#[test]
fn pty_session_runs_kubectl_client_version_when_available() {
    assert_program_outputs_when_available("kubectl", &["version", "--client=true"], "Client");
}

#[test]
fn pty_session_runs_cargo_test_workflow_when_available() {
    assert_program_outputs_when_available_with_timeout(
        "cargo",
        &[
            "test",
            "--manifest-path",
            "tests/fixtures/tiny_cargo_project/Cargo.toml",
            "--quiet",
        ],
        "test result: ok",
        Duration::from_secs(20),
    );
}

fn assert_shell_command_outputs(shell_name: &str, expected: &str) {
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

fn assert_program_outputs_when_available(program_name: &str, args: &[&str], expected: &str) {
    assert_program_outputs_when_available_with_timeout(
        program_name,
        args,
        expected,
        Duration::from_secs(5),
    );
}

fn assert_program_outputs_when_available_with_timeout(
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

fn top_snapshot_args() -> &'static [&'static str] {
    if cfg!(target_os = "macos") {
        &["-l", "1", "-n", "5"]
    } else {
        &["-b", "-n", "1"]
    }
}

fn find_program(program: &str) -> Option<OsString> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|directory| directory.join(program))
        .find(|candidate| is_executable_file(candidate.as_path()))
        .map(OsString::from)
}

fn is_executable_file(path: &Path) -> bool {
    path.is_file()
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
