//! Native tmux discovery, teaching metadata, and management primitives.

mod action;
mod action_runner;
mod error;
mod manager;
mod probe;
mod reader;
mod runner;
mod state;
mod workspace;

pub use action::{ActionId, TmuxAction};
pub use action_runner::{TmuxActionRequest, TmuxActionResult, TmuxActionRunner};
pub use error::TmuxError;
pub use manager::{TmuxManager, TmuxManagerCurrent, TmuxManagerSnapshot};
pub use probe::{TmuxProbe, TmuxProbeStatus, TmuxVersion};
pub use reader::TmuxStateReader;
pub use runner::{
    SocketTmuxCommandRunner, SystemTmuxCommandRunner, TmuxCommandFailure, TmuxCommandOutput,
    TmuxCommandRunner,
};
pub use state::{TmuxPane, TmuxSession, TmuxState, TmuxTarget, TmuxWindow};
pub use workspace::{TmuxWorkspaceLauncher, TmuxWorkspaceResult};
