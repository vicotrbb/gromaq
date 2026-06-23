use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::process::Command;
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

#[test]
fn pty_session_runs_bash_command_when_available() {
    assert_shell_command_outputs("bash", "gromaq-bash");
}

#[test]
fn pty_session_runs_bash_interactive_workflow_when_available() {
    assert_interactive_shell_outputs_when_available(
        "bash",
        b"printf 'gromaq-bash-interactive\\n'\nexit\n",
        "gromaq-bash-interactive",
    );
}

#[test]
fn pty_session_runs_zsh_command_when_available() {
    assert_shell_command_outputs("zsh", "gromaq-zsh");
}

#[test]
fn pty_session_runs_zsh_interactive_workflow_when_available() {
    assert_interactive_shell_outputs_when_available(
        "zsh",
        b"printf 'gromaq-zsh-interactive\\n'\nexit\n",
        "gromaq-zsh-interactive",
    );
}

#[test]
fn pty_session_runs_fish_command_when_available() {
    assert_program_outputs_when_available("fish", &["-c", "printf gromaq-fish"], "gromaq-fish");
}

#[test]
fn pty_session_runs_fish_interactive_workflow_when_available() {
    assert_interactive_shell_outputs_when_available(
        "fish",
        b"printf 'gromaq-fish-interactive\\n'\nexit\n",
        "gromaq-fish-interactive",
    );
}

#[test]
fn pty_session_runs_nushell_command_when_available() {
    assert_program_outputs_when_available("nu", &["-c", "print gromaq-nushell"], "gromaq-nushell");
}

#[test]
fn pty_session_runs_nushell_interactive_workflow_when_available() {
    assert_interactive_shell_outputs_when_available(
        "nu",
        b"print gromaq-nushell-interactive\nexit\n",
        "gromaq-nushell-interactive",
    );
}

#[test]
fn pty_session_runs_vim_version_when_available() {
    assert_program_outputs_when_available("vim", &["--version"], "VIM");
}

#[test]
fn pty_session_runs_vim_interactive_edit_when_available() {
    let Some(_program) = find_program("vim") else {
        eprintln!("skipping vim interactive PTY workflow test because vim is not on PATH");
        return;
    };
    let file = test_temp_path("vim-interactive.txt");
    let _ = fs::remove_file(&file);
    let command = format!(
        "TERM=xterm-256color vim -Nu NONE -n -i NONE -N --noplugin {}",
        shell_quote_path(&file)
    );
    let mut session = spawn_shell_pty_command(command);
    session.start_output_reader().unwrap();
    drain_until_any_output(&mut session, 50, Duration::from_millis(20));

    session
        .write_all(b"igromaq-vim-interactive\x1b:wq\r")
        .unwrap();

    assert!(
        session
            .wait_timeout(Duration::from_secs(5))
            .unwrap()
            .is_some()
    );
    let edited = fs::read_to_string(&file).unwrap();
    assert_eq!(edited, "gromaq-vim-interactive\n");
    let _ = fs::remove_file(file);
}

#[test]
fn pty_session_runs_vim_mouse_window_selection_when_available() {
    let Some(_program) = find_program("vim") else {
        eprintln!("skipping vim mouse PTY workflow test because vim is not on PATH");
        return;
    };
    let result_file = test_temp_path("vim-mouse-window.txt");
    let _ = fs::remove_file(&result_file);
    let command = "TERM=xterm-256color vim -Nu NONE -n -i NONE -N --noplugin".to_owned();
    let mut session = spawn_shell_pty_command(command);
    session.start_output_reader().unwrap();
    drain_until_any_output(&mut session, 50, Duration::from_millis(20));

    session
        .write_all(b":set mouse=a ttymouse=sgr\r:vsplit\r:wincmd l\r")
        .unwrap();
    drain_until_any_output(&mut session, 50, Duration::from_millis(20));
    session.write_all(b"\x1b[<0;2;2M\x1b[<0;2;2m").unwrap();
    let command = format!(
        ":call writefile([string(winnr())], {})|qa!\r",
        shell_quote_path(&result_file)
    );
    session.write_all(command.as_bytes()).unwrap();

    assert!(
        session
            .wait_timeout(Duration::from_secs(5))
            .unwrap()
            .is_some()
    );
    let selected_window = fs::read_to_string(&result_file).unwrap();
    assert_eq!(selected_window.trim(), "1");
    let _ = fs::remove_file(result_file);
}

#[test]
fn pty_session_runs_nvim_version_when_available() {
    assert_program_outputs_when_available("nvim", &["--version"], "NVIM");
}

#[test]
fn pty_session_runs_nvim_interactive_edit_when_available() {
    let Some(_program) = find_program("nvim") else {
        eprintln!("skipping nvim interactive PTY workflow test because nvim is not on PATH");
        return;
    };
    let file = test_temp_path("nvim-interactive.txt");
    let _ = fs::remove_file(&file);
    let command = format!(
        "TERM=xterm-256color nvim -u NONE -n -i NONE -N --noplugin {}",
        shell_quote_path(&file)
    );
    let mut session = spawn_shell_pty_command(command);
    session.start_output_reader().unwrap();
    drain_until_any_output(&mut session, 50, Duration::from_millis(20));

    session
        .write_all(b"igromaq-nvim-interactive\x1b:wq\r")
        .unwrap();

    assert!(
        session
            .wait_timeout(Duration::from_secs(5))
            .unwrap()
            .is_some()
    );
    let edited = fs::read_to_string(&file).unwrap();
    assert_eq!(edited, "gromaq-nvim-interactive\n");
    let _ = fs::remove_file(file);
}

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

#[test]
fn pty_session_runs_large_cargo_test_output_when_available() {
    assert_program_outputs_when_available_with_timeout(
        "cargo",
        &[
            "test",
            "--manifest-path",
            "tests/fixtures/tiny_cargo_project/Cargo.toml",
            "fixture_emits_large_test_output",
            "--",
            "--nocapture",
        ],
        "gromaq-cargo-output-255",
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

fn assert_interactive_shell_outputs_when_available(shell_name: &str, input: &[u8], expected: &str) {
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

fn spawn_shell_pty_command(command: String) -> PtySession {
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

fn drain_until_contains(
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

fn drain_until_contains_stripped(
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

fn drain_until_any_output(session: &mut PtySession, attempts: usize, pause: Duration) -> Vec<u8> {
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

fn test_temp_path(name: &str) -> std::path::PathBuf {
    let directory = std::env::current_dir()
        .unwrap()
        .join("target")
        .join("gromaq-pty-tests");
    fs::create_dir_all(&directory).unwrap();
    directory.join(format!("{}-{name}", std::process::id()))
}

struct TmuxServerGuard {
    socket_name: String,
}

impl TmuxServerGuard {
    fn new(socket_name: String) -> Self {
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

fn wait_for_tmux_active_pane_index(
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

fn tmux_active_pane_index(socket_name: &str) -> String {
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

fn shell_quote_path(path: &Path) -> String {
    shell_quote(&path.to_string_lossy())
}

fn shell_quote(value: &str) -> String {
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
