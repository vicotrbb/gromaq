//! Native tmux manager workspace preset UI and launch helpers.

use crate::config::TmuxWorkspaceSettings;
use crate::tmux::{TmuxCommandRunner, TmuxError, TmuxWorkspaceLauncher, TmuxWorkspaceResult};

use super::state::TmuxManagerPanelState;

/// UI-facing tmux workspace preset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TmuxWorkspaceUiPreset {
    /// Config key used to identify the preset.
    pub key: String,
    /// Validated tmux workspace settings.
    pub settings: TmuxWorkspaceSettings,
}

impl TmuxWorkspaceUiPreset {
    /// Build a UI-facing workspace preset.
    pub fn new(key: impl Into<String>, settings: TmuxWorkspaceSettings) -> Self {
        Self {
            key: key.into(),
            settings,
        }
    }
}

impl TmuxManagerPanelState {
    /// Launch the selected workspace preset using the existing tmux workspace launcher.
    pub fn launch_selected_workspace<R>(
        &mut self,
        runner: &R,
    ) -> Option<Result<TmuxWorkspaceResult, TmuxError>>
    where
        R: TmuxCommandRunner,
    {
        let preset = self.workspace_presets.get(self.selected_workspace)?.clone();
        let result =
            TmuxWorkspaceLauncher::new(runner).start_or_attach(&preset.key, &preset.settings);
        self.workspace_feedback = Some(workspace_feedback(&preset.key, &result));
        Some(result)
    }

    /// Return the latest workspace launch feedback.
    pub fn workspace_feedback(&self) -> Option<&str> {
        self.workspace_feedback.as_deref()
    }
}

pub(super) fn workspace_row(panel: &TmuxManagerPanelState) -> Option<String> {
    if panel.workspace_presets.is_empty() {
        return None;
    }
    let presets = panel
        .workspace_presets
        .iter()
        .enumerate()
        .map(|(index, preset)| workspace_summary(preset, index == panel.selected_workspace))
        .collect::<Vec<_>>()
        .join(" ");
    Some(format!("Workspaces {presets}"))
}

fn workspace_summary(preset: &TmuxWorkspaceUiPreset, selected: bool) -> String {
    let marker = if selected { "*" } else { "" };
    let root = preset.settings.root.as_deref().unwrap_or("-");
    let windows = preset
        .settings
        .windows
        .iter()
        .map(|window| format!("{}({})", window.name, window.panes.len()))
        .collect::<Vec<_>>()
        .join(" ");
    format!(
        "{}{marker} session {} root {root} windows {windows}",
        preset.key, preset.settings.session
    )
}

fn workspace_feedback(key: &str, result: &Result<TmuxWorkspaceResult, TmuxError>) -> String {
    match result {
        Ok(TmuxWorkspaceResult::Existing { session }) => {
            format!("workspace {key} found session {session}")
        }
        Ok(TmuxWorkspaceResult::Attached { session }) => {
            format!("workspace {key} attached session {session}")
        }
        Ok(TmuxWorkspaceResult::Started {
            session,
            windows,
            panes,
        }) => {
            format!("workspace {key} started session {session} windows {windows} panes {panes}")
        }
        Err(TmuxError::InvalidWorkspace { reason, .. }) => {
            format!("workspace {key} invalid: {reason}")
        }
        Err(TmuxError::Missing) => format!("workspace {key} failed: tmux missing"),
        Err(TmuxError::Command(failure)) => {
            let stderr = failure.stderr.trim();
            if stderr.is_empty() {
                return format!("workspace {key} failed");
            }
            format!("workspace {key} failed: {stderr}")
        }
        Err(error) => format!("workspace {key} failed: {error:?}"),
    }
}
