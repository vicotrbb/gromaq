use std::cell::RefCell;
use std::ffi::OsString;
use std::fs;
use std::path::Path;

use gromaq::app::{NativeAppConfig, NativeAppRunReport, NativeTerminalRuntimeConfig};
use gromaq::cli::{
    AdapterReport, CliExit, NativeAppLaunchConfig, NativeAppLaunchError, NativeAppLauncher,
    run_with_backend, run_with_backend_and_app, run_with_backend_and_clipboard,
};
use gromaq::native_gpu::{
    GpuAdapterSnapshot, GpuBootstrapBackend, GpuBootstrapError, GpuBootstrapRequest,
    GpuGlyphAtlasUploadReport, GpuGlyphAtlasUploadRunner, GpuSmokeReport, GpuSmokeRunner,
    GpuTerminalTextPerfReport, GpuTerminalTextPerfRunner, GpuTerminalTextReport,
    GpuTerminalTextRunner, GpuTerminalTextSnapshotReport, GpuTerminalTextSnapshotRunner,
    GpuTextAtlasUploadReport, GpuTextAtlasUploadRunner, GpuTextureUploadReport,
    GpuTextureUploadRunner, GpuTexturedQuadReport, GpuTexturedQuadRunner,
};
use gromaq::renderer::RendererConfig;
use gromaq::{GromaqConfig, HostClipboard};

#[path = "cli/clipboard.rs"]
mod clipboard;
#[path = "cli/gpu_smoke.rs"]
mod gpu_smoke;
#[path = "cli/real_shell.rs"]
mod real_shell;
#[path = "cli/runtime_smoke.rs"]
mod runtime_smoke;

#[derive(Debug)]
struct MockBackend {
    requests: RefCell<Vec<GpuBootstrapRequest>>,
}

#[derive(Debug)]
struct MockContext {
    adapter: GpuAdapterSnapshot,
}

#[derive(Debug)]
struct MockAppLauncher {
    launches: RefCell<Vec<NativeAppLaunchConfig>>,
}

#[derive(Debug)]
struct NoGlyphFrameAppLauncher;

#[derive(Debug)]
struct ReadOnlyClipboard {
    text: String,
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
        self.launches.borrow_mut().push(config);
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
            glyph_frame_snapshot_written: snapshot_written,
            glyph_frame_snapshot_bytes: if snapshot_written {
                snapshot_bytes.len()
            } else {
                0
            },
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

#[test]
fn gpu_info_cli_reports_adapter_metadata() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--gpu-info"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("Mock GPU"));
    assert!(exit.stdout.contains("MockBackend"));
    assert!(exit.stderr.is_empty());
    assert_eq!(backend.requests.borrow().len(), 1);
}

#[test]
fn unknown_cli_argument_returns_usage_error() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--wat"], &backend);

    assert_eq!(exit.code, 2);
    assert!(exit.stdout.is_empty());
    assert!(exit.stderr.starts_with("usage: gromaq ["));
    assert!(
        exit.stderr
            .contains("--runtime-real-shell-command-output-smoke")
    );
    assert!(exit.stderr.ends_with("unknown argument: --wat\n"));
    assert!(backend.requests.borrow().is_empty());
}

#[test]
fn config_template_cli_prints_parseable_default_toml_without_gpu_bootstrap() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--config-template"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stderr.is_empty());
    assert!(exit.stdout.contains("[terminal]"));
    assert!(exit.stdout.contains("[shell]"));
    assert!(exit.stdout.contains("# program = \"/bin/zsh\""));
    assert!(exit.stdout.contains("[font]"));
    assert!(exit.stdout.contains("size_px = 32"));
    assert!(exit.stdout.contains("line_height_px = 44"));
    assert!(exit.stdout.contains("# cell_width_px = 18"));
    assert!(exit.stdout.contains("[theme]"));
    assert!(
        exit.stdout
            .contains("# presets: gromaq-dark, gromaq-graphite, gromaq-ghostty")
    );
    assert!(exit.stdout.contains("preset = \"gromaq-ghostty\""));
    assert!(exit.stdout.contains("selection = \"#2f3b52\""));
    assert!(exit.stdout.contains("cursor_style = \"block\""));
    assert!(exit.stdout.contains("cursor_blinking = true"));
    assert!(exit.stdout.contains("ansi = [\"#242933\", \"#ff6b7a\""));
    assert!(exit.stdout.contains("surface_padding_px = 14"));
    assert!(exit.stdout.contains("dim_opacity = 0.68"));
    assert!(exit.stdout.contains("[performance]"));
    let parsed = GromaqConfig::from_toml_str(&exit.stdout).unwrap();
    assert_eq!(parsed, GromaqConfig::default());
    assert!(backend.requests.borrow().is_empty());
}

