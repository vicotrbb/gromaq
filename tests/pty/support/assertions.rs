use std::ffi::OsString;
use std::time::Duration;

use gromaq::pty::{PtyConfig, PtySession, ShellCommand};

use super::{
    drain_until_any_output, drain_until_contains, drain_until_contains_stripped, find_program,
    strip_ansi_sequences,
};

pub(crate) fn assert_shell_command_outputs(shell_name: &str, expected: &str) {
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

pub(crate) fn assert_interactive_shell_outputs_when_available(
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

pub(crate) fn assert_program_outputs_when_available(
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

pub(crate) fn assert_program_outputs_when_available_with_timeout(
    program_name: &str,
    args: &[&str],
    expected: &str,
    timeout: Duration,
) {
    assert_program_outputs_any_when_available_with_timeout(
        program_name,
        args,
        &[expected],
        timeout,
    );
}

pub(crate) fn assert_program_outputs_any_when_available(
    program_name: &str,
    args: &[&str],
    expected_any: &[&str],
) {
    assert_program_outputs_any_when_available_with_timeout(
        program_name,
        args,
        expected_any,
        Duration::from_secs(5),
    );
}

pub(crate) fn assert_program_outputs_any_when_available_with_timeout(
    program_name: &str,
    args: &[&str],
    expected_any: &[&str],
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
        output_contains_any(&normalized_output, expected_any),
        "{program_name} output did not contain any of {expected_any:?}: {output:?}"
    );
    assert!(
        session
            .wait_timeout(Duration::from_secs(3))
            .unwrap()
            .is_some()
    );
}

pub(crate) fn output_contains_any(output: &str, expected_any: &[&str]) -> bool {
    expected_any
        .iter()
        .any(|expected| output.contains(expected))
}

#[cfg(test)]
mod tests {
    use super::output_contains_any;

    #[test]
    fn output_contains_any_accepts_linux_top_snapshot_header() {
        let output = "top - 04:48:43 up 1 min\r\nTasks: 193 total\r\n";

        assert!(output_contains_any(output, &["Processes", "Tasks"]));
    }
}

pub(crate) fn assert_shell_command_enters_and_leaves_alternate_screen_when_available(
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

pub(crate) fn spawn_shell_pty_command(command: String) -> PtySession {
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
