//! Detach shortcut proof for the native tmux UI smoke.

use std::cell::Cell;

use winit::keyboard::{Key, ModifiersState};

use crate::tmux::{
    ActionId, TmuxActionResult, TmuxCommandOutput, TmuxCommandRunner, TmuxError,
    TmuxManagerSnapshot,
};

pub(in crate::cli::runtime_tmux_smoke::ui) fn drive_detach_session_shortcut(
    snapshot: &TmuxManagerSnapshot,
) -> bool {
    let Ok(mut runtime) = super::super::smoke_runtime() else {
        return false;
    };
    runtime.open_tmux_manager_panel_with_workspaces(snapshot.clone(), Vec::new());
    let runner = DetachRunner::default();
    let requested =
        runtime.handle_tmux_manager_key(&Key::Character("d".into()), ModifiersState::empty());
    let success = matches!(
        runtime.dispatch_tmux_manager_action(requested, &runner),
        Some(TmuxActionResult::Success {
            action_id: ActionId::DetachSession,
            ..
        })
    );
    success && runner.called.get()
}

#[derive(Default)]
struct DetachRunner {
    called: Cell<bool>,
}

impl TmuxCommandRunner for DetachRunner {
    fn run_tmux(&self, args: &[&str]) -> Result<TmuxCommandOutput, TmuxError> {
        if args == ["detach-client"] {
            self.called.set(true);
            return Ok(TmuxCommandOutput::new(String::new(), String::new()));
        }
        Err(TmuxError::Parse {
            context: "runtime tmux ui detach shortcut proof",
            row: args.join(" "),
        })
    }
}
