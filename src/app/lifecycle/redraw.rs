use std::time::Instant;

use super::{NativeAppAction, NativeAppLifecycle};

impl NativeAppLifecycle {
    /// Next timer deadline for polling PTY output without forcing a redraw.
    pub fn next_pty_pump_deadline(&self, now: Instant) -> Option<Instant> {
        if self.has_window && !self.close_requested {
            Some(now + self.frame_interval_target_duration())
        } else {
            None
        }
    }

    /// Record that a redraw was presented by the native app boundary.
    pub fn on_redraw_requested(&mut self) -> NativeAppAction {
        self.on_redraw_requested_at(Instant::now())
    }

    /// Record that a redraw was presented by the native app boundary at `presented_at`.
    pub fn on_redraw_requested_at(&mut self, presented_at: Instant) -> NativeAppAction {
        self.on_redraw_attempt_finished_at(presented_at, true)
    }

    /// Record that a redraw attempt finished, with presentation success tracked separately.
    pub fn on_redraw_attempt_finished(&mut self, frame_presented: bool) -> NativeAppAction {
        self.on_redraw_attempt_finished_at(Instant::now(), frame_presented)
    }

    /// Record that a redraw attempt finished at `finished_at`.
    pub fn on_redraw_attempt_finished_at(
        &mut self,
        finished_at: Instant,
        frame_presented: bool,
    ) -> NativeAppAction {
        self.redraw_attempts = self.redraw_attempts.saturating_add(1);
        if !frame_presented {
            return self.action_after_redraw_attempt();
        }
        let presented_frame_index = self.frames_presented.saturating_add(1);
        self.record_frame_presented_at(finished_at, presented_frame_index);
        self.frames_presented = presented_frame_index;
        let Some(limit) = self.config.exit_after_presented_frames else {
            return self.action_after_redraw_attempt();
        };
        if self.frames_presented >= limit {
            self.close_requested = true;
            NativeAppAction::Exit
        } else {
            self.action_after_redraw_attempt()
        }
    }

    fn action_after_redraw_attempt(&mut self) -> NativeAppAction {
        if self
            .config
            .exit_after_redraw_attempts
            .is_some_and(|limit| self.redraw_attempts >= limit)
        {
            self.close_requested = true;
            NativeAppAction::Exit
        } else if self.config.redraw_until_presented_frame_limit && self.has_window {
            self.redraw_requests += 1;
            NativeAppAction::RequestRedraw
        } else {
            NativeAppAction::None
        }
    }
}
