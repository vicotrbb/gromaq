use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};

use gromaq::HostClipboard;
use gromaq::app::NativeAppRunReport;
use gromaq::cli::{AdapterReport, NativeAppLaunchConfig, NativeAppLaunchError, NativeAppLauncher};
use gromaq::native_gpu::{
    GpuAdapterSnapshot, GpuBootstrapBackend, GpuBootstrapError, GpuBootstrapRequest,
    GpuGlyphAtlasUploadReport, GpuGlyphAtlasUploadRunner, GpuSmokeReport, GpuSmokeRunner,
    GpuTerminalTextPerfReport, GpuTerminalTextPerfRunner, GpuTerminalTextReport,
    GpuTerminalTextRunner, GpuTerminalTextSnapshotReport, GpuTerminalTextSnapshotRunner,
    GpuTextAtlasUploadReport, GpuTextAtlasUploadRunner, GpuTextureUploadReport,
    GpuTextureUploadRunner, GpuTexturedQuadReport, GpuTexturedQuadRunner,
};
#[path = "support/gpu_welcome_image.rs"]
mod gpu_welcome_image;

#[derive(Debug)]
pub(crate) struct MockBackend {
    pub(crate) requests: RefCell<Vec<GpuBootstrapRequest>>,
}
#[derive(Debug)]
pub(crate) struct MockContext {
    adapter: GpuAdapterSnapshot,
}
#[derive(Debug)]
pub(crate) struct MockAppLauncher {
    pub(crate) launches: RefCell<Vec<NativeAppLaunchConfig>>,
}
#[derive(Debug)]
pub(crate) struct ReadOnlyClipboard {
    pub(crate) text: String,
}

impl GpuBootstrapBackend for MockBackend {
    type Context = MockContext;

    fn request_device(
        &self,
        request: &GpuBootstrapRequest,
    ) -> Result<Self::Context, GpuBootstrapError> {
        self.requests.borrow_mut().push(request.clone());
        Ok(MockContext {
            adapter: GpuAdapterSnapshot {
                name: "Mock GPU".to_owned(),
                backend: "MockBackend".to_owned(),
                device_type: "DiscreteGpu".to_owned(),
                vendor: 1,
                device: 2,
            },
        })
    }
}

impl AdapterReport for MockContext {
    fn adapter_report(&self) -> &GpuAdapterSnapshot {
        &self.adapter
    }
}

impl NativeAppLauncher for MockAppLauncher {
    fn launch(
        &self,
        config: NativeAppLaunchConfig,
    ) -> Result<NativeAppRunReport, NativeAppLaunchError> {
        let frames_presented = config.app.exit_after_presented_frames.unwrap_or_default();
        let warmup_frames = config.app.frame_interval_warmup_frames;
        let frame_interval_samples = frames_presented.saturating_sub(warmup_frames);
        let snapshot_bytes = b"P6\n1 1\n255\n\x17\x1b$";
        let snapshot_written = match &config.app.glyph_frame_snapshot_path {
            Some(path) => {
                fs::write(path, snapshot_bytes)
                    .map_err(|error| NativeAppLaunchError::new(error.to_string()))?;
                true
            }
            None => false,
        };
        self.launches.borrow_mut().push(config.clone());
        Ok(NativeAppRunReport {
            redraw_attempts: frames_presented,
            frames_presented,
            monitor_refresh_millihertz: Some(60_000),
            surface_present_mode: Some("Mailbox"),
            window_width_px: Some(2560),
            window_height_px: Some(1600),
            window_scale_milliscale: Some(2000),
            glyph_frame_presented: true,
            tmux_status_strip_rendered: config.app.tmux_ui_enabled,
            tmux_status_pane_command_rendered: config.app.tmux_ui_enabled,
            tmux_manager_panel_rendered: config.app.open_tmux_manager_on_start,
            default_startup_content_checked: config.app.startup_text.is_none()
                && config.app.welcome_screen,
            tmux_manager_sessions: usize::from(config.app.open_tmux_manager_on_start),
            tmux_manager_windows: usize::from(config.app.open_tmux_manager_on_start),
            tmux_manager_panes: usize::from(config.app.open_tmux_manager_on_start),
            terminal_cols: 140,
            terminal_rows: 35,
            glyph_frame_width: 2560,
            glyph_frame_height: 1600,
            glyph_frame_glyph_quads: 12,
            glyph_frame_background_quads: 1,
            glyph_frame_decoration_quads: 0,
            glyph_frame_cursor_quads: 1,
            glyph_frame_atlas_bytes: 4096,
            glyph_frame_atlas_occupied_slots: 8,
            glyph_frame_snapshot_written: snapshot_written,
            glyph_frame_snapshot_bytes: usize::from(snapshot_written) * snapshot_bytes.len(),
            glyph_frame_snapshot_width: if snapshot_written { 1 } else { 0 },
            glyph_frame_snapshot_height: if snapshot_written { 1 } else { 0 },
            frame_interval_target_fps: 60,
            frame_interval_warmup_frames: warmup_frames,
            frame_interval_samples,
            frame_interval_avg_ns: 6_940_000,
            frame_interval_max_ns: 8_000_000,
            frame_interval_max_sample_index: 17,
            frame_interval_p95_ns: 8_000_000,
            frame_interval_p95_exact_ns: 8_000_000,
            frame_intervals_over_target: 0,
            frame_intervals_over_double_target: 0,
            dropped_frames: 0,
            first_dropped_frame_interval_sample: 0,
            last_dropped_frame_interval_sample: 0,
            ..NativeAppRunReport::default()
        })
    }
}