#[test]
fn config_check_cli_validates_toml_without_gpu_bootstrap() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let path = test_cli_config_path("valid-config.toml");
    fs::write(
        &path,
        r##"
        [terminal]
        cols = 96
        rows = 32
        scrollback_lines = 2048

        [font]
        family = "Gromaq Mono"
        size_px = 16.5
        cell_width_px = 11
        line_height_px = 21

        [theme]
        preset = "gromaq-dark"
        background = "#1f2028"
        foreground = "#e8e2d6"
        cursor = "#f4c06a"
        selection = "#26364f"
        cursor_style = "underline"
        cursor_blinking = false
        surface_padding_px = 18
        dim_opacity = 0.42

        [performance]
        target_fps = 120
        dirty_region_rendering = true

        [shell]
        program = "/bin/zsh"
        args = ["-l"]
        cwd = "/tmp"
        "##,
    )
    .unwrap();

    let path_arg = path.to_string_lossy().into_owned();
    let exit = run_with_backend(["gromaq", "--config-check", path_arg.as_str()], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("config check: ok"));
    assert!(exit.stdout.contains("terminal: 96x32"));
    assert!(exit.stdout.contains("scrollback lines: 2048"));
    assert!(exit.stdout.contains("shell: /bin/zsh"));
    assert!(exit.stdout.contains("shell args: -l"));
    assert!(exit.stdout.contains("shell cwd: /tmp"));
    assert!(exit.stdout.contains("font: Gromaq Mono 16.5px"));
    assert!(
        exit.stdout
            .contains("font source: <unresolved: native runtime failed: configured font family is not installed or supported by name: Gromaq Mono; use an explicit font file path>")
    );
    assert!(exit.stdout.contains("font fallbacks: <unknown>"));
    assert!(exit.stdout.contains("cell width: 11px"));
    assert!(exit.stdout.contains("line height: 21px"));
    assert!(exit.stdout.contains("theme preset: gromaq-dark"));
    assert!(exit.stdout.contains("theme background: #1f2028"));
    assert!(exit.stdout.contains("theme foreground: #e8e2d6"));
    assert!(exit.stdout.contains("theme cursor: #f4c06a"));
    assert!(exit.stdout.contains("theme selection: #26364f"));
    assert!(exit.stdout.contains("theme cursor style: underline"));
    assert!(exit.stdout.contains("theme cursor blinking: false"));
    assert!(exit.stdout.contains("theme surface padding px: 18"));
    assert!(exit.stdout.contains("theme dim opacity: 0.42"));
    assert!(exit.stdout.contains("target fps: 120"));
    assert!(exit.stdout.contains("dirty-region rendering: true"));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
    let _ = fs::remove_file(path);
}

#[test]
fn config_check_cli_reports_invalid_config_without_gpu_bootstrap() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let path = test_cli_config_path("invalid-config.toml");
    fs::write(&path, "[performance]\ntarget_fps = 0\n").unwrap();

    let path_arg = path.to_string_lossy().into_owned();
    let exit = run_with_backend(["gromaq", "--config-check", path_arg.as_str()], &backend);

    assert_eq!(exit.code, 1);
    assert!(exit.stdout.is_empty());
    assert!(exit.stderr.contains("config check failed:"));
    assert!(exit.stderr.contains("target fps"));
    assert!(backend.requests.borrow().is_empty());
    let _ = fs::remove_file(path);
}

#[test]
fn config_check_cli_requires_path() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--config-check"], &backend);

    assert_eq!(exit.code, 2);
    assert!(exit.stdout.is_empty());
    assert!(exit.stderr.contains("missing config path"));
    assert!(backend.requests.borrow().is_empty());
}

#[test]
fn frame_scheduler_smoke_cli_reports_144hz_timeline_without_gpu_bootstrap() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--frame-scheduler-smoke"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("frame scheduler smoke: ok"));
    assert!(exit.stdout.contains("target fps: 144"));
    assert!(exit.stdout.contains("target interval ns: 6944444"));
    assert!(exit.stdout.contains("frame-paced wait ns:"));
    assert!(exit.stdout.contains("frames presented: 3"));
    assert!(exit.stdout.contains("dropped frames: 2"));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
}

