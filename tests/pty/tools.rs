use std::time::Duration;

use crate::support::*;

#[test]
fn pty_session_runs_ssh_version_when_available() {
    assert_program_outputs_when_available("ssh", &["-V"], "OpenSSH");
}

#[test]
fn pty_session_runs_ssh_config_dump_when_available() {
    assert_program_outputs_when_available("ssh", &["-G", "localhost"], "hostname localhost");
}

#[test]
fn pty_session_runs_kubectl_client_version_when_available() {
    assert_program_outputs_when_available(
        "kubectl",
        &["version", "--client=true", "--output=yaml"],
        "clientVersion",
    );
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
