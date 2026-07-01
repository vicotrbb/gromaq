use gromaq::app::NativeAppRunReport;
use gromaq::cli::{NativeAppLaunchConfig, NativeAppLaunchError, NativeAppLauncher};

#[derive(Debug)]
pub(crate) struct NoGlyphFrameAppLauncher;

#[derive(Debug)]
pub(crate) struct NoTmuxUiFrameAppLauncher;

#[derive(Debug)]
pub(crate) struct DroppedFrameAppLauncher;

impl NativeAppLauncher for DroppedFrameAppLauncher {
    fn launch(
        &self,
        config: NativeAppLaunchConfig,
    ) -> Result<NativeAppRunReport, NativeAppLaunchError> {
        let frames_presented = config.app.exit_after_presented_frames.unwrap_or_default();
        let warmup_frames = config.app.frame_interval_warmup_frames;
        Ok(NativeAppRunReport {
            redraw_attempts: frames_presented,
            frames_presented,
            monitor_refresh_millihertz: Some(60_000),
            surface_present_mode: Some("Mailbox"),
            window_width_px: Some(2560),
            window_height_px: Some(1600),
            window_scale_milliscale: Some(2000),
            glyph_frame_presented: true,
            glyph_frame_width: 2560,
            glyph_frame_height: 1600,
            glyph_frame_glyph_quads: 12,
            glyph_frame_background_quads: 1,
            glyph_frame_decoration_quads: 0,
            glyph_frame_cursor_quads: 1,
            glyph_frame_atlas_bytes: 4096,
            glyph_frame_atlas_occupied_slots: 8,
            frame_interval_target_fps: 60,
            frame_interval_warmup_frames: warmup_frames,
            frame_interval_samples: frames_presented.saturating_sub(warmup_frames),
            frame_interval_avg_ns: 6_940_000,
            frame_interval_max_ns: 8_000_000,
            frame_interval_max_sample_index: 17,
            frame_interval_p95_ns: 8_000_000,
            frame_interval_p95_exact_ns: 8_000_000,
            frame_intervals_over_target: 2,
            frame_intervals_over_double_target: 0,
            dropped_frames: 1,
            first_dropped_frame_interval_sample: 17,
            last_dropped_frame_interval_sample: 17,
            ..NativeAppRunReport::default()
        })
    }
}

impl NativeAppLauncher for NoGlyphFrameAppLauncher {
    fn launch(
        &self,
        config: NativeAppLaunchConfig,
    ) -> Result<NativeAppRunReport, NativeAppLaunchError> {
        let redraw_attempts = config.app.exit_after_redraw_attempts.unwrap_or_default();
        Ok(NativeAppRunReport {
            redraw_attempts,
            frames_presented: 0,
            surface_frame_occluded: redraw_attempts,
            frame_interval_target_fps: 60,
            frame_interval_warmup_frames: config.app.frame_interval_warmup_frames,
            frame_interval_samples: 0,
            glyph_frame_presented: false,
            ..NativeAppRunReport::default()
        })
    }
}

impl NativeAppLauncher for NoTmuxUiFrameAppLauncher {
    fn launch(
        &self,
        config: NativeAppLaunchConfig,
    ) -> Result<NativeAppRunReport, NativeAppLaunchError> {
        let frames_presented = config.app.exit_after_presented_frames.unwrap_or_default();
        Ok(NativeAppRunReport {
            redraw_attempts: frames_presented,
            frames_presented,
            glyph_frame_presented: true,
            glyph_frame_width: 2560,
            glyph_frame_height: 1600,
            glyph_frame_glyph_quads: 12,
            glyph_frame_background_quads: 1,
            glyph_frame_cursor_quads: 1,
            ..NativeAppRunReport::default()
        })
    }
}
