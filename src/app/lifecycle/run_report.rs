use std::time::{Duration, Instant};

use super::report::NativeAppRunReportInput;
use super::{NativeAppLifecycle, NativeAppRunReport};

const NANOS_PER_SECOND: u64 = 1_000_000_000;

impl NativeAppLifecycle {
    /// Snapshot event-loop metrics after the native app exits.
    pub fn run_report(&self) -> NativeAppRunReport {
        self.frame_intervals.run_report(NativeAppRunReportInput {
            windows_created: self.windows_created,
            redraw_requests: self.redraw_requests,
            redraw_attempts: self.redraw_attempts,
            frames_presented: self.frames_presented,
            surface_frame_timeouts: self.surface_frame_timeouts,
            surface_frame_occluded: self.surface_frame_occluded,
            monitor_refresh_millihertz: self.monitor_refresh_millihertz,
            surface_present_mode: self.surface_present_mode,
            window_width_px: self.window_width_px,
            window_height_px: self.window_height_px,
            window_scale_milliscale: self.window_scale_milliscale,
            glyph_frame_presented: self.last_glyph_frame_presentation.glyph_frame_presented,
            glyph_frame_width: self.last_glyph_frame_presentation.width,
            glyph_frame_height: self.last_glyph_frame_presentation.height,
            glyph_frame_glyph_quads: self.last_glyph_frame_presentation.glyph_quads,
            glyph_frame_background_quads: self.last_glyph_frame_presentation.background_quads,
            glyph_frame_decoration_quads: self.last_glyph_frame_presentation.decoration_quads,
            glyph_frame_cursor_quads: self.last_glyph_frame_presentation.cursor_quads,
            glyph_frame_atlas_bytes: self.last_glyph_frame_presentation.atlas_bytes,
            glyph_frame_atlas_occupied_slots: self
                .last_glyph_frame_presentation
                .atlas_occupied_slots,
            glyph_frame_snapshot_written: self.last_glyph_frame_presentation.snapshot_written,
            glyph_frame_snapshot_bytes: self.last_glyph_frame_presentation.snapshot_bytes,
            glyph_frame_snapshot_width: self.last_glyph_frame_presentation.snapshot_width,
            glyph_frame_snapshot_height: self.last_glyph_frame_presentation.snapshot_height,
            frame_interval_target_fps: self.frame_interval_target_fps(),
            frame_interval_warmup_frames: self.config.frame_interval_warmup_frames,
        })
    }

    pub(super) fn record_frame_presented_at(
        &mut self,
        presented_at: Instant,
        presented_frame_index: u64,
    ) {
        let target_fps = self.frame_interval_target_fps();
        self.frame_intervals.record_presented_at(
            presented_at,
            target_fps,
            presented_frame_index,
            self.config.frame_interval_warmup_frames,
        );
    }

    pub(super) fn frame_interval_target_duration(&self) -> Duration {
        Duration::from_nanos(NANOS_PER_SECOND / u64::from(self.frame_interval_target_fps()))
    }

    fn frame_interval_target_fps(&self) -> u32 {
        self.monitor_refresh_millihertz
            .map(refresh_millihertz_to_fps)
            .map(|refresh_fps| refresh_fps.min(self.config.target_fps.max(1)))
            .unwrap_or_else(|| self.config.target_fps.max(1))
    }
}

fn refresh_millihertz_to_fps(refresh_millihertz: u32) -> u32 {
    refresh_millihertz
        .saturating_add(999)
        .saturating_div(1_000)
        .max(1)
}
