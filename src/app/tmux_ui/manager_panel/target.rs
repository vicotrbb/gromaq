//! Current target and pane labels for the native tmux manager panel.

use crate::tmux::{TmuxManagerSnapshot, TmuxPane};

pub(super) fn current_target_label(snapshot: &TmuxManagerSnapshot) -> String {
    let Some(current) = snapshot.current.as_ref() else {
        return "none".to_owned();
    };
    let mut label = format!(
        "{}:{}:{}",
        current.session_name, current.window_index, current.pane_id
    );
    if let Some(pane) = snapshot.state.panes.iter().find(|pane| {
        pane.session_name == current.session_name
            && pane.window_index == current.window_index
            && pane.id == current.pane_id
    }) {
        let command = pane_command_label(pane);
        let dimensions = pane_dimensions(pane);
        if !command.is_empty() || !dimensions.is_empty() {
            label.push_str(" | pane ");
            label.push_str(&command);
            label.push_str(&dimensions);
        }
    }
    label
}

pub(super) fn pane_command_label(pane: &TmuxPane) -> String {
    if pane.title.is_empty() || pane.title == pane.current_command {
        pane.current_command.clone()
    } else {
        format!("{}:{}", pane.title, pane.current_command)
    }
}

pub(super) fn pane_dimensions(pane: &TmuxPane) -> String {
    match (pane.width, pane.height) {
        (Some(width), Some(height)) => format!(" {width}x{height}"),
        _ => String::new(),
    }
}

pub(super) fn is_current_pane(snapshot: &TmuxManagerSnapshot, pane: &TmuxPane) -> bool {
    snapshot.current.as_ref().is_some_and(|current| {
        current.session_name == pane.session_name
            && current.window_index == pane.window_index
            && current.pane_id == pane.id
    })
}
