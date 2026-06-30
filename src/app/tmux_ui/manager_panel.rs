//! Native tmux manager panel state and rendering helpers.

mod actions;
mod input;
mod rendering;
mod selection;
mod state;
mod workspaces;

pub use input::TmuxManagerKeyOutcome;
pub use rendering::apply_tmux_manager_panel;
pub use state::{TmuxManagerFocus, TmuxManagerPanelState};
pub use workspaces::TmuxWorkspaceUiPreset;