#[test]
fn runtime_real_shell_command_output_smoke_preserves_output_after_prompt_redraw() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(
        ["gromaq", "--runtime-real-shell-command-output-smoke"],
        &backend,
    );

    assert_eq!(exit.code, 0);
    assert!(
        exit.stdout
            .contains("runtime real-shell command-output smoke: ok")
    );
    assert!(exit.stdout.contains("shell: /bin/sh"));
    assert!(exit.stdout.contains("command output observed: true"));
    assert!(exit.stdout.contains("prompt observed: true"));
    assert!(exit.stdout.contains("full redraw preserved output: true"));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
}

#[test]
fn runtime_text_zoom_smoke_reports_browser_style_zoom_metrics_without_gpu_bootstrap() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--runtime-text-zoom-smoke"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("runtime text zoom smoke: ok"));
    assert!(exit.stdout.contains("default font size px: 32"));
    assert!(exit.stdout.contains("default cell width px: 18"));
    assert!(exit.stdout.contains("default line height px: 44"));
    assert!(exit.stdout.contains("zoomed font size px: 37"));
    assert!(exit.stdout.contains("zoomed cell width px: 21"));
    assert!(exit.stdout.contains("zoomed line height px: 51"));
    assert!(exit.stdout.contains("zoom in reduced grid: true"));
    assert!(exit.stdout.contains("reset restored metrics: true"));
    assert!(exit.stdout.contains("reset restored grid: true"));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
}

#[test]
fn no_arguments_launches_native_terminal_app() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let app = MockAppLauncher {
        launches: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend_and_app(["gromaq"], &backend, &app);

    assert_eq!(
        exit,
        CliExit {
            code: 0,
            stdout: String::new(),
            stderr: String::new(),
        }
    );
    assert!(backend.requests.borrow().is_empty());
    assert_eq!(app.launches.borrow().len(), 1);
    assert_eq!(app.launches.borrow()[0], NativeAppLaunchConfig::default());
}

#[test]
fn window_smoke_launches_bounded_native_terminal_app() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let app = MockAppLauncher {
        launches: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend_and_app(["gromaq", "--window-smoke"], &backend, &app);

    assert_eq!(
        exit,
        CliExit {
            code: 0,
            stdout: "window smoke: ok\npresented frame limit: 1\nredraw attempts: 1\nsurface timeouts: 0\nsurface occluded: 0\n".to_owned(),
            stderr: String::new(),
        }
    );
    assert!(backend.requests.borrow().is_empty());
    assert_eq!(app.launches.borrow().len(), 1);
    let launch = &app.launches.borrow()[0];
    assert_eq!(launch.app.exit_after_presented_frames, Some(1));
    assert_eq!(launch.app.exit_after_redraw_attempts, Some(16));
    assert!(launch.app.redraw_until_presented_frame_limit);
    assert_eq!(launch.runtime, NativeAppLaunchConfig::default().runtime);
    assert_eq!(launch.renderer, NativeAppLaunchConfig::default().renderer);
    assert_eq!(launch.config_path, None);
}

#[test]
fn window_smoke_fails_when_no_surface_frame_is_presented() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let app = NoGlyphFrameAppLauncher;

    let exit = run_with_backend_and_app(["gromaq", "--window-smoke"], &backend, &app);

    assert_eq!(exit.code, 1);
    assert!(exit.stdout.is_empty());
    assert!(
        exit.stderr
            .contains("window smoke failed: no surface frame was presented; redraw attempts: 16; surface timeouts: 0; surface occluded: 16")
    );
    assert!(backend.requests.borrow().is_empty());
}

