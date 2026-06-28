use super::ToolWorkflowSpec;

#[cfg(target_os = "macos")]
const TOP_SNAPSHOT_ARGS: &[&str] = &["-l", "1", "-n", "5"];
#[cfg(not(target_os = "macos"))]
const TOP_SNAPSHOT_ARGS: &[&str] = &["-b", "-n", "1"];

#[cfg(target_os = "macos")]
const TOP_SNAPSHOT_EXPECTED: &str = "Processes";
#[cfg(not(target_os = "macos"))]
const TOP_SNAPSHOT_EXPECTED: &str = "Tasks";

pub(super) const TOOL_WORKFLOWS: &[ToolWorkflowSpec] = &[
    ToolWorkflowSpec {
        name: "fish-version",
        program: "fish",
        args: &["--version"],
        expected: "fish",
    },
    ToolWorkflowSpec {
        name: "vim-version",
        program: "vim",
        args: &["--version"],
        expected: "VIM",
    },
    ToolWorkflowSpec {
        name: "nvim-version",
        program: "nvim",
        args: &["--version"],
        expected: "NVIM",
    },
    ToolWorkflowSpec {
        name: "tmux-version",
        program: "tmux",
        args: &["-V"],
        expected: "tmux",
    },
    ToolWorkflowSpec {
        name: "less-version",
        program: "less",
        args: &["--version"],
        expected: "less",
    },
    ToolWorkflowSpec {
        name: "top-snapshot",
        program: "top",
        args: TOP_SNAPSHOT_ARGS,
        expected: TOP_SNAPSHOT_EXPECTED,
    },
    ToolWorkflowSpec {
        name: "htop-version",
        program: "htop",
        args: &["--version"],
        expected: "htop",
    },
    ToolWorkflowSpec {
        name: "btop-version",
        program: "btop",
        args: &["--version"],
        expected: "btop",
    },
    ToolWorkflowSpec {
        name: "ssh-version",
        program: "ssh",
        args: &["-V"],
        expected: "OpenSSH",
    },
    ToolWorkflowSpec {
        name: "ssh-config",
        program: "ssh",
        args: &["-G", "localhost"],
        expected: "hostname localhost",
    },
    ToolWorkflowSpec {
        name: "kubectl-version",
        program: "kubectl",
        args: &["version", "--client=true", "--output=yaml"],
        expected: "clientVersion",
    },
    ToolWorkflowSpec {
        name: "kubectl-config",
        program: "kubectl",
        args: &["config", "view", "--output=jsonpath={.kind}"],
        expected: "Config",
    },
];
