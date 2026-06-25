//! Native app lifecycle state.

use std::time::Instant;

use super::NativeGlyphFramePresentation;
use crate::renderer::SurfaceFrameError;

mod action;
mod config;
mod event;
mod report;
mod run_report;

pub use action::NativeAppAction;
pub use config::NativeAppConfig;
pub use event::{NativeAppEvent, NativeAppEventProxy};
pub use report::NativeAppRunReport;

use report::PresentedFrameIntervals;

/// Testable native app lifecycle state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeAppLifecycle {
    config: NativeAppConfig,
    has_window: bool,
    close_requested: bool,
    windows_created: u64,
    redraw_requests: u64,
    redraw_attempts: u64,
    frames_presented: u64,
    surface_frame_timeouts: u64,
    surface_frame_occluded: u64,
    monitor_refresh_millihertz: Option<u32>,
    surface_present_mode: Option<&'static str>,
    window_width_px: Option<u32>,
    window_height_px: Option<u32>,
    window_scale_milliscale: Option<u32>,
    last_glyph_frame_presentation: NativeGlyphFramePresentation,
    frame_intervals: PresentedFrameIntervals,
}

impl NativeAppLifecycle {
    /// Create lifecycle state for a native app configuration.
    pub fn new(config: NativeAppConfig) -> Self {
        Self {
            config,
            has_window: false,
            close_requested: false,
            windows_created: 0,
            redraw_requests: 0,
            redraw_attempts: 0,
            frames_presented: 0,
            surface_frame_timeouts: 0,
            surface_frame_occluded: 0,
            monitor_refresh_millihertz: None,
            surface_present_mode: None,
            window_width_px: None,
            window_height_px: None,
            window_scale_milliscale: None,
            last_glyph_frame_presentation: NativeGlyphFramePresentation::default(),
            frame_intervals: PresentedFrameIntervals::default(),
        }
    }

    /// Access native app configuration.
    pub fn config(&self) -> &NativeAppConfig {
        &self.config
    }

    /// Apply native app settings that can change without recreating the window.
    pub fn apply_config(&mut self, config: NativeAppConfig) {
        self.config = config;
    }

    /// Handle a platform resume notification.
    pub fn on_resumed(&mut self) -> NativeAppAction {
        if self.has_window {
            NativeAppAction::None
        } else {
            NativeAppAction::CreateWindow
        }
    }

    /// Record that the native window was created.
    pub fn on_window_created(&mut self) {
        self.on_window_created_with_monitor_refresh(None);
    }

    /// Record that the native window was created on a monitor with a known refresh rate.
    pub fn on_window_created_with_monitor_refresh(
        &mut self,
        monitor_refresh_millihertz: Option<u32>,
    ) {
        self.on_window_created_with_surface_report(monitor_refresh_millihertz, None);
    }

    /// Record that the native window was created with known monitor/surface metadata.
    pub fn on_window_created_with_surface_report(
        &mut self,
        monitor_refresh_millihertz: Option<u32>,
        surface_present_mode: Option<&'static str>,
    ) {
        self.on_window_created_with_full_report(
            monitor_refresh_millihertz,
            surface_present_mode,
            None,
            None,
            None,
        );
    }

    /// Record that the native window was created with known monitor, surface, and window metadata.
    pub fn on_window_created_with_full_report(
        &mut self,
        monitor_refresh_millihertz: Option<u32>,
        surface_present_mode: Option<&'static str>,
        window_width_px: Option<u32>,
        window_height_px: Option<u32>,
        window_scale_milliscale: Option<u32>,
    ) {
        self.has_window = true;
        self.windows_created += 1;
        self.monitor_refresh_millihertz = monitor_refresh_millihertz;
        self.surface_present_mode = surface_present_mode;
        self.window_width_px = window_width_px;
        self.window_height_px = window_height_px;
        self.window_scale_milliscale = window_scale_milliscale;
    }

