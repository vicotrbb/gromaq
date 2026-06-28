//! Real PTY smoke coverage for external tool workflows.

mod ansi;
mod output;

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::time::Duration;

use ansi::strip_ansi_sequences;
use output::render_tool_workflow_report;

use crate::cli::CliExit;
use crate::pty::{PtyConfig, PtySession, ShellCommand};

const TOOL_WORKFLOW_TIMEOUT: Duration = Duration::from_secs(5);
const TOOL_WORKFLOWS: &[ToolWorkflowSpec] = &[
    ToolWorkflowSpec {
        name: "ssh",
        args: &["-V"],
        expected: "OpenSSH",
    },
    ToolWorkflowSpec {
        name: "kubectl",
        args: &["version", "--client=true", "--output=yaml"],
        expected: "clientVersion",
    },
];

pub(super) fn runtime_tool_workflow_smoke_exit() -> CliExit {
    let report = run_tool_workflows(TOOL_WORKFLOWS);
    if let Some(failure) = report.results.iter().find_map(ToolWorkflowResult::failure) {
        return CliExit {
            code: 1,
            stdout: render_tool_workflow_report(&report),
            stderr: format!("runtime tool workflow smoke failed: {failure}\n"),
        };
    }

    CliExit {
        code: 0,
        stdout: render_tool_workflow_report(&report),
        stderr: String::new(),
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ToolWorkflowSpec {
    name: &'static str,
    args: &'static [&'static str],
    expected: &'static str,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct ToolWorkflowReport {
    pub(super) results: Vec<ToolWorkflowResult>,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum ToolWorkflowResult {
    Passed {
        name: &'static str,
        expected: &'static str,
        output_bytes: usize,
    },
    Skipped {
        name: &'static str,
    },
    Failed {
        name: &'static str,
        reason: String,
    },
}

impl ToolWorkflowResult {
    pub(super) fn failure(&self) -> Option<&str> {
        match self {
            Self::Failed { reason, .. } => Some(reason.as_str()),
            Self::Passed { .. } | Self::Skipped { .. } => None,
        }
    }
}

fn run_tool_workflows(specs: &[ToolWorkflowSpec]) -> ToolWorkflowReport {
    ToolWorkflowReport {
        results: specs.iter().map(run_tool_workflow).collect(),
    }
}

fn run_tool_workflow(spec: &ToolWorkflowSpec) -> ToolWorkflowResult {
    let Some(program) = find_program(spec.name) else {
        return ToolWorkflowResult::Skipped { name: spec.name };
    };
    match run_pty_command(&program, spec.args) {
        Ok(output) => validate_tool_output(spec, &output),
        Err(error) => ToolWorkflowResult::Failed {
            name: spec.name,
            reason: format!("{} PTY command failed: {error}", spec.name),
        },
    }
}

fn validate_tool_output(spec: &ToolWorkflowSpec, output: &str) -> ToolWorkflowResult {
    let normalized = strip_ansi_sequences(output);
    if normalized.contains(spec.expected) {
        return ToolWorkflowResult::Passed {
            name: spec.name,
            expected: spec.expected,
            output_bytes: output.len(),
        };
    }

    ToolWorkflowResult::Failed {
        name: spec.name,
        reason: format!(
            "{} PTY output did not contain {:?}: {:?}",
            spec.name, spec.expected, normalized
        ),
    }
}

fn run_pty_command(program: &OsStr, args: &[&str]) -> Result<String, String> {
    let mut session = PtySession::spawn(PtyConfig {
        rows: 24,
        cols: 80,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: program.to_owned(),
            args: args.iter().map(OsString::from).collect(),
            cwd: std::env::current_dir().ok(),
        },
    })
    .map_err(|error| error.to_string())?;
    let output = session
        .read_to_string_timeout(TOOL_WORKFLOW_TIMEOUT)
        .map_err(|error| error.to_string())?;
    match session
        .wait_timeout(Duration::from_secs(3))
        .map_err(|error| error.to_string())?
    {
        Some(_) => Ok(output),
        None => Err("PTY child did not exit before timeout".to_owned()),
    }
}

fn find_program(program: &str) -> Option<OsString> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|directory| directory.join(program))
        .find(|candidate| is_executable_file(candidate.as_path()))
        .map(PathBuf::into_os_string)
}

fn is_executable_file(path: &Path) -> bool {
    path.is_file()
}
