//! Native app run reporting and presented-frame interval accounting.

mod frame_intervals;

pub(super) use frame_intervals::PresentedFrameIntervals;

/// Native app event-loop report captured after the app exits.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NativeAppRunReport {
    /// Count of native windows created during the run.
    pub windows_created: u64,
    /// Count of native redraw requests scheduled by app logic.
    pub redraw_requests: u64,
    /// Count of native redraw callbacks processed by the app boundary.
    pub redraw_attempts: u64,
    /// Count of redraw attempts that presented a native surface frame.
    pub frames_presented: u64,
    /// Count of skipped surface-frame acquisitions that timed out.
    pub surface_frame_timeouts: u64,
    /// Count of skipped surface-frame acquisitions while the surface was occluded.
    pub surface_frame_occluded: u64,
    /// Active monitor refresh rate in millihertz, if the platform reported one.
    pub monitor_refresh_millihertz: Option<u32>,
    /// Configured native surface presentation mode, if a surface was configured.
    pub surface_present_mode: Option<&'static str>,
    /// Actual native window width in physical pixels, if a window was created.
    pub window_width_px: Option<u32>,
    /// Actual native window height in physical pixels, if a window was created.
    pub window_height_px: Option<u32>,
    /// Native window scale factor multiplied by 1000, if a window was created.
    pub window_scale_milliscale: Option<u32>,
    /// Whether the last recorded presentation included a terminal glyph frame.
    pub glyph_frame_presented: bool,
    /// Width of the last presented terminal glyph frame in pixels.
    pub glyph_frame_width: u32,
    /// Height of the last presented terminal glyph frame in pixels.
    pub glyph_frame_height: u32,
    /// Textured glyph quads in the last presented terminal glyph frame.
    pub glyph_frame_glyph_quads: usize,
    /// Solid background quads in the last presented terminal glyph frame.
    pub glyph_frame_background_quads: usize,
    /// Solid text-decoration quads in the last presented terminal glyph frame.
    pub glyph_frame_decoration_quads: usize,
    /// Solid cursor quads in the last presented terminal glyph frame.
    pub glyph_frame_cursor_quads: usize,
    /// Packed glyph atlas bytes in the last presented terminal glyph frame.
    pub glyph_frame_atlas_bytes: usize,
    /// Occupied glyph atlas slots in the last presented terminal glyph frame.
    pub glyph_frame_atlas_occupied_slots: usize,
    /// Whether a prepared native glyph-frame snapshot artifact was written.
    pub glyph_frame_snapshot_written: bool,
    /// Bytes written for the prepared native glyph-frame snapshot artifact.
    pub glyph_frame_snapshot_bytes: usize,
    /// Width of the prepared native glyph-frame snapshot artifact.
    pub glyph_frame_snapshot_width: u32,
    /// Height of the prepared native glyph-frame snapshot artifact.
    pub glyph_frame_snapshot_height: u32,
    /// Effective FPS target used for presented-frame interval accounting.
    pub frame_interval_target_fps: u32,
    /// Number of initial presented frames excluded from interval metrics.
    pub frame_interval_warmup_frames: u64,
    /// Count of measured intervals between presented frames.
    pub frame_interval_samples: u64,
    /// Total measured presented-frame interval duration in nanoseconds.
    pub frame_interval_total_ns: u64,
    /// Average measured presented-frame interval duration in nanoseconds.
    pub frame_interval_avg_ns: u64,
    /// Maximum measured presented-frame interval duration in nanoseconds.
    pub frame_interval_max_ns: u64,
    /// One-based sample index where the maximum presented-frame interval was observed.
    pub frame_interval_max_sample_index: u64,
    /// Approximate p95 presented-frame interval in nanoseconds, using fixed buckets.
    pub frame_interval_p95_ns: u64,
    /// Exact p95 presented-frame interval in nanoseconds when all intervals fit in telemetry.
    pub frame_interval_p95_exact_ns: u64,
    /// Count of measured intervals that exceeded the effective target frame interval.
    pub frame_intervals_over_target: u64,
    /// Count of measured intervals that exceeded twice the effective target frame interval.
    pub frame_intervals_over_double_target: u64,
    /// Number of target frame intervals missed between presented frames.
    pub dropped_frames: u64,
    /// First one-based interval sample index that missed at least one target frame.
    pub first_dropped_frame_interval_sample: u64,
    /// Last one-based interval sample index that missed at least one target frame.
    pub last_dropped_frame_interval_sample: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct NativeAppRunReportInput {
    pub(super) windows_created: u64,
    pub(super) redraw_requests: u64,
    pub(super) redraw_attempts: u64,
    pub(super) frames_presented: u64,
    pub(super) surface_frame_timeouts: u64,
    pub(super) surface_frame_occluded: u64,
    pub(super) monitor_refresh_millihertz: Option<u32>,
    pub(super) surface_present_mode: Option<&'static str>,
    pub(super) window_width_px: Option<u32>,
    pub(super) window_height_px: Option<u32>,
    pub(super) window_scale_milliscale: Option<u32>,
    pub(super) glyph_frame_presented: bool,
    pub(super) glyph_frame_width: u32,
    pub(super) glyph_frame_height: u32,
    pub(super) glyph_frame_glyph_quads: usize,
    pub(super) glyph_frame_background_quads: usize,
    pub(super) glyph_frame_decoration_quads: usize,
    pub(super) glyph_frame_cursor_quads: usize,
    pub(super) glyph_frame_atlas_bytes: usize,
    pub(super) glyph_frame_atlas_occupied_slots: usize,
    pub(super) glyph_frame_snapshot_written: bool,
    pub(super) glyph_frame_snapshot_bytes: usize,
    pub(super) glyph_frame_snapshot_width: u32,
    pub(super) glyph_frame_snapshot_height: u32,
    pub(super) frame_interval_target_fps: u32,
    pub(super) frame_interval_warmup_frames: u64,
}
