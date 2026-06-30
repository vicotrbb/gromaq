//! tmux workspace preset launcher.

use crate::config::TmuxWorkspaceSettings;

use super::{SystemTmuxCommandRunner, TmuxCommandRunner, TmuxError};

/// Result of starting or attaching to a tmux workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TmuxWorkspaceResult {
    /// The workspace session already exists and was left running.
    Existing {
        /// tmux session name.
        session: String,
    },
    /// An existing workspace session was attached.
    Attached {
        /// tmux session name.
        session: String,
    },
    /// A missing workspace session was created.
    Started {
        /// tmux session name.
        session: String,
        /// Number of windows created.
        windows: usize,
        /// Number of panes requested.
        panes: usize,
    },
}

/// Starts or attaches tmux workspace presets using structured CLI args.
#[derive(Debug, Clone)]
pub struct TmuxWorkspaceLauncher<R = SystemTmuxCommandRunner> {
    runner: R,
}

impl<R> TmuxWorkspaceLauncher<R>
where
    R: TmuxCommandRunner,
{
    /// Create a workspace launcher.
    pub fn new(runner: R) -> Self {
        Self { runner }
    }

    /// Attach to an existing session or create the preset if absent.
    pub fn start_or_attach(
        &self,
        key: &str,
        workspace: &TmuxWorkspaceSettings,
    ) -> Result<TmuxWorkspaceResult, TmuxError> {
        validate_workspace(key, workspace)?;
        if self.session_exists(&workspace.session)? {
            self.runner
                .run_tmux(&["attach-session", "-t", &workspace.session])?;
            return Ok(TmuxWorkspaceResult::Attached {
                session: workspace.session.clone(),
            });
        }
        self.start_workspace(workspace)?;
        Ok(TmuxWorkspaceResult::Started {
            session: workspace.session.clone(),
            windows: workspace.windows.len(),
            panes: pane_count(workspace),
        })
    }

    /// Ensure the workspace session exists without attaching a tmux client.
    pub fn start_if_absent(
        &self,
        key: &str,
        workspace: &TmuxWorkspaceSettings,
    ) -> Result<TmuxWorkspaceResult, TmuxError> {
        validate_workspace(key, workspace)?;
        if self.session_exists(&workspace.session)? {
            return Ok(TmuxWorkspaceResult::Existing {
                session: workspace.session.clone(),
            });
        }
        self.start_workspace(workspace)?;
        Ok(TmuxWorkspaceResult::Started {
            session: workspace.session.clone(),
            windows: workspace.windows.len(),
            panes: pane_count(workspace),
        })
    }

    fn session_exists(&self, session: &str) -> Result<bool, TmuxError> {
        match self.runner.run_tmux(&["has-session", "-t", session]) {
            Ok(_) => Ok(true),
            Err(TmuxError::Command(_)) => Ok(false),
            Err(error) => Err(error),
        }
    }

    fn start_workspace(&self, workspace: &TmuxWorkspaceSettings) -> Result<(), TmuxError> {
        let first = &workspace.windows[0];
        let mut args = vec![
            "new-session".to_owned(),
            "-d".to_owned(),
            "-s".to_owned(),
            workspace.session.clone(),
            "-n".to_owned(),
            first.name.clone(),
        ];
        push_root(&mut args, workspace.root.as_deref());
        args.push(first.panes[0].clone());
        self.run_owned(args)?;

        for pane in first.panes.iter().skip(1) {
            self.run_owned(split_args(
                &workspace.session,
                0,
                workspace.root.as_deref(),
                pane,
            ))?;
        }
        for (index, window) in workspace.windows.iter().enumerate().skip(1) {
            let mut args = vec![
                "new-window".to_owned(),
                "-t".to_owned(),
                workspace.session.clone(),
                "-n".to_owned(),
                window.name.clone(),
            ];
            push_root(&mut args, workspace.root.as_deref());
            args.push(window.panes[0].clone());
            self.run_owned(args)?;
            for pane in window.panes.iter().skip(1) {
                self.run_owned(split_args(
                    &workspace.session,
                    index,
                    workspace.root.as_deref(),
                    pane,
                ))?;
            }
        }
        Ok(())
    }

    fn run_owned(&self, args: Vec<String>) -> Result<(), TmuxError> {
        let refs = args.iter().map(String::as_str).collect::<Vec<_>>();
        self.runner.run_tmux(&refs).map(|_| ())
    }
}

fn validate_workspace(key: &str, workspace: &TmuxWorkspaceSettings) -> Result<(), TmuxError> {
    if workspace.session.trim().is_empty() {
        return invalid_workspace(key, "session is empty");
    }
    if workspace.windows.is_empty() {
        return invalid_workspace(key, "windows are empty");
    }
    for window in &workspace.windows {
        if window.name.trim().is_empty() {
            return invalid_workspace(key, "window name is empty");
        }
        if window.panes.iter().any(|pane| pane.trim().is_empty()) || window.panes.is_empty() {
            return invalid_workspace(key, "pane command is empty");
        }
    }
    Ok(())
}

fn invalid_workspace<T>(workspace: &str, reason: &'static str) -> Result<T, TmuxError> {
    Err(TmuxError::InvalidWorkspace {
        workspace: workspace.to_owned(),
        reason,
    })
}

fn split_args(session: &str, window_index: usize, root: Option<&str>, pane: &str) -> Vec<String> {
    let mut args = vec![
        "split-window".to_owned(),
        "-t".to_owned(),
        format!("{session}:{window_index}"),
    ];
    push_root(&mut args, root);
    args.push(pane.to_owned());
    args
}

fn push_root(args: &mut Vec<String>, root: Option<&str>) {
    if let Some(root) = root {
        args.extend(["-c".to_owned(), root.to_owned()]);
    }
}

fn pane_count(workspace: &TmuxWorkspaceSettings) -> usize {
    workspace
        .windows
        .iter()
        .map(|window| window.panes.len())
        .sum()
}