    /// Handle the event-loop idle boundary before waiting for more events.
    pub fn on_about_to_wait(&mut self) -> NativeAppAction {
        self.on_about_to_wait_at(Instant::now())
    }

    /// Handle the event-loop idle boundary at a deterministic instant.
    pub fn on_about_to_wait_at(&mut self, _now: Instant) -> NativeAppAction {
        if self.close_requested {
            NativeAppAction::Exit
        } else {
            NativeAppAction::None
        }
    }

    /// Record that terminal output changed the grid and a redraw should be scheduled.
    pub fn on_terminal_output_ready(&mut self) -> NativeAppAction {
        if self.has_window && !self.close_requested {
            self.redraw_requests += 1;
            NativeAppAction::RequestRedraw
        } else if self.close_requested {
            NativeAppAction::Exit
        } else {
            NativeAppAction::None
        }
    }

    /// Handle a native event-loop user event.
    pub fn on_user_event(&mut self, event: NativeAppEvent) -> NativeAppAction {
        match event {
            NativeAppEvent::PtyOutputReady => self.on_terminal_output_ready(),
        }
    }

    /// Record the latest native terminal glyph-frame presentation metrics.
    pub fn record_glyph_frame_presentation(&mut self, report: NativeGlyphFramePresentation) {
        if report.glyph_frame_presented || report.snapshot_written {
            self.last_glyph_frame_presentation = report;
        }
    }

    /// Record a skipped surface-frame acquisition outcome.
    pub fn record_surface_frame_skip(&mut self, error: SurfaceFrameError) {
        match error {
            SurfaceFrameError::Timeout => {
                self.surface_frame_timeouts = self.surface_frame_timeouts.saturating_add(1);
            }
            SurfaceFrameError::Occluded => {
                self.surface_frame_occluded = self.surface_frame_occluded.saturating_add(1);
            }
            SurfaceFrameError::Outdated
            | SurfaceFrameError::Lost
            | SurfaceFrameError::Validation
            | SurfaceFrameError::InvalidFrame(_) => {}
        }
    }

    /// Next timer deadline for polling PTY output without forcing a redraw.
    pub fn next_pty_pump_deadline(&self, now: Instant) -> Option<Instant> {
        if self.has_window && !self.close_requested {
            Some(now + self.frame_interval_target_duration())
        } else {
            None
        }
    }

    /// Record that the native window requested application shutdown.
    pub fn on_close_requested(&mut self) -> NativeAppAction {
        self.close_requested = true;
        NativeAppAction::Exit
    }

    /// Record that the native window was destroyed.
    pub fn on_destroyed(&mut self) -> NativeAppAction {
        self.has_window = false;
        NativeAppAction::Exit
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

    /// Whether the lifecycle currently owns a native window.
    pub fn has_window(&self) -> bool {
        self.has_window
    }

    /// Whether shutdown was requested.
    pub fn close_requested(&self) -> bool {
        self.close_requested
    }

    /// Count of native windows created by this lifecycle.
    pub fn windows_created(&self) -> u64 {
        self.windows_created
    }

    /// Count of redraw requests scheduled by this lifecycle.
    pub fn redraw_requests(&self) -> u64 {
        self.redraw_requests
    }

    /// Count of redraw callbacks processed by this lifecycle.
    pub fn redraw_attempts(&self) -> u64 {
        self.redraw_attempts
    }

    /// Count of redraw events observed by this lifecycle.
    pub fn frames_presented(&self) -> u64 {
        self.frames_presented
    }

    /// Count of skipped redraw attempts caused by surface acquisition timeouts.
    pub fn surface_frame_timeouts(&self) -> u64 {
        self.surface_frame_timeouts
    }

    /// Count of skipped redraw attempts caused by the surface being occluded.
    pub fn surface_frame_occluded(&self) -> u64 {
        self.surface_frame_occluded
    }
}
