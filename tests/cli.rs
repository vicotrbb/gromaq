use std::cell::RefCell;
use std::fs;

use gromaq::app::NativeAppConfig;
use gromaq::cli::{
    AdapterReport, CliExit, NativeAppLaunchError, NativeAppLauncher, run_with_backend,
    run_with_backend_and_app, run_with_backend_and_clipboard,
};
use gromaq::native_gpu::{
    GpuAdapterSnapshot, GpuBootstrapBackend, GpuBootstrapError, GpuBootstrapRequest,
    GpuGlyphAtlasUploadReport, GpuGlyphAtlasUploadRunner, GpuSmokeReport, GpuSmokeRunner,
    GpuTerminalTextReport, GpuTerminalTextRunner, GpuTextAtlasUploadReport,
    GpuTextAtlasUploadRunner, GpuTextureUploadReport, GpuTextureUploadRunner,
    GpuTexturedQuadReport, GpuTexturedQuadRunner,
};
use gromaq::{HostClipboard, MemoryClipboard};

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
    launches: RefCell<Vec<NativeAppConfig>>,
}

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
    fn launch(&self, config: NativeAppConfig) -> Result<(), NativeAppLaunchError> {
        self.launches.borrow_mut().push(config);
        Ok(())
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
            quads: 3,
            rasterized_glyphs: 2,
            reused_glyphs: 1,
            drawn_pixels: 96,
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

    assert_eq!(
        exit,
        CliExit {
            code: 2,
            stdout: String::new(),
            stderr: "usage: gromaq [--gpu-info|--gpu-smoke|--gpu-upload-smoke|--gpu-glyph-atlas-smoke|--gpu-text-atlas-smoke|--gpu-textured-quad-smoke|--gpu-terminal-text-smoke|--clipboard-smoke|--config-check <path>|--osc52-clipboard-smoke|--runtime-clipboard-paste-smoke|--runtime-glyph-frame-smoke|--runtime-perf-smoke|--runtime-large-output-smoke|--runtime-bounded-state-smoke|--runtime-alternate-screen-smoke|--runtime-reflow-smoke|--runtime-idle-smoke|--frame-scheduler-smoke]\nunknown argument: --wat\n".to_owned(),
        }
    );
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
        r#"
        [terminal]
        cols = 96
        rows = 32
        scrollback_lines = 2048

        [font]
        family = "Gromaq Mono"
        size_px = 16.5

        [performance]
        target_fps = 120
        dirty_region_rendering = true
        "#,
    )
    .unwrap();

    let path_arg = path.to_string_lossy().into_owned();
    let exit = run_with_backend(["gromaq", "--config-check", path_arg.as_str()], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("config check: ok"));
    assert!(exit.stdout.contains("terminal: 96x32"));
    assert!(exit.stdout.contains("scrollback lines: 2048"));
    assert!(exit.stdout.contains("font: Gromaq Mono 16.5px"));
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
fn runtime_clipboard_paste_smoke_cli_routes_clipboard_text_to_runtime_pty() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let mut clipboard = MemoryClipboard::new("previous clipboard");

    let exit = run_with_backend_and_clipboard(
        ["gromaq", "--runtime-clipboard-paste-smoke"],
        &backend,
        &mut clipboard,
    );

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("runtime clipboard paste smoke: ok"));
    assert!(exit.stdout.contains("pasted bytes: 30"));
    assert!(exit.stdout.contains("clipboard pastes: 1"));
    assert!(exit.stdout.contains("previous text restored: true"));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
    assert_eq!(clipboard.read_text().as_deref(), Some("previous clipboard"));
}

#[test]
fn runtime_glyph_frame_smoke_cli_reports_prepared_frame_without_gpu_bootstrap() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--runtime-glyph-frame-smoke"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("runtime glyph frame smoke: ok"));
    assert!(exit.stdout.contains("pumped bytes: 19"));
    assert!(exit.stdout.contains("planned glyphs:"));
    assert!(exit.stdout.contains("renderer atlas hits:"));
    assert!(exit.stdout.contains("renderer atlas misses:"));
    assert!(exit.stdout.contains("renderer atlas entries:"));
    assert!(exit.stdout.contains("rasterized glyphs:"));
    assert!(exit.stdout.contains("prepared quads:"));
    assert!(exit.stdout.contains("atlas bytes:"));
    assert!(exit.stdout.contains("frame size:"));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
}

#[test]
fn runtime_large_output_smoke_cli_reports_rendered_burst_without_gpu_bootstrap() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--runtime-large-output-smoke"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("runtime large-output smoke: ok"));
    assert!(exit.stdout.contains("lines: 512"));
    assert!(exit.stdout.contains("pumped bytes:"));
    assert!(exit.stdout.contains("scrollback lines: 128"));
    assert!(exit.stdout.contains("rendered frames: 1"));
    assert!(
        exit.stdout
            .contains("last visible line: gromaq-runtime-line-511")
    );
    assert!(exit.stdout.contains("render p95 ns:"));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
}

