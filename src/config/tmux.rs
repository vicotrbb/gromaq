//! tmux configuration sections and validation.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::error::{GromaqError, Result};

/// tmux integration configuration.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct TmuxSettings {
    /// Whether tmux-native affordances are enabled.
    pub enabled: bool,
    /// Named tmux workspace presets.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub workspaces: BTreeMap<String, TmuxWorkspaceSettings>,
}

/// A project-aware tmux workspace preset.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct TmuxWorkspaceSettings {
    /// tmux session name.
    pub session: String,
    /// Optional workspace root directory.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root: Option<String>,
    /// Windows to create when starting the workspace.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub windows: Vec<TmuxWorkspaceWindowSettings>,
}

/// A tmux workspace window preset.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct TmuxWorkspaceWindowSettings {
    /// Window name.
    pub name: String,
    /// Pane shell commands to start in this window.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub panes: Vec<String>,
}

pub(super) fn validate_tmux_settings(settings: &TmuxSettings) -> Result<()> {
    for (key, workspace) in &settings.workspaces {
        validate_key(key)?;
        validate_non_empty(key, "session", &workspace.session)?;
        if let Some(root) = &workspace.root {
            validate_non_empty(key, "root", root)?;
        }
        for (window_index, window) in workspace.windows.iter().enumerate() {
            validate_window(key, window_index, window)?;
        }
    }
    Ok(())
}

fn validate_key(key: &str) -> Result<()> {
    validate_non_empty(key, "key", key)
}

fn validate_window(
    workspace: &str,
    window_index: usize,
    window: &TmuxWorkspaceWindowSettings,
) -> Result<()> {
    validate_non_empty(workspace, "windows.name", &window.name)?;
    for (pane_index, pane) in window.panes.iter().enumerate() {
        validate_non_empty(workspace, pane_field(pane_index), pane)?;
    }
    if window.panes.is_empty() {
        return Err(GromaqError::InvalidTmuxWorkspace {
            workspace: workspace.to_owned(),
            field: pane_field(window_index),
        });
    }
    Ok(())
}

fn validate_non_empty(workspace: &str, field: &'static str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(GromaqError::InvalidTmuxWorkspace {
            workspace: workspace.to_owned(),
            field,
        });
    }
    Ok(())
}

fn pane_field(index: usize) -> &'static str {
    match index {
        0 => "panes[0]",
        1 => "panes[1]",
        2 => "panes[2]",
        _ => "panes",
    }
}
