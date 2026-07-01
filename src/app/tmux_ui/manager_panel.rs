//! Native tmux manager panel state and rendering helpers.

mod actions;
mod availability;
mod enter_action;
mod hints;
mod initial_action;
mod input;
mod rendering;
mod selection;
mod state;
mod workspaces;

pub use input::{TmuxManagerKeyOutcome, TmuxManagerMouseOutcome};
pub use rendering::apply_tmux_manager_panel;
pub use state::{TmuxManagerFocus, TmuxManagerPanelState};
pub use workspaces::TmuxWorkspaceUiPreset;
