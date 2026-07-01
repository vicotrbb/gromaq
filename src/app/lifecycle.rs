//! Native app lifecycle state.

use std::time::Instant;

use super::NativeGlyphFramePresentation;
use crate::renderer::SurfaceFrameError;

mod action;
mod config;
mod event;
mod redraw;
mod report;
mod run_report;
mod window;

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
            self.last_glyph_frame_presentation = self
                .last_glyph_frame_presentation
                .with_preserved_snapshot(report);
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
