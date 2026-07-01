//! Run-report materialization for presented-frame intervals.

use super::PresentedFrameIntervals;
use crate::app::lifecycle::report::{NativeAppRunReport, NativeAppRunReportInput};
use crate::app::perf::percentile_rank;

impl PresentedFrameIntervals {
    pub(in crate::app::lifecycle) fn run_report(
        self,
        input: NativeAppRunReportInput,
    ) -> NativeAppRunReport {
        let frame_interval_p95_ns = self.histogram.p95_upper_bound_ns(self.samples);
        let frame_interval_p95_exact_ns = self.exact_p95_ns();
        NativeAppRunReport {
            windows_created: input.windows_created,
            redraw_requests: input.redraw_requests,
            redraw_attempts: input.redraw_attempts,
            frames_presented: input.frames_presented,
            surface_frame_timeouts: input.surface_frame_timeouts,
            surface_frame_occluded: input.surface_frame_occluded,
            monitor_refresh_millihertz: input.monitor_refresh_millihertz,
            surface_present_mode: input.surface_present_mode,
            window_width_px: input.window_width_px,
            window_height_px: input.window_height_px,
            window_scale_milliscale: input.window_scale_milliscale,
            glyph_frame_presented: input.glyph_frame_presented,
            tmux_status_strip_rendered: input.tmux_status_strip_rendered,
            tmux_status_pane_command_rendered: input.tmux_status_pane_command_rendered,
            tmux_manager_panel_rendered: input.tmux_manager_panel_rendered,
            default_startup_content_checked: input.default_startup_content_checked,
            tmux_manager_sessions: input.tmux_manager_sessions,
            tmux_manager_windows: input.tmux_manager_windows,
            tmux_manager_panes: input.tmux_manager_panes,
            terminal_cols: input.terminal_cols,
            terminal_rows: input.terminal_rows,
            glyph_frame_width: input.glyph_frame_width,
            glyph_frame_height: input.glyph_frame_height,
            glyph_frame_glyph_quads: input.glyph_frame_glyph_quads,
            glyph_frame_background_quads: input.glyph_frame_background_quads,
            glyph_frame_decoration_quads: input.glyph_frame_decoration_quads,
            glyph_frame_cursor_quads: input.glyph_frame_cursor_quads,
            glyph_frame_atlas_bytes: input.glyph_frame_atlas_bytes,
            glyph_frame_atlas_occupied_slots: input.glyph_frame_atlas_occupied_slots,
            glyph_frame_snapshot_written: input.glyph_frame_snapshot_written,
            glyph_frame_snapshot_bytes: input.glyph_frame_snapshot_bytes,
            glyph_frame_snapshot_width: input.glyph_frame_snapshot_width,
            glyph_frame_snapshot_height: input.glyph_frame_snapshot_height,
            frame_interval_target_fps: input.frame_interval_target_fps,
            frame_interval_warmup_frames: input.frame_interval_warmup_frames,
            frame_interval_samples: self.samples,
            frame_interval_total_ns: self.total_ns,
            frame_interval_avg_ns: self.avg_ns,
            frame_interval_max_ns: self.max_ns,
            frame_interval_max_sample_index: self.max_sample_index,
            frame_interval_p95_ns,
            frame_interval_p95_exact_ns,
            frame_intervals_over_target: self.intervals_over_target,
            frame_intervals_over_double_target: self.intervals_over_double_target,
            dropped_frames: self.dropped_frames,
            first_dropped_frame_interval_sample: self.first_dropped_frame_interval_sample,
            last_dropped_frame_interval_sample: self.last_dropped_frame_interval_sample,
        }
    }

    fn exact_p95_ns(&self) -> u64 {
        if self.samples == 0 || usize::try_from(self.samples).ok() != Some(self.interval_sample_len)
        {
            return 0;
        }
        let mut samples = self.interval_samples_ns;
        samples[..self.interval_sample_len].sort_unstable();
        let rank = percentile_rank(self.samples, 95).saturating_sub(1);
        let index = usize::try_from(rank).unwrap_or(self.interval_sample_len - 1);
        samples[index]
    }
}