impl HostClipboard for ReadOnlyClipboard {
    fn read_text(&self) -> Option<String> {
        Some(self.text.clone())
    }

    fn write_text(&mut self, _text: &str) {}
}

impl GpuSmokeRunner for MockContext {
    fn run_smoke(&self) -> Result<GpuSmokeReport, GpuBootstrapError> {
        Ok(GpuSmokeReport {
            width: 4,
            height: 4,
            first_pixel: [26, 51, 76, 255],
            nonzero_bytes: 64,
        })
    }
}

impl GpuTextureUploadRunner for MockContext {
    fn run_texture_upload_smoke(&self) -> Result<GpuTextureUploadReport, GpuBootstrapError> {
        Ok(GpuTextureUploadReport {
            width: 2,
            height: 2,
            first_pixel: [255, 0, 0, 255],
            last_pixel: [255, 255, 255, 255],
            matching_bytes: 16,
            total_bytes: 16,
        })
    }
}

impl GpuGlyphAtlasUploadRunner for MockContext {
    fn run_glyph_atlas_upload_smoke(&self) -> Result<GpuGlyphAtlasUploadReport, GpuBootstrapError> {
        Ok(GpuGlyphAtlasUploadReport {
            width: 4,
            height: 2,
            occupied_slots: 2,
            first_pixel: [255, 0, 0, 255],
            second_slot_first_pixel: [0, 255, 0, 255],
            matching_bytes: 32,
            total_bytes: 32,
        })
    }
}

impl GpuTextAtlasUploadRunner for MockContext {
    fn run_text_atlas_upload_smoke(&self) -> Result<GpuTextAtlasUploadReport, GpuBootstrapError> {
        Ok(GpuTextAtlasUploadReport {
            width: 32,
            height: 18,
            occupied_slots: 2,
            rasterized_glyphs: 2,
            reused_glyphs: 1,
            matching_bytes: 2304,
            total_bytes: 2304,
            covered_pixels: 96,
        })
    }
}

impl GpuTexturedQuadRunner for MockContext {
    fn run_textured_quad_smoke(&self) -> Result<GpuTexturedQuadReport, GpuBootstrapError> {
        Ok(GpuTexturedQuadReport {
            width: 4,
            height: 4,
            first_pixel: [255, 0, 0, 255],
            drawn_pixels: 16,
        })
    }
}

impl GpuTerminalTextRunner for MockContext {
    fn run_terminal_text_smoke(&self) -> Result<GpuTerminalTextReport, GpuBootstrapError> {
        Ok(GpuTerminalTextReport {
            width: 80,
            height: 24,
            glyphs: 3,
            background_quads: 1,
            quads: 3,
            decoration_quads: 1,
            cursor_quads: 1,
            rasterized_glyphs: 2,
            reused_glyphs: 1,
            first_drawn_pixel: [23, 27, 36, 255],
            background_pixel: [23, 27, 36, 255],
            glyph_pixel: [237, 243, 251, 255],
            glyph_background_contrast_x100: 1544,
            cursor_pixel: [229, 229, 229, 255],
            drawn_pixels: 160,
        })
    }
}

impl GpuTerminalTextPerfRunner for MockContext {
    fn run_terminal_text_perf_smoke(&self) -> Result<GpuTerminalTextPerfReport, GpuBootstrapError> {
        Ok(GpuTerminalTextPerfReport {
            frames: 16,
            width: 80,
            height: 24,
            drawn_pixels: 160,
            min_ns: 1_000,
            avg_ns: 2_000,
            max_ns: 3_000,
            p95_ns: 3_000,
        })
    }
}

impl GpuTerminalTextSnapshotRunner for MockContext {
    fn run_terminal_text_snapshot(
        &self,
        path: &Path,
    ) -> Result<GpuTerminalTextSnapshotReport, GpuBootstrapError> {
        let snapshot = b"P6\n2 1\n255\n\x17\x1b\x24\xed\xf3\xfb";
        fs::write(path, snapshot).map_err(|error| {
            GpuBootstrapError::SmokeReadback(format!("failed to write mock snapshot: {error}"))
        })?;
        Ok(GpuTerminalTextSnapshotReport {
            width: 2,
            height: 1,
            bytes_written: snapshot.len(),
            glyphs: 2,
            background_pixel: [23, 27, 36, 255],
            glyph_pixel: [237, 243, 251, 255],
            glyph_background_contrast_x100: 1544,
            cursor_pixel: [229, 229, 229, 255],
            drawn_pixels: 2,
        })
    }
}

pub(crate) fn test_cli_config_path(name: &str) -> PathBuf {
    let directory = std::env::current_dir()
        .unwrap()
        .join("target")
        .join("gromaq-cli-tests");
    fs::create_dir_all(&directory).unwrap();
    directory.join(format!("{}-{name}", std::process::id()))
}

pub(crate) fn system_mono_font_path() -> PathBuf {
    [
        "/System/Library/Fonts/SFNSMono.ttf",
        "/System/Library/Fonts/Menlo.ttc",
        "/System/Library/Fonts/Supplemental/Courier New.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
        "/usr/share/fonts/dejavu-sans-fonts/DejaVuSansMono.ttf",
        "/usr/share/fonts/truetype/liberation2/LiberationMono-Regular.ttf",
        "/usr/share/fonts/liberation/LiberationMono-Regular.ttf",
        "/usr/share/fonts/truetype/noto/NotoSansMono-Regular.ttf",
    ]
    .into_iter()
    .map(PathBuf::from)
    .find(|path| path.exists())
    .expect("system monospace test font is available")
}
