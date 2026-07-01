//! Last-rendered visual surface bookkeeping.

use super::NativeTerminalRuntime;
use crate::tmux::TmuxManagerSnapshot;

pub(super) fn tmux_manager_counts(snapshot: &TmuxManagerSnapshot) -> (usize, usize, usize) {
    (
        snapshot.state.sessions.len(),
        snapshot.state.windows.len(),
        snapshot.state.panes.len(),
    )
}

impl<S> NativeTerminalRuntime<S> {
    pub(super) fn reset_last_rendered_visual_surfaces(&mut self) {
        self.last_rendered_tmux_status_strip = false;
        self.last_rendered_tmux_manager_panel = false;
        self.last_rendered_tmux_manager_sessions = 0;
        self.last_rendered_tmux_manager_windows = 0;
        self.last_rendered_tmux_manager_panes = 0;
        self.last_rendered_tmux_manager_region = None;
    }

    pub(super) fn record_rendered_tmux_manager(
        &mut self,
        counts: (usize, usize, usize),
        region: crate::DirtyRegion,
    ) {
        self.last_rendered_tmux_manager_panel = true;
        self.last_rendered_tmux_manager_sessions = counts.0;
        self.last_rendered_tmux_manager_windows = counts.1;
        self.last_rendered_tmux_manager_panes = counts.2;
        self.last_rendered_tmux_manager_region = Some(region);
    }
}
