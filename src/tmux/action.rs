//! tmux action registry and teaching metadata.

mod catalog;

use catalog::ACTIONS;

/// Stable tmux action identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionId {
    /// Start a new session.
    StartSession,
    /// Attach to an existing session.
    AttachSession,
    /// Detach from the current session.
    DetachSession,
    /// Split the active pane horizontally.
    SplitPaneRight,
    /// Split the active pane vertically.
    SplitPaneDown,
    /// Create a new window.
    NewWindow,
    /// Rename a session.
    RenameSession,
    /// Rename a window.
    RenameWindow,
    /// Move to the next window.
    NextWindow,
    /// Move to the previous window.
    PreviousWindow,
    /// Toggle pane zoom.
    ZoomPane,
    /// Select a pane.
    SelectPane,
    /// Kill a pane.
    KillPane,
    /// Kill a window.
    KillWindow,
    /// Kill a session.
    KillSession,
    /// Show tmux help.
    ShowHelp,
}

/// User-facing metadata for a tmux action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TmuxAction {
    /// Stable action id.
    pub id: ActionId,
    /// Stable string id for config and smoke output.
    pub stable_id: &'static str,
    /// Human label.
    pub label: &'static str,
    /// Short behavior description.
    pub description: &'static str,
    /// tmux command taught to the user.
    pub tmux_command: &'static str,
    /// Default tmux key binding hint when applicable.
    pub key_binding: Option<&'static str>,
    /// Whether the action destroys tmux state.
    pub destructive: bool,
    /// Whether Gromaq must ask before running it.
    pub confirmation_required: bool,
    /// Whether the action needs an active tmux session.
    pub requires_active_tmux: bool,
    /// Whether the action can be run while outside tmux.
    pub can_run_outside_tmux: bool,
}

impl TmuxAction {
    /// Return all built-in tmux actions.
    pub fn registry() -> &'static [TmuxAction] {
        ACTIONS
    }

    /// Look up an action by stable enum id.
    pub fn by_id(id: ActionId) -> Option<&'static TmuxAction> {
        ACTIONS.iter().find(|action| action.id == id)
    }

    /// Look up an action by stable string id.
    pub fn by_stable_id(stable_id: &str) -> Option<&'static TmuxAction> {
        ACTIONS.iter().find(|action| action.stable_id == stable_id)
    }
}

#[allow(clippy::too_many_arguments)]
const fn action(
    id: ActionId,
    stable_id: &'static str,
    label: &'static str,
    description: &'static str,
    tmux_command: &'static str,
    key_binding: Option<&'static str>,
    destructive: bool,
    confirmation_required: bool,
    requires_active_tmux: bool,
    can_run_outside_tmux: bool,
) -> TmuxAction {
    TmuxAction {
        id,
        stable_id,
        label,
        description,
        tmux_command,
        key_binding,
        destructive,
        confirmation_required,
        requires_active_tmux,
        can_run_outside_tmux,
    }
}