#[test]
fn runtime_bounded_state_smoke_cli_reports_capped_long_session_without_gpu_bootstrap() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--runtime-bounded-state-smoke"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("runtime bounded-state smoke: ok"));
    assert!(exit.stdout.contains("batches: 4"));
    assert!(exit.stdout.contains("lines: 2048"));
    assert!(exit.stdout.contains("pumped bytes:"));
    assert!(exit.stdout.contains("scrollback cap: 128"));
    assert!(exit.stdout.contains("scrollback lines: 128"));
    assert!(exit.stdout.contains("rendered frames: 4"));
    assert!(
        exit.stdout
            .contains("last visible line: gromaq-bounded-line-2047")
    );
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
}

#[test]
fn runtime_alternate_screen_smoke_cli_reports_restored_primary_without_gpu_bootstrap() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--runtime-alternate-screen-smoke"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("runtime alternate-screen smoke: ok"));
    assert!(exit.stdout.contains("stages: 3"));
    assert!(exit.stdout.contains("pumped bytes:"));
    assert!(exit.stdout.contains("primary restored: true"));
    assert!(exit.stdout.contains("alternate rendered: true"));
    assert!(
        exit.stdout
            .contains("alternate scrollback suppressed: true")
    );
    assert!(exit.stdout.contains("rendered frames: 3"));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
}

#[test]
fn runtime_reflow_smoke_cli_reports_resize_reflow_without_gpu_bootstrap() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--runtime-reflow-smoke"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("runtime reflow smoke: ok"));
    assert!(exit.stdout.contains("pumped bytes:"));
    assert!(exit.stdout.contains("resize events: 1"));
    assert!(exit.stdout.contains("scrollback lines: 2"));
    assert!(exit.stdout.contains("visible lines: klmno|pqrst"));
    assert!(exit.stdout.contains("rendered frames: 1"));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
}

#[test]
fn runtime_idle_smoke_cli_reports_clean_frame_suppression_without_gpu_bootstrap() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--runtime-idle-smoke"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("runtime idle smoke: ok"));
    assert!(exit.stdout.contains("pumped bytes: 0"));
    assert!(exit.stdout.contains("render attempts: 16"));
    assert!(exit.stdout.contains("clean frame skips: 16"));
    assert!(exit.stdout.contains("rendered frames: 0"));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
}

#[test]
fn runtime_perf_smoke_cli_reports_structured_metrics_without_gpu_bootstrap() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--runtime-perf-smoke"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("runtime perf smoke: ok"));
    assert!(exit.stdout.contains("pumped bytes: 1"));
    assert!(exit.stdout.contains("rendered frames: 1"));
    assert!(exit.stdout.contains("render p95 ns:"));
    assert!(exit.stdout.contains("input-to-render p95 ns:"));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
}

#[test]
fn clipboard_smoke_cli_roundtrips_without_gpu_bootstrap() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let mut clipboard = MemoryClipboard::new("previous clipboard");

    let exit =
        run_with_backend_and_clipboard(["gromaq", "--clipboard-smoke"], &backend, &mut clipboard);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("clipboard smoke: ok"));
    assert!(exit.stdout.contains("roundtrip bytes: 22"));
    assert!(exit.stdout.contains("previous text restored: true"));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
    assert_eq!(clipboard.read_text().as_deref(), Some("previous clipboard"));
}

#[test]
fn clipboard_smoke_cli_clears_sentinel_without_previous_text() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let mut clipboard = MemoryClipboard::default();

    let exit =
        run_with_backend_and_clipboard(["gromaq", "--clipboard-smoke"], &backend, &mut clipboard);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("clipboard smoke: ok"));
    assert!(exit.stdout.contains("previous text restored: false"));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
    assert_eq!(clipboard.read_text().as_deref(), Some(""));
}

#[test]
fn clipboard_smoke_cli_clears_stale_sentinel_text() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let mut clipboard = MemoryClipboard::new("gromaq clipboard smoke");

    let exit =
        run_with_backend_and_clipboard(["gromaq", "--clipboard-smoke"], &backend, &mut clipboard);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("clipboard smoke: ok"));
    assert!(exit.stdout.contains("previous text restored: false"));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
    assert_eq!(clipboard.read_text().as_deref(), Some(""));
}

#[test]
fn clipboard_smoke_cli_reports_readback_mismatch() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let mut clipboard = ReadOnlyClipboard {
        text: "unchanged".to_owned(),
    };

    let exit =
        run_with_backend_and_clipboard(["gromaq", "--clipboard-smoke"], &backend, &mut clipboard);

    assert_eq!(exit.code, 1);
    assert!(exit.stdout.is_empty());
    assert!(exit.stderr.contains(
        "clipboard smoke failed: expected \"gromaq clipboard smoke\", read \"unchanged\""
    ));
    assert!(backend.requests.borrow().is_empty());
}

