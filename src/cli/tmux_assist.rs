//! Native tmux assist action catalog output.

use crate::cli::CliExit;
use crate::tmux::{SystemTmuxCommandRunner, TmuxAction, TmuxProbe};

pub(super) fn tmux_assist_exit() -> CliExit {
    CliExit {
        code: 0,
        stdout: render_tmux_assist(),
        stderr: String::new(),
    }
}

fn render_tmux_assist() -> String {
    let probe = TmuxProbe::new(SystemTmuxCommandRunner).probe();
    let mut output = String::from("tmux assist\n");
    match probe {
        Ok(status) => {
            output.push_str(&format!("tmux installed: {}\n", status.installed));
            let version = status
                .version
                .map(|version| version.raw)
                .unwrap_or_else(|| "unavailable".to_owned());
            output.push_str(&format!("tmux version: {version}\n"));
            output.push_str(&format!("inside tmux: {}\n", status.inside_tmux));
            output.push_str(&format!(
                "attachable sessions: {}\n",
                status.attachable_sessions
            ));
        }
        Err(error) => {
            output.push_str("tmux installed: unknown\n");
            output.push_str(&format!("tmux probe error: {error:?}\n"));
        }
    }
    output.push_str("actions:\n");
    for action in TmuxAction::registry() {
        output.push_str(&format!("{}\n", action.label));
        output.push_str(&format!("  id: {}\n", action.stable_id));
        output.push_str(&format!("  {}\n", action.description));
        output.push_str(&format!("  tmux command: {}\n", action.tmux_command));
        if let Some(key) = action.key_binding {
            output.push_str(&format!("  tmux key: {key}\n"));
        }
        output.push_str(&format!("  destructive: {}\n", action.destructive));
        output.push_str(&format!(
            "  confirmation required: {}\n",
            action.confirmation_required
        ));
        output.push_str(&format!(
            "  requires active tmux: {}\n",
            action.requires_active_tmux
        ));
        output.push_str(&format!(
            "  can run outside tmux: {}\n",
            action.can_run_outside_tmux
        ));
    }
    output
}
