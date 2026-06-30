//! tmux action command construction.

use super::TmuxActionRequest;
use crate::tmux::ActionId;

pub(super) fn command_args(request: &TmuxActionRequest) -> Result<Vec<String>, String> {
    let target = || {
        request
            .target
            .clone()
            .ok_or_else(|| "missing tmux target".to_owned())
    };
    let name = || {
        request
            .new_name
            .clone()
            .ok_or_else(|| "missing new name".to_owned())
    };
    let args = match request.action_id {
        ActionId::StartSession => vec!["new-session".into(), "-s".into(), target()?],
        ActionId::AttachSession => vec!["attach-session".into(), "-t".into(), target()?],
        ActionId::DetachSession => vec!["detach-client".into()],
        ActionId::SplitPaneRight => split_args("-h", request.target.clone()),
        ActionId::SplitPaneDown => split_args("-v", request.target.clone()),
        ActionId::NewWindow => new_window_args(request.target.clone(), request.new_name.clone()),
        ActionId::RenameSession => vec!["rename-session".into(), "-t".into(), target()?, name()?],
        ActionId::RenameWindow => vec!["rename-window".into(), "-t".into(), target()?, name()?],
        ActionId::NextWindow => vec!["next-window".into()],
        ActionId::PreviousWindow => vec!["previous-window".into()],
        ActionId::ZoomPane => vec!["resize-pane".into(), "-Z".into()],
        ActionId::SelectPane => vec!["select-pane".into(), "-t".into(), target()?],
        ActionId::KillPane => vec!["kill-pane".into(), "-t".into(), target()?],
        ActionId::KillWindow => vec!["kill-window".into(), "-t".into(), target()?],
        ActionId::KillSession => vec!["kill-session".into(), "-t".into(), target()?],
        ActionId::ShowHelp => vec!["list-keys".into()],
    };
    Ok(args)
}

fn split_args(direction: &str, target: Option<String>) -> Vec<String> {
    let mut args = vec!["split-window".into(), direction.into()];
    if let Some(target) = target {
        args.extend(["-t".into(), target]);
    }
    args
}

fn new_window_args(target: Option<String>, name: Option<String>) -> Vec<String> {
    let mut args = vec!["new-window".into()];
    if let Some(target) = target {
        args.extend(["-t".into(), target]);
    }
    if let Some(name) = name {
        args.extend(["-n".into(), name]);
    }
    args
}