#[test]
fn window_perf_smoke_launches_bounded_multi_frame_native_terminal_app() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let app = MockAppLauncher {
        launches: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend_and_app(["gromaq", "--window-perf-smoke"], &backend, &app);

    assert_eq!(exit.code, 0);
    assert!(exit.stderr.is_empty());
    assert!(exit.stdout.starts_with(
        "window perf smoke: ok\npresented frame limit: 192\nredraw attempts: 192\nframes presented: 192\nsurface timeouts: 0\nsurface occluded: 0\ntarget fps: 144\nmonitor refresh mhz: 60000\nsurface present mode: Mailbox\nwindow physical size: 2560x1600\nwindow scale milliscale: 2000\nglyph frame presented: true\nglyph frame size: 2560x1600\nglyph frame glyph quads: 12\nglyph frame background quads: 1\nglyph frame decoration quads: 0\nglyph frame cursor quads: 1\nglyph frame atlas bytes: 4096\nglyph frame atlas occupied slots: 8\nframe interval target fps: 60\nframe interval target ns: 16666666\nframe interval p95 budget ns: 20000000\nframe interval warmup frames: 12\nelapsed ns: "
    ));
    assert!(exit.stdout.contains("frame interval samples: 180\n"));
    assert!(exit.stdout.contains("frame interval avg ns: 6940000\n"));
    assert!(exit.stdout.contains("frame interval max ns: 8000000\n"));
    assert!(exit.stdout.contains("frame interval max sample: 17\n"));
    assert!(exit.stdout.contains("frame interval p95 ns: 8000000\n"));
    assert!(
        exit.stdout
            .contains("frame interval p95 exact ns: 8000000\n")
    );
    assert!(exit.stdout.contains("frame intervals over target: 2\n"));
    assert!(
        exit.stdout
            .contains("frame intervals over double target: 0\n")
    );
    assert!(exit.stdout.contains("dropped frames: 1\n"));
    assert!(
        exit.stdout
            .contains("first dropped frame interval sample: 17\n")
    );
    assert!(
        exit.stdout
            .contains("last dropped frame interval sample: 17\n")
    );
    assert!(exit.stdout.contains("frame pacing accepted: false\n"));
    let _elapsed_ns = exit
        .stdout
        .lines()
        .find_map(|line| line.strip_prefix("elapsed ns: "))
        .and_then(|elapsed| elapsed.parse::<u128>().ok())
        .expect("window perf smoke should report elapsed nanoseconds");
    assert!(backend.requests.borrow().is_empty());
    assert_eq!(app.launches.borrow().len(), 1);
    let launch = &app.launches.borrow()[0];
    assert_eq!(launch.app.exit_after_presented_frames, Some(192));
    assert_eq!(launch.app.exit_after_redraw_attempts, Some(768));
    assert!(launch.app.redraw_until_presented_frame_limit);
    assert_eq!(launch.app.frame_interval_warmup_frames, 12);
    assert_eq!(launch.app.target_fps, 144);
    assert_eq!(
        launch.app.startup_text.as_deref(),
        Some("gromaq window perf smoke\nframe pacing probe\n")
    );
    assert_eq!(launch.runtime.shell.program, "/bin/sh");
    assert_eq!(
        launch.runtime.shell.args,
        vec![
            OsString::from("-lc"),
            OsString::from("printf 'gromaq window perf smoke\\nframe pacing probe\\n'")
        ]
    );
    assert_eq!(launch.renderer, NativeAppLaunchConfig::default().renderer);
    assert_eq!(launch.config_path, None);
}

#[test]
fn window_perf_smoke_fails_when_no_glyph_frame_is_presented() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let app = NoGlyphFrameAppLauncher;

    let exit = run_with_backend_and_app(["gromaq", "--window-perf-smoke"], &backend, &app);

    assert_eq!(exit.code, 1);
    assert!(exit.stdout.is_empty());
    assert!(
        exit.stderr.contains(
            "window perf smoke failed: no glyph frame was presented; redraw attempts: 768; frames presented: 0; surface timeouts: 0; surface occluded: 768"
        )
    );
    assert!(backend.requests.borrow().is_empty());
}

#[test]
fn window_glyph_frame_snapshot_smoke_writes_artifact() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let app = MockAppLauncher {
        launches: RefCell::new(Vec::new()),
    };
    let path = test_cli_config_path("window-glyph-frame.ppm");

    let exit = run_with_backend_and_app(
        [
            "gromaq",
            "--window-glyph-frame-snapshot",
            &path.to_string_lossy(),
        ],
        &backend,
        &app,
    );

    assert_eq!(exit.code, 0);
    assert!(exit.stderr.is_empty());
    assert!(exit.stdout.contains("window glyph frame snapshot: ok"));
    assert!(exit.stdout.contains("bytes written: 14"));
    assert!(exit.stdout.contains("frame size: 1x1"));
    assert!(exit.stdout.contains("glyph frame presented: true"));
    let bytes = fs::read(&path).unwrap();
    let _ = fs::remove_file(&path);
    assert_eq!(bytes, b"P6\n1 1\n255\n\x17\x1b$");
    assert!(backend.requests.borrow().is_empty());
    assert_eq!(app.launches.borrow().len(), 1);
    let launch = &app.launches.borrow()[0];
    assert_eq!(launch.app.exit_after_presented_frames, Some(60));
    assert_eq!(launch.app.exit_after_redraw_attempts, Some(60));
    assert!(launch.app.redraw_until_presented_frame_limit);
    assert_eq!(
        launch.app.glyph_frame_snapshot_path.as_deref(),
        Some(path.as_path())
    );
    assert_eq!(
        launch.app.startup_text.as_deref(),
        Some("gromaq window glyph frame snapshot\n")
    );
    assert_eq!(launch.runtime.shell.program, "/bin/sh");
    assert_eq!(
        launch.runtime.shell.args,
        vec![
            OsString::from("-lc"),
            OsString::from("printf 'gromaq window glyph frame snapshot\\n'")
        ]
    );
}