#[test]
fn osc52_clipboard_smoke_cli_decodes_and_writes_without_gpu_bootstrap() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let mut clipboard = MemoryClipboard::new("previous clipboard");

    let exit = run_with_backend_and_clipboard(
        ["gromaq", "--osc52-clipboard-smoke"],
        &backend,
        &mut clipboard,
    );

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("OSC 52 clipboard smoke: ok"));
    assert!(exit.stdout.contains("decoded bytes: 18"));
    assert!(exit.stdout.contains("previous text restored: true"));
    assert!(exit.stderr.is_empty());
    assert!(backend.requests.borrow().is_empty());
    assert_eq!(clipboard.read_text().as_deref(), Some("previous clipboard"));
}

#[test]
fn osc52_clipboard_smoke_cli_reports_readback_mismatch() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let mut clipboard = ReadOnlyClipboard {
        text: "unchanged".to_owned(),
    };

    let exit = run_with_backend_and_clipboard(
        ["gromaq", "--osc52-clipboard-smoke"],
        &backend,
        &mut clipboard,
    );

    assert_eq!(exit.code, 1);
    assert!(exit.stdout.is_empty());
    assert!(exit.stderr.contains(
        "OSC 52 clipboard smoke failed: expected clipboard text \"gromaq osc52 smoke\", read \"unchanged\""
    ));
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
    assert_eq!(app.launches.borrow()[0], NativeAppConfig::default());
}

fn test_cli_config_path(name: &str) -> std::path::PathBuf {
    let directory = std::env::current_dir()
        .unwrap()
        .join("target")
        .join("gromaq-cli-tests");
    fs::create_dir_all(&directory).unwrap();
    directory.join(format!("{}-{name}", std::process::id()))
}

#[test]
fn gpu_text_atlas_smoke_cli_reports_font_backed_atlas_upload_result() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--gpu-text-atlas-smoke"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("GPU text atlas smoke: ok"));
    assert!(exit.stdout.contains("32x18"));
    assert!(exit.stdout.contains("occupied slots: 2"));
    assert!(exit.stdout.contains("rasterized glyphs: 2"));
    assert!(exit.stdout.contains("reused glyphs: 1"));
    assert!(exit.stdout.contains("covered pixels: 96"));
    assert!(exit.stdout.contains("matching bytes: 2304/2304"));
    assert!(exit.stderr.is_empty());
    assert_eq!(backend.requests.borrow().len(), 1);
}

#[test]
fn gpu_textured_quad_smoke_cli_reports_draw_result() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--gpu-textured-quad-smoke"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("GPU textured quad smoke: ok"));
    assert!(exit.stdout.contains("4x4"));
    assert!(exit.stdout.contains("first pixel: [255, 0, 0, 255]"));
    assert!(exit.stdout.contains("drawn pixels: 16"));
    assert!(exit.stderr.is_empty());
    assert_eq!(backend.requests.borrow().len(), 1);
}

#[test]
fn gpu_terminal_text_smoke_cli_reports_draw_result() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--gpu-terminal-text-smoke"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("GPU terminal text smoke: ok"));
    assert!(exit.stdout.contains("glyphs: 3"));
    assert!(exit.stdout.contains("quads: 3"));
    assert!(exit.stdout.contains("drawn pixels: 96"));
    assert!(exit.stderr.is_empty());
    assert_eq!(backend.requests.borrow().len(), 1);
}

#[test]
fn gpu_smoke_cli_reports_readback_result() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--gpu-smoke"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("GPU smoke: ok"));
    assert!(exit.stdout.contains("4x4"));
    assert!(exit.stdout.contains("first pixel: [26, 51, 76, 255]"));
    assert!(exit.stderr.is_empty());
    assert_eq!(backend.requests.borrow().len(), 1);
}

#[test]
fn gpu_glyph_atlas_smoke_cli_reports_atlas_upload_result() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--gpu-glyph-atlas-smoke"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("GPU glyph atlas smoke: ok"));
    assert!(exit.stdout.contains("4x2"));
    assert!(exit.stdout.contains("occupied slots: 2"));
    assert!(exit.stdout.contains("matching bytes: 32/32"));
    assert!(exit.stderr.is_empty());
    assert_eq!(backend.requests.borrow().len(), 1);
}

#[test]
fn gpu_upload_smoke_cli_reports_upload_readback_result() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };

    let exit = run_with_backend(["gromaq", "--gpu-upload-smoke"], &backend);

    assert_eq!(exit.code, 0);
    assert!(exit.stdout.contains("GPU upload smoke: ok"));
    assert!(exit.stdout.contains("2x2"));
    assert!(exit.stdout.contains("first pixel: [255, 0, 0, 255]"));
    assert!(exit.stdout.contains("matching bytes: 16/16"));
    assert!(exit.stderr.is_empty());
    assert_eq!(backend.requests.borrow().len(), 1);
}