#[test]
fn window_smoke_reports_unavailable_native_app_launcher() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--window-smoke"], &backend);

    assert_eq!(exit.code, 2);
    assert!(exit.stdout.is_empty());
    assert!(
        exit.stderr
            .contains("native app launch unavailable for --window-smoke")
    );
    assert!(backend.requests.borrow().is_empty());
}

#[test]
fn window_perf_smoke_reports_unavailable_native_app_launcher() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--window-perf-smoke"], &backend);

    assert_eq!(exit.code, 2);
    assert!(exit.stdout.is_empty());
    assert!(
        exit.stderr
            .contains("native app launch unavailable for --window-perf-smoke")
    );
    assert!(backend.requests.borrow().is_empty());
}

#[test]
fn config_launch_cli_loads_config_and_launches_native_app_without_gpu_bootstrap() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let app = MockAppLauncher {
        launches: RefCell::new(Vec::new()),
    };
    let path = test_cli_config_path("launch-config.toml");
    let font_path = system_mono_font_path();
    fs::write(
        &path,
        format!(
            r#"
        [terminal]
        cols = 132
        rows = 40
        scrollback_lines = 4096

        [performance]
        target_fps = 120
        dirty_region_rendering = false

        [font]
        family = "{}"
        size_px = 16.5

        [shell]
        program = "/bin/zsh"
        args = ["-l", "-i"]
        cwd = "/tmp"
        "#,
            font_path.display()
        ),
    )
    .unwrap();

    let path_arg = path.to_string_lossy().into_owned();
    let exit = run_with_backend_and_app(["gromaq", "--config", path_arg.as_str()], &backend, &app);

    assert_eq!(
        exit,
        CliExit {
            code: 0,
            stdout: String::new(),
            stderr: String::new(),
        }
    );
    assert!(backend.requests.borrow().is_empty());
    let launches = app.launches.borrow();
    assert_eq!(launches.len(), 1);
    assert_eq!(
        launches[0].app,
        NativeAppConfig {
            target_fps: 120,
            ..NativeAppConfig::default()
        }
    );
    assert_eq!(
        launches[0].runtime,
        NativeTerminalRuntimeConfig {
            terminal_cols: 132,
            terminal_rows: 40,
            scrollback_lines: 4096,
            shell: gromaq::pty::ShellCommand {
                program: "/bin/zsh".into(),
                args: vec!["-l".into(), "-i".into()],
                cwd: Some("/tmp".into()),
            },
            ..NativeTerminalRuntimeConfig::default()
        }
    );
    assert_eq!(
        launches[0].renderer,
        RendererConfig {
            target_fps: 120,
            dirty_regions: false,
            font_size_px: 17,
            cell_width_px: 9,
            line_height_px: 44,
            ..RendererConfig::default()
        }
    );
    assert_eq!(launches[0].font_family, font_path.to_string_lossy());
    assert_eq!(launches[0].config_path.as_deref(), Some(path.as_path()));
    let _ = fs::remove_file(path);
}

#[test]
fn config_launch_cli_reports_invalid_config_without_launch_or_gpu_bootstrap() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let app = MockAppLauncher {
        launches: RefCell::new(Vec::new()),
    };
    let path = test_cli_config_path("invalid-launch-config.toml");
    fs::write(&path, "[terminal]\ncols = 0\n").unwrap();

    let path_arg = path.to_string_lossy().into_owned();
    let exit = run_with_backend_and_app(["gromaq", "--config", path_arg.as_str()], &backend, &app);

    assert_eq!(exit.code, 1);
    assert!(exit.stdout.is_empty());
    assert!(exit.stderr.contains("config launch failed:"));
    assert!(exit.stderr.contains("columns"));
    assert!(backend.requests.borrow().is_empty());
    assert!(app.launches.borrow().is_empty());
    let _ = fs::remove_file(path);
}

fn test_cli_config_path(name: &str) -> std::path::PathBuf {
    let directory = std::env::current_dir()
        .unwrap()
        .join("target")
        .join("gromaq-cli-tests");
    fs::create_dir_all(&directory).unwrap();
    directory.join(format!("{}-{name}", std::process::id()))
}

fn system_mono_font_path() -> std::path::PathBuf {
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
    .map(std::path::PathBuf::from)
    .find(|path| path.exists())
    .expect("system monospace test font is available")
}
