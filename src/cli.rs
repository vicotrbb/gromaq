//! Command-line entry points for the native application.

use std::{
    collections::VecDeque,
    fs,
    path::PathBuf,
    time::{Duration, Instant},
};

use base64::{Engine as _, engine::general_purpose};
use thiserror::Error;
use winit::keyboard::{Key, ModifiersState, NamedKey};

use crate::app::{
    NativeAppConfig, NativePtyResize, NativePtySessionIo, NativePtySpawner, NativeTerminalApp,
    NativeTerminalRuntime, NativeTerminalRuntimeConfig, is_native_paste_shortcut,
    load_default_native_glyph_cache, run_native_app_with_runtime_renderer_and_config_file,
};
use crate::clipboard::{HostClipboard, NativeClipboard};
use crate::config::{ConfigFileReloader, GromaqConfig, ShellSettings};
use crate::mouse::{MouseButton, MouseEvent, MouseEventKind};
use crate::native_gpu::{
    GpuAdapterSnapshot, GpuBootstrap, GpuBootstrapBackend, GpuBootstrapConfig, GpuBootstrapError,
    GpuGlyphAtlasUploadRunner, GpuSmokeRunner, GpuTerminalTextRunner, GpuTextAtlasUploadRunner,
    GpuTextureUploadRunner, GpuTexturedQuadRunner,
};
use crate::pty::{PtyConfig, PtyError, ShellCommand};
use crate::renderer::{
    FrameDecision, FrameScheduler, PreparedSurfaceGlyphFrame, RenderReason, RendererConfig,
    WgpuRenderer,
};
use crate::terminal::{Terminal, TerminalConfig};

const CLIPBOARD_SMOKE_TEXT: &str = "gromaq clipboard smoke";
const OSC52_CLIPBOARD_SMOKE_TEXT: &str = "gromaq osc52 smoke";
const RUNTIME_CLIPBOARD_PASTE_SMOKE_TEXT: &str = "gromaq runtime clipboard paste";
const RUNTIME_GLYPH_FRAME_SMOKE_TEXT: &str = "gromaq glyph frame";
const RUNTIME_SCROLLBACK_SMOKE_TEXT: &str = "one\r\ntwo\r\nthree\r\nfour\r\nfive\r\nsix";
const RUNTIME_OUTPUT_SMOKE_COLS: u16 = 32;
const RUNTIME_OUTPUT_SMOKE_ROWS: u16 = 8;
const RUNTIME_LARGE_OUTPUT_LINES: usize = 512;
const RUNTIME_LARGE_OUTPUT_SCROLLBACK_LINES: usize = 128;
const RUNTIME_BOUNDED_STATE_BATCHES: usize = 4;
const RUNTIME_CONTINUOUS_OUTPUT_BATCHES: usize = 32;
const RUNTIME_CONTINUOUS_OUTPUT_LINES_PER_BATCH: usize = 8;
const RUNTIME_CONTINUOUS_OUTPUT_SCROLLBACK_LINES: usize = 64;
const RUNTIME_ALTERNATE_SCREEN_SMOKE_STAGES: usize = 3;
const RUNTIME_REFLOW_SMOKE_LINK: &str = "https://gromaq.dev";
const RUNTIME_FOCUS_SMOKE_ENABLE_REPORTING: &str = "\x1b[?1004h";
const RUNTIME_MOUSE_SMOKE_ENABLE_REPORTING: &str = "\x1b[?1000h\x1b[?1006h";
const RUNTIME_RESPONSE_SMOKE_QUERIES: &str = "\x1b[3;5H\x1b[6n\x1b[5n\x1b[c\x1b[>c";
const RUNTIME_IDLE_SMOKE_RENDER_ATTEMPTS: u64 = 16;

/// Captured CLI result for tests and the binary wrapper.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliExit {
    /// Process exit code.
    pub code: i32,
    /// Standard output text.
    pub stdout: String,
    /// Standard error text.
    pub stderr: String,
}

/// Error returned by the native app launcher boundary.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("native app launch failed: {message}")]
pub struct NativeAppLaunchError {
    message: String,
}

impl NativeAppLaunchError {
    /// Create a native app launch error from a displayable message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// Native launch configuration derived from defaults or a user config file.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct NativeAppLaunchConfig {
    /// Window and frame-pacing configuration.
    pub app: NativeAppConfig,
    /// Terminal, scrollback, and shell runtime configuration.
    pub runtime: NativeTerminalRuntimeConfig,
    /// Renderer configuration for glyph planning and frame presentation.
    pub renderer: RendererConfig,
    /// Optional TOML config path to poll for reloadable changes after launch.
    pub config_path: Option<PathBuf>,
}

impl NativeAppLaunchConfig {
    /// Build a launch configuration from a validated user configuration.
    pub fn from_gromaq_config(config: &GromaqConfig) -> Result<Self, NativeAppLaunchError> {
        let app = NativeAppConfig::from_gromaq_config(config)
            .map_err(|error| NativeAppLaunchError::new(error.to_string()))?;
        let shell = shell_command_from_settings(&config.shell);
        let runtime = NativeTerminalRuntimeConfig::from_gromaq_config(config, shell)
            .map_err(|error| NativeAppLaunchError::new(error.to_string()))?;
        let renderer = RendererConfig::from_gromaq_config(config)
            .map_err(|error| NativeAppLaunchError::new(error.to_string()))?;
        Ok(Self {
            app,
            runtime,
            renderer,
            config_path: None,
        })
    }
}

fn shell_command_from_settings(settings: &ShellSettings) -> ShellCommand {
    let mut shell = settings
        .program
        .as_ref()
        .map(|program| ShellCommand {
            program: program.into(),
            args: Vec::new(),
            cwd: None,
        })
        .unwrap_or_else(ShellCommand::default_shell);
    shell.args = settings.args.iter().map(Into::into).collect();
    shell.cwd = settings.cwd.as_ref().map(PathBuf::from);
    shell
}

/// Launches the native terminal app for the no-argument CLI path.
pub trait NativeAppLauncher {
    /// Launch the native app using `config`.
    fn launch(&self, config: NativeAppLaunchConfig) -> Result<(), NativeAppLaunchError>;
}

/// Production native app launcher.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RealNativeAppLauncher;

impl NativeAppLauncher for RealNativeAppLauncher {
    fn launch(&self, config: NativeAppLaunchConfig) -> Result<(), NativeAppLaunchError> {
        run_native_app_with_runtime_renderer_and_config_file(
            config.app,
            config.runtime,
            config.renderer,
            config.config_path.as_deref(),
        )
        .map_err(|error| NativeAppLaunchError::new(error.to_string()))
    }
}

/// Run the CLI with an injected GPU backend.
pub fn run_with_backend<I, S, B>(args: I, backend: &B) -> CliExit
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
    B: GpuBootstrapBackend,
    B::Context: AdapterReport
        + GpuSmokeRunner
        + GpuTextureUploadRunner
        + GpuGlyphAtlasUploadRunner
        + GpuTextAtlasUploadRunner
        + GpuTexturedQuadRunner
        + GpuTerminalTextRunner,
{
    let mut clipboard = NativeClipboard::new();
    run_with_backend_and_clipboard(args, backend, &mut clipboard)
}

/// Run the CLI with injected GPU and clipboard boundaries.
pub fn run_with_backend_and_clipboard<I, S, B, C>(
    args: I,
    backend: &B,
    clipboard: &mut C,
) -> CliExit
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
    B: GpuBootstrapBackend,
    B::Context: AdapterReport
        + GpuSmokeRunner
        + GpuTextureUploadRunner
        + GpuGlyphAtlasUploadRunner
        + GpuTextAtlasUploadRunner
        + GpuTexturedQuadRunner
        + GpuTerminalTextRunner,
    C: HostClipboard,
{
    run_with_optional_app_and_clipboard(
        args,
        backend,
        Option::<&RealNativeAppLauncher>::None,
        clipboard,
    )
}

/// Run the CLI with injected GPU and native app launch boundaries.
pub fn run_with_backend_and_app<I, S, B, A>(args: I, backend: &B, app_launcher: &A) -> CliExit
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
    B: GpuBootstrapBackend,
    B::Context: AdapterReport
        + GpuSmokeRunner
        + GpuTextureUploadRunner
        + GpuGlyphAtlasUploadRunner
        + GpuTextAtlasUploadRunner
        + GpuTexturedQuadRunner
        + GpuTerminalTextRunner,
    A: NativeAppLauncher,
{
    let mut clipboard = NativeClipboard::new();
    run_with_optional_app_and_clipboard(args, backend, Some(app_launcher), &mut clipboard)
}

fn run_with_optional_app_and_clipboard<I, S, B, A, C>(
    args: I,
    backend: &B,
    app_launcher: Option<&A>,
    clipboard: &mut C,
) -> CliExit
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
    B: GpuBootstrapBackend,
    B::Context: AdapterReport
        + GpuSmokeRunner
        + GpuTextureUploadRunner
        + GpuGlyphAtlasUploadRunner
        + GpuTextAtlasUploadRunner
        + GpuTexturedQuadRunner
        + GpuTerminalTextRunner,
    A: NativeAppLauncher,
    C: HostClipboard,
{
    let mut args = args.into_iter();
    let _program = args.next();
    let Some(arg) = args.next() else {
        if let Some(app_launcher) = app_launcher {
            return launch_native_app_exit(app_launcher, NativeAppLaunchConfig::default());
        }
        return CliExit {
            code: 0,
            stdout: usage(),
            stderr: String::new(),
        };
    };
    let arg = arg.as_ref();
    if arg != "--gpu-info"
        && arg != "--gpu-smoke"
        && arg != "--gpu-upload-smoke"
        && arg != "--gpu-glyph-atlas-smoke"
        && arg != "--gpu-text-atlas-smoke"
        && arg != "--gpu-textured-quad-smoke"
        && arg != "--gpu-terminal-text-smoke"
        && arg != "--clipboard-smoke"
        && arg != "--config"
        && arg != "--config-check"
        && arg != "--config-template"
        && arg != "--osc52-clipboard-smoke"
        && arg != "--runtime-clipboard-paste-smoke"
        && arg != "--runtime-glyph-frame-smoke"
        && arg != "--runtime-scrollback-smoke"
        && arg != "--runtime-perf-smoke"
        && arg != "--runtime-large-output-smoke"
        && arg != "--runtime-bounded-state-smoke"
        && arg != "--runtime-continuous-output-smoke"
        && arg != "--runtime-alternate-screen-smoke"
        && arg != "--runtime-reflow-smoke"
        && arg != "--runtime-config-reload-smoke"
        && arg != "--runtime-focus-smoke"
        && arg != "--runtime-mouse-smoke"
        && arg != "--runtime-response-smoke"
        && arg != "--runtime-idle-smoke"
        && arg != "--frame-scheduler-smoke"
    {
        return CliExit {
            code: 2,
            stdout: String::new(),
            stderr: format!("{}unknown argument: {arg}\n", usage()),
        };
    }
    if arg == "--config-check" {
        let Some(path) = args.next() else {
            return CliExit {
                code: 2,
                stdout: String::new(),
                stderr: format!("{}missing config path for --config-check\n", usage()),
            };
        };
        if let Some(extra) = args.next() {
            return CliExit {
                code: 2,
                stdout: String::new(),
                stderr: format!("{}unexpected extra argument: {}\n", usage(), extra.as_ref()),
            };
        }
        return config_check_exit(path.as_ref());
    }
    if arg == "--config-template" {
        if let Some(extra) = args.next() {
            return CliExit {
                code: 2,
                stdout: String::new(),
                stderr: format!("{}unexpected extra argument: {}\n", usage(), extra.as_ref()),
            };
        }
        return config_template_exit();
    }
    if arg == "--config" {
        let Some(path) = args.next() else {
            return CliExit {
                code: 2,
                stdout: String::new(),
                stderr: format!("{}missing config path for --config\n", usage()),
            };
        };
        if let Some(extra) = args.next() {
            return CliExit {
                code: 2,
                stdout: String::new(),
                stderr: format!("{}unexpected extra argument: {}\n", usage(), extra.as_ref()),
            };
        }
        let Some(app_launcher) = app_launcher else {
            return CliExit {
                code: 2,
                stdout: String::new(),
                stderr: format!("{}native app launch unavailable for --config\n", usage()),
            };
        };
        return launch_config_file_exit(path.as_ref(), app_launcher);
    }
    if let Some(extra) = args.next() {
        return CliExit {
            code: 2,
            stdout: String::new(),
            stderr: format!("{}unexpected extra argument: {}\n", usage(), extra.as_ref(),),
        };
    }

    if arg == "--clipboard-smoke" {
        return clipboard_smoke_exit(clipboard);
    }
    if arg == "--osc52-clipboard-smoke" {
        return osc52_clipboard_smoke_exit(clipboard);
    }
    if arg == "--runtime-clipboard-paste-smoke" {
        return runtime_clipboard_paste_smoke_exit(clipboard);
    }
    if arg == "--runtime-glyph-frame-smoke" {
        return runtime_glyph_frame_smoke_exit();
    }
    if arg == "--runtime-scrollback-smoke" {
        return runtime_scrollback_smoke_exit();
    }
    if arg == "--runtime-perf-smoke" {
        return runtime_perf_smoke_exit();
    }
    if arg == "--runtime-large-output-smoke" {
        return runtime_large_output_smoke_exit();
    }
    if arg == "--runtime-bounded-state-smoke" {
        return runtime_bounded_state_smoke_exit();
    }
    if arg == "--runtime-continuous-output-smoke" {
        return runtime_continuous_output_smoke_exit();
    }
    if arg == "--runtime-alternate-screen-smoke" {
        return runtime_alternate_screen_smoke_exit();
    }
    if arg == "--runtime-reflow-smoke" {
        return runtime_reflow_smoke_exit();
    }
    if arg == "--runtime-config-reload-smoke" {
        return runtime_config_reload_smoke_exit();
    }
    if arg == "--runtime-focus-smoke" {
        return runtime_focus_smoke_exit();
    }
    if arg == "--runtime-mouse-smoke" {
        return runtime_mouse_smoke_exit();
    }
    if arg == "--runtime-response-smoke" {
        return runtime_response_smoke_exit();
    }
    if arg == "--runtime-idle-smoke" {
        return runtime_idle_smoke_exit();
    }
    if arg == "--frame-scheduler-smoke" {
        return frame_scheduler_smoke_exit();
    }

    let bootstrap = GpuBootstrap::new(GpuBootstrapConfig::native_default());
    match bootstrap.initialize_with(backend) {
        Ok(context) if arg == "--gpu-info" => gpu_info_exit(context.adapter_report()),
        Ok(context) if arg == "--gpu-smoke" => match context.run_smoke() {
            Ok(report) => CliExit {
                code: 0,
                stdout: format!(
                    "GPU smoke: ok\nsize: {}x{}\nfirst pixel: {:?}\nnon-zero bytes: {}\n",
                    report.width, report.height, report.first_pixel, report.nonzero_bytes
                ),
                stderr: String::new(),
            },
            Err(error) => CliExit::from(error),
        },
        Ok(context) if arg == "--gpu-upload-smoke" => match context.run_texture_upload_smoke() {
            Ok(report) => CliExit {
                code: 0,
                stdout: format!(
                    "GPU upload smoke: ok\nsize: {}x{}\nfirst pixel: {:?}\nlast pixel: {:?}\nmatching bytes: {}/{}\n",
                    report.width,
                    report.height,
                    report.first_pixel,
                    report.last_pixel,
                    report.matching_bytes,
                    report.total_bytes
                ),
                stderr: String::new(),
            },
            Err(error) => CliExit::from(error),
        },
        Ok(context) if arg == "--gpu-glyph-atlas-smoke" => {
            match context.run_glyph_atlas_upload_smoke() {
                Ok(report) => CliExit {
                    code: 0,
                    stdout: format!(
                        "GPU glyph atlas smoke: ok\nsize: {}x{}\noccupied slots: {}\nfirst pixel: {:?}\nsecond slot first pixel: {:?}\nmatching bytes: {}/{}\n",
                        report.width,
                        report.height,
                        report.occupied_slots,
                        report.first_pixel,
                        report.second_slot_first_pixel,
                        report.matching_bytes,
                        report.total_bytes
                    ),
                    stderr: String::new(),
                },
                Err(error) => CliExit::from(error),
            }
        }
        Ok(context) if arg == "--gpu-text-atlas-smoke" => {
            match context.run_text_atlas_upload_smoke() {
                Ok(report) => CliExit {
                    code: 0,
                    stdout: format!(
                        "GPU text atlas smoke: ok\nsize: {}x{}\noccupied slots: {}\nrasterized glyphs: {}\nreused glyphs: {}\ncovered pixels: {}\nmatching bytes: {}/{}\n",
                        report.width,
                        report.height,
                        report.occupied_slots,
                        report.rasterized_glyphs,
                        report.reused_glyphs,
                        report.covered_pixels,
                        report.matching_bytes,
                        report.total_bytes
                    ),
                    stderr: String::new(),
                },
                Err(error) => CliExit::from(error),
            }
        }
        Ok(context) if arg == "--gpu-textured-quad-smoke" => {
            match context.run_textured_quad_smoke() {
                Ok(report) => CliExit {
                    code: 0,
                    stdout: format!(
                        "GPU textured quad smoke: ok\nsize: {}x{}\nfirst pixel: {:?}\ndrawn pixels: {}\n",
                        report.width, report.height, report.first_pixel, report.drawn_pixels
                    ),
                    stderr: String::new(),
                },
                Err(error) => CliExit::from(error),
            }
        }
        Ok(context) => match context.run_terminal_text_smoke() {
            Ok(report) => CliExit {
                code: 0,
                stdout: format!(
                    "GPU terminal text smoke: ok\nsize: {}x{}\nglyphs: {}\nbackground quads: {}\nquads: {}\nrasterized glyphs: {}\nreused glyphs: {}\nfirst drawn pixel: {:?}\ndrawn pixels: {}\n",
                    report.width,
                    report.height,
                    report.glyphs,
                    report.background_quads,
                    report.quads,
                    report.rasterized_glyphs,
                    report.reused_glyphs,
                    report.first_drawn_pixel,
                    report.drawn_pixels
                ),
                stderr: String::new(),
            },
            Err(error) => CliExit::from(error),
        },
        Err(error) => CliExit {
            code: 1,
            stdout: String::new(),
            stderr: format!("{error}\n"),
        },
    }
}

/// Adapter metadata reporting abstraction.
pub trait AdapterReport {
    /// Return stable adapter metadata.
    fn adapter_report(&self) -> &GpuAdapterSnapshot;
}

impl AdapterReport for GpuAdapterSnapshot {
    fn adapter_report(&self) -> &GpuAdapterSnapshot {
        self
    }
}

impl GpuSmokeRunner for GpuAdapterSnapshot {
    fn run_smoke(&self) -> Result<crate::native_gpu::GpuSmokeReport, GpuBootstrapError> {
        Err(GpuBootstrapError::SmokeReadback(
            "adapter metadata does not own a live GPU context".to_owned(),
        ))
    }
}

impl GpuTextureUploadRunner for GpuAdapterSnapshot {
    fn run_texture_upload_smoke(
        &self,
    ) -> Result<crate::native_gpu::GpuTextureUploadReport, GpuBootstrapError> {
        Err(GpuBootstrapError::SmokeReadback(
            "adapter metadata does not own a live GPU context".to_owned(),
        ))
    }
}

impl GpuGlyphAtlasUploadRunner for GpuAdapterSnapshot {
    fn run_glyph_atlas_upload_smoke(
        &self,
    ) -> Result<crate::native_gpu::GpuGlyphAtlasUploadReport, GpuBootstrapError> {
        Err(GpuBootstrapError::SmokeReadback(
            "adapter metadata does not own a live GPU context".to_owned(),
        ))
    }
}

impl GpuTextAtlasUploadRunner for GpuAdapterSnapshot {
    fn run_text_atlas_upload_smoke(
        &self,
    ) -> Result<crate::native_gpu::GpuTextAtlasUploadReport, GpuBootstrapError> {
        Err(GpuBootstrapError::SmokeReadback(
            "adapter metadata does not own a live GPU context".to_owned(),
        ))
    }
}

impl GpuTexturedQuadRunner for GpuAdapterSnapshot {
    fn run_textured_quad_smoke(
        &self,
    ) -> Result<crate::native_gpu::GpuTexturedQuadReport, GpuBootstrapError> {
        Err(GpuBootstrapError::SmokeReadback(
            "adapter metadata does not own a live GPU context".to_owned(),
        ))
    }
}

impl GpuTerminalTextRunner for GpuAdapterSnapshot {
    fn run_terminal_text_smoke(
        &self,
    ) -> Result<crate::native_gpu::GpuTerminalTextReport, GpuBootstrapError> {
        Err(GpuBootstrapError::SmokeReadback(
            "adapter metadata does not own a live GPU context".to_owned(),
        ))
    }
}

impl AdapterReport for crate::native_gpu::NativeGpuContext {
    fn adapter_report(&self) -> &GpuAdapterSnapshot {
        self.adapter()
    }
}

fn format_adapter(adapter: &GpuAdapterSnapshot) -> String {
    format!(
        "GPU adapter: {}\nbackend: {}\ndevice type: {}\nvendor: {}\ndevice: {}\n",
        adapter.name, adapter.backend, adapter.device_type, adapter.vendor, adapter.device
    )
}

fn gpu_info_exit(adapter: &GpuAdapterSnapshot) -> CliExit {
    CliExit {
        code: 0,
        stdout: format_adapter(adapter),
        stderr: String::new(),
    }
}

fn usage() -> String {
    "usage: gromaq [--gpu-info|--gpu-smoke|--gpu-upload-smoke|--gpu-glyph-atlas-smoke|--gpu-text-atlas-smoke|--gpu-textured-quad-smoke|--gpu-terminal-text-smoke|--clipboard-smoke|--config <path>|--config-check <path>|--config-template|--osc52-clipboard-smoke|--runtime-clipboard-paste-smoke|--runtime-glyph-frame-smoke|--runtime-scrollback-smoke|--runtime-perf-smoke|--runtime-large-output-smoke|--runtime-bounded-state-smoke|--runtime-continuous-output-smoke|--runtime-alternate-screen-smoke|--runtime-reflow-smoke|--runtime-config-reload-smoke|--runtime-focus-smoke|--runtime-mouse-smoke|--runtime-response-smoke|--runtime-idle-smoke|--frame-scheduler-smoke]\n".to_owned()
}

fn launch_config_file_exit<A>(path: &str, app_launcher: &A) -> CliExit
where
    A: NativeAppLauncher,
{
    let config = match GromaqConfig::from_toml_file(path) {
        Ok(config) => config,
        Err(error) => {
            return CliExit {
                code: 1,
                stdout: String::new(),
                stderr: format!("config launch failed: {error}\n"),
            };
        }
    };
    let launch_config = match NativeAppLaunchConfig::from_gromaq_config(&config) {
        Ok(mut launch_config) => {
            launch_config.config_path = Some(PathBuf::from(path));
            launch_config
        }
        Err(error) => {
            return CliExit {
                code: 1,
                stdout: String::new(),
                stderr: format!("{error}\n"),
            };
        }
    };
    launch_native_app_exit(app_launcher, launch_config)
}

fn launch_native_app_exit<A>(app_launcher: &A, config: NativeAppLaunchConfig) -> CliExit
where
    A: NativeAppLauncher,
{
    match app_launcher.launch(config) {
        Ok(()) => CliExit {
            code: 0,
            stdout: String::new(),
            stderr: String::new(),
        },
        Err(error) => CliExit {
            code: 1,
            stdout: String::new(),
            stderr: format!("{error}\n"),
        },
    }
}

fn config_check_exit(path: &str) -> CliExit {
    match GromaqConfig::from_toml_file(path) {
        Ok(config) => CliExit {
            code: 0,
            stdout: format!(
                "config check: ok\npath: {}\nterminal: {}x{}\nscrollback lines: {}\nshell: {}\nshell args: {}\nshell cwd: {}\nfont: {} {}px\ntarget fps: {}\ndirty-region rendering: {}\n",
                path,
                config.terminal.cols,
                config.terminal.rows,
                config.terminal.scrollback_lines,
                config.shell.program.as_deref().unwrap_or("<default>"),
                format_config_list(&config.shell.args),
                config.shell.cwd.as_deref().unwrap_or("<default>"),
                config.font.family,
                config.font.size_px,
                config.performance.target_fps,
                config.performance.dirty_region_rendering
            ),
            stderr: String::new(),
        },
        Err(error) => CliExit {
            code: 1,
            stdout: String::new(),
            stderr: format!("config check failed: {error}\n"),
        },
    }
}

fn config_template_exit() -> CliExit {
    let config = GromaqConfig::default();
    CliExit {
        code: 0,
        stdout: format!(
            "# Gromaq configuration template\n\n[terminal]\ncols = {}\nrows = {}\nscrollback_lines = {}\n\n[shell]\n# program = \"/bin/zsh\"\n# args = [\"-l\"]\n# cwd = \"/tmp\"\n\n[font]\nfamily = \"{}\"\nsize_px = {}\n\n[performance]\ntarget_fps = {}\ndirty_region_rendering = {}\n",
            config.terminal.cols,
            config.terminal.rows,
            config.terminal.scrollback_lines,
            config.font.family,
            config.font.size_px,
            config.performance.target_fps,
            config.performance.dirty_region_rendering
        ),
        stderr: String::new(),
    }
}

fn format_config_list(values: &[String]) -> String {
    if values.is_empty() {
        "<none>".to_owned()
    } else {
        values.join(" ")
    }
}

fn frame_scheduler_smoke_exit() -> CliExit {
    let mut scheduler = match FrameScheduler::new(144) {
        Ok(scheduler) => scheduler,
        Err(error) => return frame_scheduler_smoke_error(error),
    };
    let target_interval = scheduler.target_interval();
    let start = Instant::now();
    let first = scheduler.decide(start, true);
    if first != FrameDecision::render(RenderReason::FirstDirtyFrame) {
        return frame_scheduler_smoke_failure("first dirty frame was not renderable");
    }
    scheduler.record_presented(start);

    let paced = scheduler.decide(start + Duration::from_millis(2), true);
    let Some(wait_for) = paced.wait_for else {
        return frame_scheduler_smoke_failure("dirty frame was not frame-paced before interval");
    };
    if paced.reason != RenderReason::FramePaced {
        return frame_scheduler_smoke_failure("dirty frame was not frame-paced before interval");
    }
    let wait_ns = duration_as_nanos_u64(wait_for);

    let second_presented_at = start + target_interval;
    let second = scheduler.decide(second_presented_at, true);
    if second != FrameDecision::render(RenderReason::Dirty) {
        return frame_scheduler_smoke_failure("dirty frame did not render at target interval");
    }
    scheduler.record_presented(second_presented_at);

    let late_presented_at =
        second_presented_at + target_interval + target_interval + target_interval;
    scheduler.record_presented(late_presented_at);
    let idle = scheduler.decide(late_presented_at + Duration::from_nanos(1), false);
    if idle != FrameDecision::idle() {
        return frame_scheduler_smoke_failure("clean frame was not suppressed");
    }

    let metrics = scheduler.metrics();
    if metrics.frames_presented != 3 || metrics.dropped_frames != 2 {
        return frame_scheduler_smoke_failure("presented-frame metrics did not match timeline");
    }

    CliExit {
        code: 0,
        stdout: format!(
            "frame scheduler smoke: ok\ntarget fps: 144\ntarget interval ns: {}\nframe-paced wait ns: {}\nframes presented: {}\ndropped frames: {}\n",
            duration_as_nanos_u64(target_interval),
            wait_ns,
            metrics.frames_presented,
            metrics.dropped_frames
        ),
        stderr: String::new(),
    }
}

fn duration_as_nanos_u64(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
}

fn frame_scheduler_smoke_error(error: impl std::fmt::Display) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("frame scheduler smoke failed: {error}\n"),
    }
}

fn frame_scheduler_smoke_failure(reason: &str) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("frame scheduler smoke failed: {reason}\n"),
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct RuntimePerfSmokePtySpawner;

#[derive(Debug, Default)]
struct RuntimePerfSmokePtySession {
    output: VecDeque<Vec<u8>>,
}

impl NativePtySpawner for RuntimePerfSmokePtySpawner {
    type Session = RuntimePerfSmokePtySession;

    fn spawn(&self, _config: PtyConfig) -> Result<Self::Session, PtyError> {
        Ok(RuntimePerfSmokePtySession::default())
    }
}

impl NativePtySessionIo for RuntimePerfSmokePtySession {
    fn drain_output(&mut self) -> Result<Vec<u8>, PtyError> {
        Ok(self.output.pop_front().unwrap_or_default())
    }

    fn write_input(&mut self, bytes: &[u8]) -> Result<(), PtyError> {
        self.output.push_back(bytes.to_vec());
        Ok(())
    }

    fn resize(&mut self, _size: crate::app::NativePtyResize) -> Result<(), PtyError> {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
struct RuntimeInputCaptureSmokePtySpawner {
    output: &'static [u8],
}

#[derive(Debug, Default)]
struct RuntimeInputCaptureSmokePtySession {
    output: VecDeque<Vec<u8>>,
    input: Vec<Vec<u8>>,
}

impl RuntimeInputCaptureSmokePtySpawner {
    const fn new(output: &'static [u8]) -> Self {
        Self { output }
    }
}

impl RuntimeInputCaptureSmokePtySession {
    fn new(output: &'static [u8]) -> Self {
        Self {
            output: VecDeque::from([output.to_vec()]),
            input: Vec::new(),
        }
    }
}

impl NativePtySpawner for RuntimeInputCaptureSmokePtySpawner {
    type Session = RuntimeInputCaptureSmokePtySession;

    fn spawn(&self, _config: PtyConfig) -> Result<Self::Session, PtyError> {
        Ok(RuntimeInputCaptureSmokePtySession::new(self.output))
    }
}

impl NativePtySessionIo for RuntimeInputCaptureSmokePtySession {
    fn drain_output(&mut self) -> Result<Vec<u8>, PtyError> {
        Ok(self.output.pop_front().unwrap_or_default())
    }

    fn write_input(&mut self, bytes: &[u8]) -> Result<(), PtyError> {
        self.input.push(bytes.to_vec());
        Ok(())
    }

    fn resize(&mut self, _size: NativePtyResize) -> Result<(), PtyError> {
        Ok(())
    }
}

fn runtime_perf_smoke_exit() -> CliExit {
    let mut runtime = match NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 24,
        terminal_rows: 4,
        scrollback_lines: 128,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    }) {
        Ok(runtime) => runtime,
        Err(error) => return runtime_perf_smoke_error(error),
    };
    if let Err(error) = runtime.start_shell(&RuntimePerfSmokePtySpawner) {
        return runtime_perf_smoke_error(error);
    }

    let key = Key::Character("x".into());
    let sent = match runtime.send_winit_key_input(&key, ModifiersState::empty()) {
        Ok(sent) => sent,
        Err(error) => return runtime_perf_smoke_error(error),
    };
    let pumped_bytes = match runtime.pump_pty_output() {
        Ok(bytes) => bytes,
        Err(error) => return runtime_perf_smoke_error(error),
    };
    let mut renderer = match WgpuRenderer::new(RendererConfig::default()) {
        Ok(renderer) => renderer,
        Err(error) => return runtime_perf_smoke_error(error),
    };
    let rendered = match runtime.render_terminal_frame(&mut renderer) {
        Ok(rendered) => rendered,
        Err(error) => return runtime_perf_smoke_error(error),
    };
    let metrics = runtime.dump_runtime_perf_metrics();

    if !sent
        || pumped_bytes == 0
        || !rendered
        || metrics.rendered_dirty_regions == 0
        || metrics.rendered_dirty_cells == 0
        || metrics.render_time_samples == 0
        || metrics.input_to_render_samples == 0
    {
        return CliExit {
            code: 1,
            stdout: String::new(),
            stderr: "runtime perf smoke failed: input echo did not reach a rendered frame\n"
                .to_owned(),
        };
    }

    CliExit {
        code: 0,
        stdout: format!(
            "runtime perf smoke: ok\npumped bytes: {}\nrendered frames: {}\nrendered dirty regions: {}\nrendered dirty cells: {}\nrendered dirty cells max: {}\nrender samples: {}\nrender avg ns: {}\nrender max ns: {}\nrender p95 ns: {}\ninput-to-render samples: {}\ninput-to-render avg ns: {}\ninput-to-render max ns: {}\ninput-to-render p95 ns: {}\n",
            pumped_bytes,
            metrics.rendered_frames,
            metrics.rendered_dirty_regions,
            metrics.rendered_dirty_cells,
            metrics.rendered_dirty_cells_max,
            metrics.render_time_samples,
            metrics.render_time_avg_ns,
            metrics.render_time_max_ns,
            metrics.render_time_p95_ns,
            metrics.input_to_render_samples,
            metrics.input_to_render_avg_ns,
            metrics.input_to_render_max_ns,
            metrics.input_to_render_p95_ns
        ),
        stderr: String::new(),
    }
}

fn runtime_perf_smoke_error(error: impl std::fmt::Display) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime perf smoke failed: {error}\n"),
    }
}

fn runtime_focus_smoke_exit() -> CliExit {
    let mut runtime = match NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 24,
        terminal_rows: 4,
        scrollback_lines: 128,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    }) {
        Ok(runtime) => runtime,
        Err(error) => return runtime_focus_smoke_error(error),
    };
    let spawner =
        RuntimeInputCaptureSmokePtySpawner::new(RUNTIME_FOCUS_SMOKE_ENABLE_REPORTING.as_bytes());
    if let Err(error) = runtime.start_shell(&spawner) {
        return runtime_focus_smoke_error(error);
    }

    let pumped_bytes = match runtime.pump_pty_output() {
        Ok(bytes) => bytes,
        Err(error) => return runtime_focus_smoke_error(error),
    };
    let focused = match runtime.send_focus_event(true) {
        Ok(focused) => focused,
        Err(error) => return runtime_focus_smoke_error(error),
    };
    let blurred = match runtime.send_focus_event(false) {
        Ok(blurred) => blurred,
        Err(error) => return runtime_focus_smoke_error(error),
    };
    let metrics = runtime.dump_runtime_perf_metrics();
    let input = runtime
        .shell_session()
        .map(|session| session.input.concat())
        .unwrap_or_default();

    if pumped_bytes != RUNTIME_FOCUS_SMOKE_ENABLE_REPORTING.len()
        || !focused
        || !blurred
        || input != b"\x1b[I\x1b[O"
        || metrics.focus_inputs != 2
        || metrics.pty_input_writes != 2
        || metrics.pty_input_bytes != 6
    {
        return CliExit {
            code: 1,
            stdout: String::new(),
            stderr: "runtime focus smoke failed: focus reports did not reach PTY writes\n"
                .to_owned(),
        };
    }

    CliExit {
        code: 0,
        stdout: format!(
            "runtime focus smoke: ok\npumped bytes: {}\nfocus in reported: {}\nfocus out reported: {}\nfocus inputs: {}\npty input writes: {}\npty input bytes: {}\n",
            pumped_bytes,
            focused,
            blurred,
            metrics.focus_inputs,
            metrics.pty_input_writes,
            metrics.pty_input_bytes
        ),
        stderr: String::new(),
    }
}

fn runtime_focus_smoke_error(error: impl std::fmt::Display) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime focus smoke failed: {error}\n"),
    }
}

fn runtime_mouse_smoke_exit() -> CliExit {
    let mut runtime = match NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 24,
        terminal_rows: 4,
        scrollback_lines: 128,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    }) {
        Ok(runtime) => runtime,
        Err(error) => return runtime_mouse_smoke_error(error),
    };
    let spawner =
        RuntimeInputCaptureSmokePtySpawner::new(RUNTIME_MOUSE_SMOKE_ENABLE_REPORTING.as_bytes());
    if let Err(error) = runtime.start_shell(&spawner) {
        return runtime_mouse_smoke_error(error);
    }

    let pumped_bytes = match runtime.pump_pty_output() {
        Ok(bytes) => bytes,
        Err(error) => return runtime_mouse_smoke_error(error),
    };
    let pressed = match runtime.send_mouse_input(MouseEvent::new(
        MouseEventKind::Press,
        MouseButton::Left,
        2,
        1,
    )) {
        Ok(pressed) => pressed,
        Err(error) => return runtime_mouse_smoke_error(error),
    };
    let released = match runtime.send_mouse_input(MouseEvent::new(
        MouseEventKind::Release,
        MouseButton::Left,
        2,
        1,
    )) {
        Ok(released) => released,
        Err(error) => return runtime_mouse_smoke_error(error),
    };
    let wheel = match runtime.send_mouse_input(MouseEvent::new(
        MouseEventKind::Press,
        MouseButton::WheelUp,
        0,
        0,
    )) {
        Ok(wheel) => wheel,
        Err(error) => return runtime_mouse_smoke_error(error),
    };
    let metrics = runtime.dump_runtime_perf_metrics();
    let input = runtime
        .shell_session()
        .map(|session| session.input.concat())
        .unwrap_or_default();
    let expected_input = [
        b"\x1b[<0;3;2M".as_slice(),
        b"\x1b[<0;3;2m".as_slice(),
        b"\x1b[<64;1;1M".as_slice(),
    ]
    .concat();

    if pumped_bytes != RUNTIME_MOUSE_SMOKE_ENABLE_REPORTING.len()
        || !pressed
        || !released
        || !wheel
        || input != expected_input
        || metrics.mouse_inputs != 3
        || metrics.pty_input_writes != 3
        || metrics.pty_input_bytes != expected_input.len() as u64
    {
        return CliExit {
            code: 1,
            stdout: String::new(),
            stderr: "runtime mouse smoke failed: mouse reports did not reach PTY writes\n"
                .to_owned(),
        };
    }

    CliExit {
        code: 0,
        stdout: format!(
            "runtime mouse smoke: ok\npumped bytes: {}\npress reported: {}\nrelease reported: {}\nwheel reported: {}\nmouse inputs: {}\npty input writes: {}\npty input bytes: {}\n",
            pumped_bytes,
            pressed,
            released,
            wheel,
            metrics.mouse_inputs,
            metrics.pty_input_writes,
            metrics.pty_input_bytes
        ),
        stderr: String::new(),
    }
}

fn runtime_mouse_smoke_error(error: impl std::fmt::Display) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime mouse smoke failed: {error}\n"),
    }
}

fn runtime_response_smoke_exit() -> CliExit {
    let mut runtime = match NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 24,
        terminal_rows: 4,
        scrollback_lines: 128,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    }) {
        Ok(runtime) => runtime,
        Err(error) => return runtime_response_smoke_error(error),
    };
    let spawner =
        RuntimeInputCaptureSmokePtySpawner::new(RUNTIME_RESPONSE_SMOKE_QUERIES.as_bytes());
    if let Err(error) = runtime.start_shell(&spawner) {
        return runtime_response_smoke_error(error);
    }

    let pumped_bytes = match runtime.pump_pty_output() {
        Ok(bytes) => bytes,
        Err(error) => return runtime_response_smoke_error(error),
    };
    let metrics = runtime.dump_runtime_perf_metrics();
    let input = runtime
        .shell_session()
        .map(|session| session.input.concat())
        .unwrap_or_default();
    let expected_response = b"\x1b[3;5R\x1b[0n\x1b[?1;2c\x1b[>0;1;0c";

    if pumped_bytes != RUNTIME_RESPONSE_SMOKE_QUERIES.len()
        || input != expected_response
        || metrics.pty_response_writes != 1
        || metrics.pty_response_bytes != expected_response.len() as u64
        || metrics.pty_input_writes != 0
    {
        return CliExit {
            code: 1,
            stdout: String::new(),
            stderr: "runtime response smoke failed: terminal responses did not reach PTY writes\n"
                .to_owned(),
        };
    }

    CliExit {
        code: 0,
        stdout: format!(
            "runtime response smoke: ok\npumped bytes: {}\nresponse writes: {}\nresponse bytes: {}\npty input writes: {}\n",
            pumped_bytes,
            metrics.pty_response_writes,
            metrics.pty_response_bytes,
            metrics.pty_input_writes
        ),
        stderr: String::new(),
    }
}

fn runtime_response_smoke_error(error: impl std::fmt::Display) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime response smoke failed: {error}\n"),
    }
}

fn runtime_idle_smoke_exit() -> CliExit {
    let mut runtime = match NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 24,
        terminal_rows: 4,
        scrollback_lines: 128,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    }) {
        Ok(runtime) => runtime,
        Err(error) => return runtime_idle_smoke_error(error),
    };
    if let Err(error) = runtime.start_shell(&RuntimePerfSmokePtySpawner) {
        return runtime_idle_smoke_error(error);
    }
    let pumped_bytes = match runtime.pump_pty_output() {
        Ok(bytes) => bytes,
        Err(error) => return runtime_idle_smoke_error(error),
    };
    let mut renderer = match WgpuRenderer::new(RendererConfig::default()) {
        Ok(renderer) => renderer,
        Err(error) => return runtime_idle_smoke_error(error),
    };
    for _ in 0..RUNTIME_IDLE_SMOKE_RENDER_ATTEMPTS {
        let rendered = match runtime.render_terminal_frame(&mut renderer) {
            Ok(rendered) => rendered,
            Err(error) => return runtime_idle_smoke_error(error),
        };
        if rendered {
            return runtime_idle_smoke_failure("clean runtime produced a rendered frame");
        }
    }
    let metrics = runtime.dump_runtime_perf_metrics();
    if pumped_bytes != 0
        || metrics.pty_output_batches != 0
        || metrics.pty_output_bytes != 0
        || metrics.render_attempts != RUNTIME_IDLE_SMOKE_RENDER_ATTEMPTS
        || metrics.clean_frame_skips != RUNTIME_IDLE_SMOKE_RENDER_ATTEMPTS
        || metrics.rendered_frames != 0
        || metrics.render_time_samples != 0
        || metrics.input_to_render_samples != 0
    {
        return runtime_idle_smoke_failure(
            "idle runtime counters did not prove clean-frame suppression",
        );
    }

    CliExit {
        code: 0,
        stdout: format!(
            "runtime idle smoke: ok\npumped bytes: {}\nrender attempts: {}\nclean frame skips: {}\nrendered frames: {}\n",
            pumped_bytes,
            metrics.render_attempts,
            metrics.clean_frame_skips,
            metrics.rendered_frames
        ),
        stderr: String::new(),
    }
}

fn runtime_idle_smoke_error(error: impl std::fmt::Display) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime idle smoke failed: {error}\n"),
    }
}

fn runtime_idle_smoke_failure(reason: &str) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime idle smoke failed: {reason}\n"),
    }
}

#[derive(Debug, Clone)]
struct RuntimeLargeOutputSmokePtySpawner {
    payload: Vec<u8>,
}

#[derive(Debug)]
struct RuntimeLargeOutputSmokePtySession {
    output: VecDeque<Vec<u8>>,
}

impl NativePtySpawner for RuntimeLargeOutputSmokePtySpawner {
    type Session = RuntimeLargeOutputSmokePtySession;

    fn spawn(&self, _config: PtyConfig) -> Result<Self::Session, PtyError> {
        Ok(RuntimeLargeOutputSmokePtySession {
            output: VecDeque::from([self.payload.clone()]),
        })
    }
}

impl NativePtySessionIo for RuntimeLargeOutputSmokePtySession {
    fn drain_output(&mut self) -> Result<Vec<u8>, PtyError> {
        Ok(self.output.pop_front().unwrap_or_default())
    }

    fn write_input(&mut self, _bytes: &[u8]) -> Result<(), PtyError> {
        Ok(())
    }

    fn resize(&mut self, _size: crate::app::NativePtyResize) -> Result<(), PtyError> {
        Ok(())
    }
}

fn runtime_large_output_payload(lines: usize) -> Vec<u8> {
    let mut payload = Vec::new();
    for line in 0..lines {
        payload.extend_from_slice(format!("gromaq-runtime-line-{line:03}\n").as_bytes());
    }
    payload
}

fn runtime_large_output_smoke_exit() -> CliExit {
    let payload = runtime_large_output_payload(RUNTIME_LARGE_OUTPUT_LINES);
    let expected_bytes = payload.len();
    let last_line = format!("gromaq-runtime-line-{:03}", RUNTIME_LARGE_OUTPUT_LINES - 1);
    let viewport_cells = runtime_output_smoke_viewport_cells();
    let spawner = RuntimeLargeOutputSmokePtySpawner { payload };
    let mut runtime = match NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: RUNTIME_OUTPUT_SMOKE_COLS,
        terminal_rows: RUNTIME_OUTPUT_SMOKE_ROWS,
        scrollback_lines: RUNTIME_LARGE_OUTPUT_SCROLLBACK_LINES,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    }) {
        Ok(runtime) => runtime,
        Err(error) => return runtime_large_output_smoke_error(error),
    };
    if let Err(error) = runtime.start_shell(&spawner) {
        return runtime_large_output_smoke_error(error);
    }

    let pumped_bytes = match runtime.pump_pty_output() {
        Ok(bytes) => bytes,
        Err(error) => return runtime_large_output_smoke_error(error),
    };
    let mut renderer = match WgpuRenderer::new(RendererConfig::default()) {
        Ok(renderer) => renderer,
        Err(error) => return runtime_large_output_smoke_error(error),
    };
    let rendered = match runtime.render_terminal_frame(&mut renderer) {
        Ok(rendered) => rendered,
        Err(error) => return runtime_large_output_smoke_error(error),
    };
    let metrics = runtime.dump_runtime_perf_metrics();
    let scrollback = runtime.terminal().dump_scrollback();
    let visible_text = renderer
        .last_plan()
        .map(|plan| {
            plan.glyphs
                .iter()
                .map(|glyph| glyph.text.as_str())
                .collect::<String>()
        })
        .unwrap_or_default();

    if pumped_bytes != expected_bytes
        || metrics.pty_output_bytes != expected_bytes as u64
        || metrics.pty_output_batches != 1
        || !rendered
        || metrics.rendered_frames != 1
        || metrics.rendered_dirty_regions == 0
        || metrics.rendered_dirty_cells == 0
        || metrics.rendered_dirty_cells_max == 0
        || metrics.rendered_dirty_cells_max > viewport_cells
        || scrollback.lines.len() != RUNTIME_LARGE_OUTPUT_SCROLLBACK_LINES
        || scrollback
            .lines
            .iter()
            .any(|line| line == "gromaq-runtime-line-000")
        || !visible_text.contains(&last_line)
    {
        return CliExit {
            code: 1,
            stdout: String::new(),
            stderr:
                "runtime large-output smoke failed: burst did not reach a rendered visible frame\n"
                    .to_owned(),
        };
    }

    CliExit {
        code: 0,
        stdout: format!(
            "runtime large-output smoke: ok\nlines: {}\npumped bytes: {}\nscrollback lines: {}\nrendered frames: {}\nrendered dirty regions: {}\nrendered dirty cells: {}\nrendered dirty cells max: {}\nlast visible line: {}\nrender p95 ns: {}\n",
            RUNTIME_LARGE_OUTPUT_LINES,
            pumped_bytes,
            scrollback.lines.len(),
            metrics.rendered_frames,
            metrics.rendered_dirty_regions,
            metrics.rendered_dirty_cells,
            metrics.rendered_dirty_cells_max,
            last_line,
            metrics.render_time_p95_ns
        ),
        stderr: String::new(),
    }
}

fn runtime_output_smoke_viewport_cells() -> u64 {
    u64::from(RUNTIME_OUTPUT_SMOKE_COLS) * u64::from(RUNTIME_OUTPUT_SMOKE_ROWS)
}

fn runtime_large_output_smoke_error(error: impl std::fmt::Display) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime large-output smoke failed: {error}\n"),
    }
}

#[derive(Debug, Clone)]
struct RuntimeChunkedOutputSmokePtySpawner {
    payloads: Vec<Vec<u8>>,
}

#[derive(Debug)]
struct RuntimeChunkedOutputSmokePtySession {
    output: VecDeque<Vec<u8>>,
}

impl NativePtySpawner for RuntimeChunkedOutputSmokePtySpawner {
    type Session = RuntimeChunkedOutputSmokePtySession;

    fn spawn(&self, _config: PtyConfig) -> Result<Self::Session, PtyError> {
        Ok(RuntimeChunkedOutputSmokePtySession {
            output: VecDeque::from(self.payloads.clone()),
        })
    }
}

impl NativePtySessionIo for RuntimeChunkedOutputSmokePtySession {
    fn drain_output(&mut self) -> Result<Vec<u8>, PtyError> {
        Ok(self.output.pop_front().unwrap_or_default())
    }

    fn write_input(&mut self, _bytes: &[u8]) -> Result<(), PtyError> {
        Ok(())
    }

    fn resize(&mut self, _size: NativePtyResize) -> Result<(), PtyError> {
        Ok(())
    }
}

fn runtime_bounded_state_payloads() -> Vec<Vec<u8>> {
    (0..RUNTIME_BOUNDED_STATE_BATCHES)
        .map(|batch| {
            let start = batch * RUNTIME_LARGE_OUTPUT_LINES;
            let end = start + RUNTIME_LARGE_OUTPUT_LINES;
            let mut payload = Vec::new();
            for line in start..end {
                payload.extend_from_slice(format!("gromaq-bounded-line-{line:04}\n").as_bytes());
            }
            payload
        })
        .collect()
}

fn runtime_bounded_state_smoke_exit() -> CliExit {
    let payloads = runtime_bounded_state_payloads();
    let expected_bytes: usize = payloads.iter().map(Vec::len).sum();
    let total_lines = RUNTIME_LARGE_OUTPUT_LINES * RUNTIME_BOUNDED_STATE_BATCHES;
    let last_line = format!("gromaq-bounded-line-{:04}", total_lines - 1);
    let viewport_cells = runtime_output_smoke_viewport_cells();
    let spawner = RuntimeChunkedOutputSmokePtySpawner { payloads };
    let mut runtime = match NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: RUNTIME_OUTPUT_SMOKE_COLS,
        terminal_rows: RUNTIME_OUTPUT_SMOKE_ROWS,
        scrollback_lines: RUNTIME_LARGE_OUTPUT_SCROLLBACK_LINES,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    }) {
        Ok(runtime) => runtime,
        Err(error) => return runtime_bounded_state_smoke_error(error),
    };
    if let Err(error) = runtime.start_shell(&spawner) {
        return runtime_bounded_state_smoke_error(error);
    }

    let mut pumped_bytes = 0_usize;
    let mut renderer = match WgpuRenderer::new(RendererConfig::default()) {
        Ok(renderer) => renderer,
        Err(error) => return runtime_bounded_state_smoke_error(error),
    };
    for _ in 0..RUNTIME_BOUNDED_STATE_BATCHES {
        let batch_bytes = match runtime.pump_pty_output() {
            Ok(bytes) => bytes,
            Err(error) => return runtime_bounded_state_smoke_error(error),
        };
        pumped_bytes = pumped_bytes.saturating_add(batch_bytes);
        let rendered = match runtime.render_terminal_frame(&mut renderer) {
            Ok(rendered) => rendered,
            Err(error) => return runtime_bounded_state_smoke_error(error),
        };
        if batch_bytes == 0 || !rendered {
            return runtime_bounded_state_smoke_failure(
                "output batch did not render a dirty frame",
            );
        }
        let state = runtime.dump_runtime_state_snapshot();
        if state.scrollback_lines > RUNTIME_LARGE_OUTPUT_SCROLLBACK_LINES
            || state.scrollback_cell_rows > RUNTIME_LARGE_OUTPUT_SCROLLBACK_LINES
            || state.scrollback_cells > state.scrollback_cell_limit
        {
            return runtime_bounded_state_smoke_failure("scrollback state exceeded configured cap");
        }
    }

    let metrics = runtime.dump_runtime_perf_metrics();
    let state = runtime.dump_runtime_state_snapshot();
    let scrollback = runtime.terminal().dump_scrollback();
    let visible_text = renderer
        .last_plan()
        .map(|plan| {
            plan.glyphs
                .iter()
                .map(|glyph| glyph.text.as_str())
                .collect::<String>()
        })
        .unwrap_or_default();

    if pumped_bytes != expected_bytes
        || metrics.pty_output_batches != RUNTIME_BOUNDED_STATE_BATCHES as u64
        || metrics.pty_output_bytes != expected_bytes as u64
        || metrics.rendered_frames != RUNTIME_BOUNDED_STATE_BATCHES as u64
        || metrics.rendered_dirty_regions == 0
        || metrics.rendered_dirty_cells == 0
        || metrics.rendered_dirty_cells_max == 0
        || metrics.rendered_dirty_cells_max > viewport_cells
        || state.scrollback_limit != RUNTIME_LARGE_OUTPUT_SCROLLBACK_LINES
        || state.scrollback_lines != RUNTIME_LARGE_OUTPUT_SCROLLBACK_LINES
        || state.scrollback_cell_rows != RUNTIME_LARGE_OUTPUT_SCROLLBACK_LINES
        || state.scrollback_cells > state.scrollback_cell_limit
        || scrollback
            .lines
            .iter()
            .any(|line| line == "gromaq-bounded-line-0000")
        || !visible_text.contains(&last_line)
    {
        return runtime_bounded_state_smoke_failure(
            "long-session output did not stay bounded while preserving latest visible content",
        );
    }

    CliExit {
        code: 0,
        stdout: format!(
            "runtime bounded-state smoke: ok\nbatches: {}\nlines: {}\npumped bytes: {}\nscrollback cap: {}\nscrollback lines: {}\nscrollback cell rows: {}\nscrollback cells: {}\nscrollback max cells: {}\nrendered frames: {}\nrendered dirty regions: {}\nrendered dirty cells: {}\nrendered dirty cells max: {}\nlast visible line: {}\n",
            RUNTIME_BOUNDED_STATE_BATCHES,
            total_lines,
            pumped_bytes,
            state.scrollback_limit,
            state.scrollback_lines,
            state.scrollback_cell_rows,
            state.scrollback_cells,
            state.scrollback_cell_limit,
            metrics.rendered_frames,
            metrics.rendered_dirty_regions,
            metrics.rendered_dirty_cells,
            metrics.rendered_dirty_cells_max,
            last_line
        ),
        stderr: String::new(),
    }
}

fn runtime_bounded_state_smoke_error(error: impl std::fmt::Display) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime bounded-state smoke failed: {error}\n"),
    }
}

fn runtime_bounded_state_smoke_failure(reason: &str) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime bounded-state smoke failed: {reason}\n"),
    }
}

fn runtime_continuous_output_payloads() -> Vec<Vec<u8>> {
    (0..RUNTIME_CONTINUOUS_OUTPUT_BATCHES)
        .map(|batch| {
            let start = batch * RUNTIME_CONTINUOUS_OUTPUT_LINES_PER_BATCH;
            let end = start + RUNTIME_CONTINUOUS_OUTPUT_LINES_PER_BATCH;
            let mut payload = Vec::new();
            for line in start..end {
                payload.extend_from_slice(format!("gromaq-continuous-line-{line:03}\n").as_bytes());
            }
            payload
        })
        .collect()
}

fn runtime_continuous_output_smoke_exit() -> CliExit {
    let payloads = runtime_continuous_output_payloads();
    let expected_bytes: usize = payloads.iter().map(Vec::len).sum();
    let total_lines = RUNTIME_CONTINUOUS_OUTPUT_BATCHES * RUNTIME_CONTINUOUS_OUTPUT_LINES_PER_BATCH;
    let last_line = format!("gromaq-continuous-line-{:03}", total_lines - 1);
    let viewport_cells = runtime_output_smoke_viewport_cells();
    let spawner = RuntimeChunkedOutputSmokePtySpawner { payloads };
    let mut runtime = match NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: RUNTIME_OUTPUT_SMOKE_COLS,
        terminal_rows: RUNTIME_OUTPUT_SMOKE_ROWS,
        scrollback_lines: RUNTIME_CONTINUOUS_OUTPUT_SCROLLBACK_LINES,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    }) {
        Ok(runtime) => runtime,
        Err(error) => return runtime_continuous_output_smoke_error(error),
    };
    if let Err(error) = runtime.start_shell(&spawner) {
        return runtime_continuous_output_smoke_error(error);
    }

    let mut pumped_bytes = 0_usize;
    let mut renderer = match WgpuRenderer::new(RendererConfig::default()) {
        Ok(renderer) => renderer,
        Err(error) => return runtime_continuous_output_smoke_error(error),
    };
    for _ in 0..RUNTIME_CONTINUOUS_OUTPUT_BATCHES {
        let batch_bytes = match runtime.pump_pty_output() {
            Ok(bytes) => bytes,
            Err(error) => return runtime_continuous_output_smoke_error(error),
        };
        pumped_bytes = pumped_bytes.saturating_add(batch_bytes);
        let rendered = match runtime.render_terminal_frame(&mut renderer) {
            Ok(rendered) => rendered,
            Err(error) => return runtime_continuous_output_smoke_error(error),
        };
        if batch_bytes == 0 || !rendered {
            return runtime_continuous_output_smoke_failure(
                "stream batch did not render a dirty frame",
            );
        }
    }

    let metrics = runtime.dump_runtime_perf_metrics();
    let scrollback = runtime.terminal().dump_scrollback();
    let visible_text = renderer
        .last_plan()
        .map(|plan| {
            plan.glyphs
                .iter()
                .map(|glyph| glyph.text.as_str())
                .collect::<String>()
        })
        .unwrap_or_default();

    if pumped_bytes != expected_bytes
        || metrics.pty_output_batches != RUNTIME_CONTINUOUS_OUTPUT_BATCHES as u64
        || metrics.pty_output_bytes != expected_bytes as u64
        || metrics.rendered_frames != RUNTIME_CONTINUOUS_OUTPUT_BATCHES as u64
        || metrics.rendered_dirty_regions == 0
        || metrics.rendered_dirty_cells == 0
        || metrics.rendered_dirty_cells_max == 0
        || metrics.rendered_dirty_cells_max > viewport_cells
        || scrollback.lines.len() != RUNTIME_CONTINUOUS_OUTPUT_SCROLLBACK_LINES
        || scrollback
            .lines
            .iter()
            .any(|line| line == "gromaq-continuous-line-000")
        || !visible_text.contains(&last_line)
    {
        return runtime_continuous_output_smoke_failure(
            "continuous output stream did not stay responsive and bounded",
        );
    }

    CliExit {
        code: 0,
        stdout: format!(
            "runtime continuous-output smoke: ok\nbatches: {}\nlines: {}\npumped bytes: {}\nscrollback lines: {}\nrendered frames: {}\nrendered dirty regions: {}\nrendered dirty cells: {}\nrendered dirty cells max: {}\nlast visible line: {}\nrender p95 ns: {}\n",
            RUNTIME_CONTINUOUS_OUTPUT_BATCHES,
            total_lines,
            pumped_bytes,
            scrollback.lines.len(),
            metrics.rendered_frames,
            metrics.rendered_dirty_regions,
            metrics.rendered_dirty_cells,
            metrics.rendered_dirty_cells_max,
            last_line,
            metrics.render_time_p95_ns
        ),
        stderr: String::new(),
    }
}

fn runtime_continuous_output_smoke_error(error: impl std::fmt::Display) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime continuous-output smoke failed: {error}\n"),
    }
}

fn runtime_continuous_output_smoke_failure(reason: &str) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime continuous-output smoke failed: {reason}\n"),
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct RuntimeAlternateScreenSmokePtySpawner;

#[derive(Debug)]
struct RuntimeAlternateScreenSmokePtySession {
    output: VecDeque<Vec<u8>>,
}

impl NativePtySpawner for RuntimeAlternateScreenSmokePtySpawner {
    type Session = RuntimeAlternateScreenSmokePtySession;

    fn spawn(&self, _config: PtyConfig) -> Result<Self::Session, PtyError> {
        Ok(RuntimeAlternateScreenSmokePtySession {
            output: VecDeque::from([
                b"primary\n".to_vec(),
                b"\x1b[?1049halt-view\n".to_vec(),
                b"\x1b[?1049lrestored\n".to_vec(),
            ]),
        })
    }
}

impl NativePtySessionIo for RuntimeAlternateScreenSmokePtySession {
    fn drain_output(&mut self) -> Result<Vec<u8>, PtyError> {
        Ok(self.output.pop_front().unwrap_or_default())
    }

    fn write_input(&mut self, _bytes: &[u8]) -> Result<(), PtyError> {
        Ok(())
    }

    fn resize(&mut self, _size: NativePtyResize) -> Result<(), PtyError> {
        Ok(())
    }
}

fn runtime_alternate_screen_smoke_exit() -> CliExit {
    let mut runtime = match NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 24,
        terminal_rows: 4,
        scrollback_lines: 16,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    }) {
        Ok(runtime) => runtime,
        Err(error) => return runtime_alternate_screen_smoke_error(error),
    };
    if let Err(error) = runtime.start_shell(&RuntimeAlternateScreenSmokePtySpawner) {
        return runtime_alternate_screen_smoke_error(error);
    }

    let mut pumped_bytes = 0_usize;
    let mut renderer = match WgpuRenderer::new(RendererConfig::default()) {
        Ok(renderer) => renderer,
        Err(error) => return runtime_alternate_screen_smoke_error(error),
    };
    let mut alt_rendered_text = String::new();
    for stage in 0..RUNTIME_ALTERNATE_SCREEN_SMOKE_STAGES {
        let stage_bytes = match runtime.pump_pty_output() {
            Ok(bytes) => bytes,
            Err(error) => return runtime_alternate_screen_smoke_error(error),
        };
        pumped_bytes = pumped_bytes.saturating_add(stage_bytes);
        let rendered = match runtime.render_terminal_frame(&mut renderer) {
            Ok(rendered) => rendered,
            Err(error) => return runtime_alternate_screen_smoke_error(error),
        };
        if stage_bytes == 0 || !rendered {
            return runtime_alternate_screen_smoke_failure(
                "alternate-screen stage did not produce a rendered dirty frame",
            );
        }
        if stage == 1 {
            alt_rendered_text = renderer
                .last_plan()
                .map(|plan| {
                    plan.glyphs
                        .iter()
                        .map(|glyph| glyph.text.as_str())
                        .collect::<String>()
                })
                .unwrap_or_default();
        }
    }

    let metrics = runtime.dump_runtime_perf_metrics();
    let grid = runtime.terminal().dump_grid();
    let scrollback = runtime.terminal().dump_scrollback();
    let primary_restored = grid.line_text(0) == "primary" && grid.line_text(1).contains("restored");
    let alt_suppressed = scrollback
        .lines
        .iter()
        .all(|line| !line.contains("alt-view"));

    let alt_rendered = alt_rendered_text.contains("alt-view");
    if metrics.pty_output_batches != RUNTIME_ALTERNATE_SCREEN_SMOKE_STAGES as u64
        || metrics.rendered_frames != RUNTIME_ALTERNATE_SCREEN_SMOKE_STAGES as u64
        || !primary_restored
        || !alt_rendered
        || !alt_suppressed
    {
        return runtime_alternate_screen_smoke_failure(&format!(
            "expected {} PTY batches and rendered frames, got {} batches and {} frames; primary restored: {}; alternate rendered: {}; alternate scrollback suppressed: {}; visible lines: {}|{}",
            RUNTIME_ALTERNATE_SCREEN_SMOKE_STAGES,
            metrics.pty_output_batches,
            metrics.rendered_frames,
            primary_restored,
            alt_rendered,
            alt_suppressed,
            grid.line_text(0),
            grid.line_text(1)
        ));
    }

    CliExit {
        code: 0,
        stdout: format!(
            "runtime alternate-screen smoke: ok\nstages: {}\npumped bytes: {}\nprimary restored: {}\nalternate rendered: {}\nalternate scrollback suppressed: {}\nrendered frames: {}\n",
            RUNTIME_ALTERNATE_SCREEN_SMOKE_STAGES,
            pumped_bytes,
            primary_restored,
            alt_rendered,
            alt_suppressed,
            metrics.rendered_frames
        ),
        stderr: String::new(),
    }
}

fn runtime_alternate_screen_smoke_error(error: impl std::fmt::Display) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime alternate-screen smoke failed: {error}\n"),
    }
}

fn runtime_alternate_screen_smoke_failure(reason: &str) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime alternate-screen smoke failed: {reason}\n"),
    }
}

#[derive(Debug, Clone)]
struct RuntimeReflowSmokePtySpawner {
    payload: Vec<u8>,
}

#[derive(Debug)]
struct RuntimeReflowSmokePtySession {
    output: VecDeque<Vec<u8>>,
    resizes: Vec<NativePtyResize>,
}

impl NativePtySpawner for RuntimeReflowSmokePtySpawner {
    type Session = RuntimeReflowSmokePtySession;

    fn spawn(&self, _config: PtyConfig) -> Result<Self::Session, PtyError> {
        Ok(RuntimeReflowSmokePtySession {
            output: VecDeque::from([self.payload.clone()]),
            resizes: Vec::new(),
        })
    }
}

impl NativePtySessionIo for RuntimeReflowSmokePtySession {
    fn drain_output(&mut self) -> Result<Vec<u8>, PtyError> {
        Ok(self.output.pop_front().unwrap_or_default())
    }

    fn write_input(&mut self, _bytes: &[u8]) -> Result<(), PtyError> {
        Ok(())
    }

    fn resize(&mut self, size: NativePtyResize) -> Result<(), PtyError> {
        self.resizes.push(size);
        Ok(())
    }
}

fn runtime_reflow_smoke_payload() -> Vec<u8> {
    format!(
        "\x1b]8;;{RUNTIME_REFLOW_SMOKE_LINK}\x1b\\\x1b[4;58:2:17:34:51mabcdefghij\x1b[0m\x1b]8;;\x1b\\\r\nklmnopqrst\r\nuv"
    )
    .into_bytes()
}

fn runtime_reflow_smoke_exit() -> CliExit {
    let payload = runtime_reflow_smoke_payload();
    let expected_bytes = payload.len();
    let spawner = RuntimeReflowSmokePtySpawner { payload };
    let mut runtime = match NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 10,
        terminal_rows: 2,
        scrollback_lines: 10,
        pixel_width: 80,
        pixel_height: 32,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    }) {
        Ok(runtime) => runtime,
        Err(error) => return runtime_reflow_smoke_error(error),
    };
    if let Err(error) = runtime.start_shell(&spawner) {
        return runtime_reflow_smoke_error(error);
    }

    let pumped_bytes = match runtime.pump_pty_output() {
        Ok(bytes) => bytes,
        Err(error) => return runtime_reflow_smoke_error(error),
    };
    let resize = NativePtyResize {
        cols: 5,
        rows: 2,
        pixel_width: 40,
        pixel_height: 32,
    };
    if let Err(error) = runtime.resize_terminal(resize) {
        return runtime_reflow_smoke_error(error);
    }
    let mut renderer = match WgpuRenderer::new(RendererConfig::default()) {
        Ok(renderer) => renderer,
        Err(error) => return runtime_reflow_smoke_error(error),
    };
    let rendered = match runtime.render_terminal_frame(&mut renderer) {
        Ok(rendered) => rendered,
        Err(error) => return runtime_reflow_smoke_error(error),
    };
    let metrics = runtime.dump_runtime_perf_metrics();
    let grid = runtime.terminal().dump_grid();
    let scrollback = runtime.terminal().dump_scrollback();
    let retained_resize = runtime
        .shell_session()
        .and_then(|session| session.resizes.last().copied());
    let planned_text = renderer
        .last_plan()
        .map(|plan| {
            plan.glyphs
                .iter()
                .map(|glyph| glyph.text.as_str())
                .collect::<String>()
        })
        .unwrap_or_default();

    if pumped_bytes != expected_bytes
        || metrics.pty_output_bytes != expected_bytes as u64
        || metrics.resize_events != 1
        || retained_resize != Some(resize)
        || !rendered
        || metrics.rendered_frames != 1
        || scrollback.lines != vec!["abcde".to_owned(), "fghij".to_owned()]
        || scrollback.hard_breaks != vec![false, true]
        || scrollback.logical_line_ids != vec![0, 0]
        || scrollback.hyperlinks != vec![RUNTIME_REFLOW_SMOKE_LINK.to_owned()]
        || scrollback.underline_colors != vec![crate::Color::Rgb(17, 34, 51)]
        || scrollback.cells.len() != 2
        || scrollback.cells.iter().any(|row| row.len() != 5)
        || scrollback
            .cells
            .iter()
            .flatten()
            .any(|cell| cell.hyperlink_id != 1 || cell.style.underline_color_id != 1)
        || grid.cols != 5
        || grid.rows != 2
        || grid.line_text(0) != "klmno"
        || grid.line_text(1) != "pqrst"
        || !planned_text.contains("klmnopqrst")
    {
        return runtime_reflow_smoke_failure(
            "runtime resize did not preserve expected scrollback, metadata, and rendered grid",
        );
    }

    CliExit {
        code: 0,
        stdout: format!(
            "runtime reflow smoke: ok\npumped bytes: {}\nresize events: {}\nscrollback lines: {}\nscrollback hard breaks: {:?}\nscrollback logical lines: {:?}\nvisible lines: {}|{}\nrendered frames: {}\n",
            pumped_bytes,
            metrics.resize_events,
            scrollback.lines.len(),
            scrollback.hard_breaks,
            scrollback.logical_line_ids,
            grid.line_text(0),
            grid.line_text(1),
            metrics.rendered_frames
        ),
        stderr: String::new(),
    }
}

fn runtime_reflow_smoke_error(error: impl std::fmt::Display) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime reflow smoke failed: {error}\n"),
    }
}

fn runtime_reflow_smoke_failure(reason: &str) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime reflow smoke failed: {reason}\n"),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RuntimeConfigReloadSmokeReport {
    unchanged_poll_changed: bool,
    changed_poll_changed: bool,
    cols: u16,
    rows: u16,
    scrollback_lines: usize,
    target_fps: u32,
    dirty_regions: bool,
    font_size_px: u16,
    shell_program: String,
}

fn runtime_config_reload_smoke_exit() -> CliExit {
    match run_runtime_config_reload_smoke() {
        Ok(report) => CliExit {
            code: 0,
            stdout: format!(
                "runtime config reload smoke: ok\nunchanged poll changed: {}\nchanged poll changed: {}\nterminal: {}x{}\nscrollback lines: {}\ntarget fps: {}\ndirty-region rendering: {}\nfont size px: {}\nshell: {}\n",
                report.unchanged_poll_changed,
                report.changed_poll_changed,
                report.cols,
                report.rows,
                report.scrollback_lines,
                report.target_fps,
                report.dirty_regions,
                report.font_size_px,
                report.shell_program,
            ),
            stderr: String::new(),
        },
        Err(error) => CliExit {
            code: 1,
            stdout: String::new(),
            stderr: format!("runtime config reload smoke failed: {error}\n"),
        },
    }
}

fn run_runtime_config_reload_smoke() -> std::result::Result<RuntimeConfigReloadSmokeReport, String>
{
    let path = runtime_config_reload_smoke_path()?;
    let result = run_runtime_config_reload_smoke_with_path(&path);
    let _ = fs::remove_file(path);
    result
}

fn run_runtime_config_reload_smoke_with_path(
    path: &PathBuf,
) -> std::result::Result<RuntimeConfigReloadSmokeReport, String> {
    fs::write(path, "[performance]\ntarget_fps = 144\n")
        .map_err(|error| format!("failed to write initial config: {error}"))?;
    let mut app = NativeTerminalApp::new_with_runtime_and_renderer_config(
        NativeAppConfig::default(),
        NativeTerminalRuntimeConfig::default(),
        RendererConfig::default(),
    )
    .map_err(|error| error.to_string())?;
    let reloader =
        ConfigFileReloader::from_file(path.clone()).map_err(|error| error.to_string())?;
    app.set_config_reloader(reloader);

    let unchanged_poll_changed = app
        .reload_config_if_changed()
        .map_err(|error| error.to_string())?;
    if unchanged_poll_changed {
        return Err("unchanged config poll reported a reload".to_owned());
    }

    fs::write(
        path,
        r#"
        [terminal]
        cols = 28
        rows = 6
        scrollback_lines = 96

        [performance]
        target_fps = 120
        dirty_region_rendering = false

        [font]
        size_px = 18.0

        [shell]
        program = "/bin/sh"
        args = ["-l"]
        cwd = "/tmp"
        "#,
    )
    .map_err(|error| format!("failed to write changed config: {error}"))?;

    let changed_poll_changed = app
        .reload_config_if_changed()
        .map_err(|error| error.to_string())?;
    if !changed_poll_changed {
        return Err("changed config poll did not report a reload".to_owned());
    }

    let grid = app.runtime().terminal().dump_grid();
    let runtime_config = app.runtime().config();
    let app_config = app.lifecycle().config();
    let renderer_config = app.renderer().config();
    if grid.cols != 28 || grid.rows != 6 {
        return Err(format!(
            "terminal dimensions did not reload, got {}x{}",
            grid.cols, grid.rows
        ));
    }
    if runtime_config.scrollback_lines != 96 {
        return Err(format!(
            "scrollback lines did not reload, got {}",
            runtime_config.scrollback_lines
        ));
    }
    if app_config.target_fps != 120 || renderer_config.target_fps != 120 {
        return Err(format!(
            "target fps did not reload, app={}, renderer={}",
            app_config.target_fps, renderer_config.target_fps
        ));
    }
    if renderer_config.dirty_regions {
        return Err("dirty-region renderer setting did not reload".to_owned());
    }
    if renderer_config.font_size_px != 18 {
        return Err(format!(
            "renderer font size did not reload, got {}",
            renderer_config.font_size_px
        ));
    }
    if runtime_config.shell.program != "/bin/sh" {
        return Err(format!(
            "shell program did not reload, got {}",
            runtime_config.shell.program.display()
        ));
    }

    Ok(RuntimeConfigReloadSmokeReport {
        unchanged_poll_changed,
        changed_poll_changed,
        cols: grid.cols,
        rows: grid.rows,
        scrollback_lines: runtime_config.scrollback_lines,
        target_fps: app_config.target_fps,
        dirty_regions: renderer_config.dirty_regions,
        font_size_px: renderer_config.font_size_px,
        shell_program: runtime_config.shell.program.display().to_string(),
    })
}

fn runtime_config_reload_smoke_path() -> std::result::Result<PathBuf, String> {
    let directory = std::env::current_dir()
        .map_err(|error| format!("failed to resolve current directory: {error}"))?
        .join("target")
        .join("gromaq-runtime-smokes");
    fs::create_dir_all(&directory)
        .map_err(|error| format!("failed to create smoke directory: {error}"))?;
    Ok(directory.join(format!(
        "{}-runtime-config-reload-smoke.toml",
        std::process::id()
    )))
}

#[derive(Debug, Clone, Copy, Default)]
struct RuntimeClipboardPasteSmokePtySpawner;

#[derive(Debug, Default)]
struct RuntimeClipboardPasteSmokePtySession {
    input: Vec<Vec<u8>>,
}

impl NativePtySpawner for RuntimeClipboardPasteSmokePtySpawner {
    type Session = RuntimeClipboardPasteSmokePtySession;

    fn spawn(&self, _config: PtyConfig) -> Result<Self::Session, PtyError> {
        Ok(RuntimeClipboardPasteSmokePtySession::default())
    }
}

impl NativePtySessionIo for RuntimeClipboardPasteSmokePtySession {
    fn drain_output(&mut self) -> Result<Vec<u8>, PtyError> {
        Ok(Vec::new())
    }

    fn write_input(&mut self, bytes: &[u8]) -> Result<(), PtyError> {
        self.input.push(bytes.to_vec());
        Ok(())
    }

    fn resize(&mut self, _size: crate::app::NativePtyResize) -> Result<(), PtyError> {
        Ok(())
    }
}

fn runtime_clipboard_paste_smoke_exit<C: HostClipboard>(clipboard: &mut C) -> CliExit {
    let paste_key_recognized =
        is_native_paste_shortcut(&Key::Named(NamedKey::Paste), ModifiersState::empty());
    if !paste_key_recognized {
        return CliExit {
            code: 1,
            stdout: String::new(),
            stderr: "runtime clipboard paste smoke failed: OS Paste key was not recognized\n"
                .to_owned(),
        };
    }
    let mut runtime = match NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 24,
        terminal_rows: 4,
        scrollback_lines: 128,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    }) {
        Ok(runtime) => runtime,
        Err(error) => return runtime_clipboard_paste_smoke_error(error),
    };
    if let Err(error) = runtime.start_shell(&RuntimeClipboardPasteSmokePtySpawner) {
        return runtime_clipboard_paste_smoke_error(error);
    }

    let previous_text = clipboard.read_text();
    clipboard.write_text(RUNTIME_CLIPBOARD_PASTE_SMOKE_TEXT);
    let paste_result = runtime.send_clipboard_paste(clipboard);
    let restored_previous_text =
        restore_clipboard_after_smoke(clipboard, previous_text, RUNTIME_CLIPBOARD_PASTE_SMOKE_TEXT);
    let pasted = match paste_result {
        Ok(pasted) => pasted,
        Err(error) => return runtime_clipboard_paste_smoke_error(error),
    };
    let metrics = runtime.dump_runtime_perf_metrics();
    let pasted_bytes = runtime
        .shell_session()
        .and_then(|session| session.input.last())
        .map(Vec::as_slice);

    if !pasted
        || pasted_bytes != Some(RUNTIME_CLIPBOARD_PASTE_SMOKE_TEXT.as_bytes())
        || metrics.clipboard_pastes != 1
        || metrics.paste_bytes != RUNTIME_CLIPBOARD_PASTE_SMOKE_TEXT.len() as u64
        || metrics.pty_input_writes != 1
        || metrics.pty_input_bytes != RUNTIME_CLIPBOARD_PASTE_SMOKE_TEXT.len() as u64
    {
        return CliExit {
            code: 1,
            stdout: String::new(),
            stderr: "runtime clipboard paste smoke failed: clipboard text did not reach the PTY\n"
                .to_owned(),
        };
    }

    CliExit {
        code: 0,
        stdout: format!(
            "runtime clipboard paste smoke: ok\npaste key recognized: {}\npasted bytes: {}\nclipboard pastes: {}\nprevious text restored: {}\n",
            paste_key_recognized,
            RUNTIME_CLIPBOARD_PASTE_SMOKE_TEXT.len(),
            metrics.clipboard_pastes,
            restored_previous_text
        ),
        stderr: String::new(),
    }
}

fn runtime_clipboard_paste_smoke_error(error: impl std::fmt::Display) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime clipboard paste smoke failed: {error}\n"),
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct RuntimeGlyphFrameSmokePtySpawner;

#[derive(Debug)]
struct RuntimeGlyphFrameSmokePtySession {
    output: VecDeque<Vec<u8>>,
}

impl NativePtySpawner for RuntimeGlyphFrameSmokePtySpawner {
    type Session = RuntimeGlyphFrameSmokePtySession;

    fn spawn(&self, _config: PtyConfig) -> Result<Self::Session, PtyError> {
        Ok(RuntimeGlyphFrameSmokePtySession {
            output: VecDeque::from([format!("{RUNTIME_GLYPH_FRAME_SMOKE_TEXT}\n").into_bytes()]),
        })
    }
}

impl NativePtySessionIo for RuntimeGlyphFrameSmokePtySession {
    fn drain_output(&mut self) -> Result<Vec<u8>, PtyError> {
        Ok(self.output.pop_front().unwrap_or_default())
    }

    fn write_input(&mut self, _bytes: &[u8]) -> Result<(), PtyError> {
        Ok(())
    }

    fn resize(&mut self, _size: crate::app::NativePtyResize) -> Result<(), PtyError> {
        Ok(())
    }
}

fn runtime_glyph_frame_smoke_exit() -> CliExit {
    let mut runtime = match NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 32,
        terminal_rows: 4,
        scrollback_lines: 128,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    }) {
        Ok(runtime) => runtime,
        Err(error) => return runtime_glyph_frame_smoke_error(error),
    };
    if let Err(error) = runtime.start_shell(&RuntimeGlyphFrameSmokePtySpawner) {
        return runtime_glyph_frame_smoke_error(error);
    }
    let pumped_bytes = match runtime.pump_pty_output() {
        Ok(bytes) => bytes,
        Err(error) => return runtime_glyph_frame_smoke_error(error),
    };
    let mut renderer = match WgpuRenderer::new(RendererConfig::default()) {
        Ok(renderer) => renderer,
        Err(error) => return runtime_glyph_frame_smoke_error(error),
    };
    let rendered = match runtime.render_terminal_frame(&mut renderer) {
        Ok(rendered) => rendered,
        Err(error) => return runtime_glyph_frame_smoke_error(error),
    };
    if !rendered {
        return runtime_glyph_frame_smoke_failure("runtime output did not produce a dirty frame");
    }
    let atlas_metrics = renderer.glyph_atlas_metrics();
    let Some(plan) = renderer.last_plan() else {
        return runtime_glyph_frame_smoke_failure("renderer did not retain a frame plan");
    };
    if plan.glyphs.is_empty() {
        return runtime_glyph_frame_smoke_failure("render plan contained no glyphs");
    }
    let mut glyph_cache = match load_default_native_glyph_cache() {
        Ok(glyph_cache) => glyph_cache,
        Err(error) => return runtime_glyph_frame_smoke_error(error),
    };
    let glyphs = match glyph_cache.rasterize_plan(plan) {
        Ok(glyphs) => glyphs,
        Err(error) => return runtime_glyph_frame_smoke_error(error),
    };
    let prepared = match PreparedSurfaceGlyphFrame::from_render_plan(
        plan,
        &glyphs.bitmaps,
        renderer.config().clear_color,
    ) {
        Ok(prepared) => prepared,
        Err(error) => return runtime_glyph_frame_smoke_error(error),
    };
    let surface_frame = prepared.as_surface_glyph_frame();

    if pumped_bytes == 0
        || surface_frame.batch.quads.is_empty()
        || surface_frame.batch.indices.is_empty()
        || surface_frame.atlas.occupied_slots == 0
        || surface_frame.atlas.rgba.is_empty()
        || atlas_metrics.misses == 0
        || atlas_metrics.hits == 0
        || atlas_metrics.entries == 0
        || atlas_metrics.evictions != 0
    {
        return runtime_glyph_frame_smoke_failure(
            "prepared glyph frame did not contain presentable glyph data",
        );
    }

    CliExit {
        code: 0,
        stdout: format!(
            "runtime glyph frame smoke: ok\npumped bytes: {}\nplanned glyphs: {}\nrenderer atlas hits: {}\nrenderer atlas misses: {}\nrenderer atlas entries: {}\nrasterized glyphs: {}\nreused glyphs: {}\nprepared quads: {}\natlas bytes: {}\nframe size: {}x{}\n",
            pumped_bytes,
            plan.glyphs.len(),
            atlas_metrics.hits,
            atlas_metrics.misses,
            atlas_metrics.entries,
            glyphs.rasterized,
            glyphs.reused,
            surface_frame.batch.quads.len(),
            surface_frame.atlas.rgba.len(),
            surface_frame.width,
            surface_frame.height
        ),
        stderr: String::new(),
    }
}

fn runtime_glyph_frame_smoke_error(error: impl std::fmt::Display) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime glyph frame smoke failed: {error}\n"),
    }
}

fn runtime_glyph_frame_smoke_failure(reason: &str) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime glyph frame smoke failed: {reason}\n"),
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct RuntimeScrollbackSmokePtySpawner;

#[derive(Debug)]
struct RuntimeScrollbackSmokePtySession {
    output: VecDeque<Vec<u8>>,
}

impl NativePtySpawner for RuntimeScrollbackSmokePtySpawner {
    type Session = RuntimeScrollbackSmokePtySession;

    fn spawn(&self, _config: PtyConfig) -> Result<Self::Session, PtyError> {
        Ok(RuntimeScrollbackSmokePtySession {
            output: VecDeque::from([RUNTIME_SCROLLBACK_SMOKE_TEXT.as_bytes().to_vec()]),
        })
    }
}

impl NativePtySessionIo for RuntimeScrollbackSmokePtySession {
    fn drain_output(&mut self) -> Result<Vec<u8>, PtyError> {
        Ok(self.output.pop_front().unwrap_or_default())
    }

    fn write_input(&mut self, _bytes: &[u8]) -> Result<(), PtyError> {
        Ok(())
    }

    fn resize(&mut self, _size: crate::app::NativePtyResize) -> Result<(), PtyError> {
        Ok(())
    }
}

fn runtime_scrollback_smoke_exit() -> CliExit {
    let mut runtime = match NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 6,
        terminal_rows: 3,
        scrollback_lines: 8,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    }) {
        Ok(runtime) => runtime,
        Err(error) => return runtime_scrollback_smoke_error(error),
    };
    if let Err(error) = runtime.start_shell(&RuntimeScrollbackSmokePtySpawner) {
        return runtime_scrollback_smoke_error(error);
    }
    let pumped_bytes = match runtime.pump_pty_output() {
        Ok(bytes) => bytes,
        Err(error) => return runtime_scrollback_smoke_error(error),
    };
    let mut renderer = match WgpuRenderer::new(RendererConfig::default()) {
        Ok(renderer) => renderer,
        Err(error) => return runtime_scrollback_smoke_error(error),
    };
    match render_runtime_scrollback_frame(&mut runtime, &mut renderer) {
        Ok(true) => {}
        Ok(false) => {
            return runtime_scrollback_smoke_failure("initial runtime output did not render");
        }
        Err(error) => return runtime_scrollback_smoke_error(error),
    }
    let before_scroll = runtime.terminal().dump_perf_metrics();
    let live_before = runtime.terminal().dump_grid();

    match runtime.send_winit_key_input(&Key::Named(NamedKey::PageUp), ModifiersState::SHIFT) {
        Ok(true) => {}
        Ok(false) => return runtime_scrollback_smoke_failure("Shift+PageUp did not scroll"),
        Err(error) => return runtime_scrollback_smoke_error(error),
    }
    match render_runtime_scrollback_frame(&mut runtime, &mut renderer) {
        Ok(true) => {}
        Ok(false) => {
            return runtime_scrollback_smoke_failure("scrolled-back viewport did not render");
        }
        Err(error) => return runtime_scrollback_smoke_error(error),
    }
    let scrolled = runtime.terminal().dump_grid();

    match runtime.send_winit_key_input(&Key::Named(NamedKey::PageDown), ModifiersState::SHIFT) {
        Ok(true) => {}
        Ok(false) => return runtime_scrollback_smoke_failure("Shift+PageDown did not return live"),
        Err(error) => return runtime_scrollback_smoke_error(error),
    }
    match render_runtime_scrollback_frame(&mut runtime, &mut renderer) {
        Ok(true) => {}
        Ok(false) => {
            return runtime_scrollback_smoke_failure("returned live viewport did not render");
        }
        Err(error) => return runtime_scrollback_smoke_error(error),
    }
    let live_after = runtime.terminal().dump_grid();
    let terminal_perf = runtime.terminal().dump_perf_metrics();
    let runtime_perf = runtime.dump_runtime_perf_metrics();
    let expected_bytes = RUNTIME_SCROLLBACK_SMOKE_TEXT.len();
    let local_scroll_rows = terminal_perf.scrolls.saturating_sub(before_scroll.scrolls);
    let viewport_cells = u64::from(live_after.cols) * u64::from(live_after.rows);

    if pumped_bytes != expected_bytes
        || runtime_perf.pty_output_batches != 1
        || runtime_perf.pty_output_bytes != expected_bytes as u64
        || runtime_perf.pty_input_writes != 0
        || runtime_perf.pty_input_bytes != 0
        || runtime_perf.rendered_frames != 3
        || runtime_perf.rendered_dirty_regions < 3
        || runtime_perf.rendered_dirty_cells == 0
        || runtime_perf.rendered_dirty_cells_max == 0
        || runtime_perf.rendered_dirty_cells_max > viewport_cells
        || local_scroll_rows != 4
        || live_before.line_text(0) != "four"
        || live_before.line_text(1) != "five"
        || live_before.line_text(2) != "six"
        || scrolled.line_text(0) != "two"
        || scrolled.line_text(1) != "three"
        || scrolled.line_text(2) != "four"
        || live_after.line_text(0) != "four"
        || live_after.line_text(1) != "five"
        || live_after.line_text(2) != "six"
    {
        return runtime_scrollback_smoke_failure(
            "local scrollback navigation did not preserve rendered history and live view",
        );
    }

    CliExit {
        code: 0,
        stdout: format!(
            "runtime scrollback smoke: ok\npumped bytes: {}\nlocal scroll rows: {}\nrendered frames: {}\nrendered dirty regions: {}\nrendered dirty cells max: {}\nscrolled visible lines: {}|{}|{}\nlive visible lines: {}|{}|{}\npty input writes: {}\n",
            pumped_bytes,
            local_scroll_rows,
            runtime_perf.rendered_frames,
            runtime_perf.rendered_dirty_regions,
            runtime_perf.rendered_dirty_cells_max,
            scrolled.line_text(0),
            scrolled.line_text(1),
            scrolled.line_text(2),
            live_after.line_text(0),
            live_after.line_text(1),
            live_after.line_text(2),
            runtime_perf.pty_input_writes
        ),
        stderr: String::new(),
    }
}

fn render_runtime_scrollback_frame(
    runtime: &mut NativeTerminalRuntime<RuntimeScrollbackSmokePtySession>,
    renderer: &mut WgpuRenderer,
) -> Result<bool, crate::error::GromaqError> {
    runtime.render_terminal_frame(renderer)
}

fn runtime_scrollback_smoke_error(error: impl std::fmt::Display) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime scrollback smoke failed: {error}\n"),
    }
}

fn runtime_scrollback_smoke_failure(reason: &str) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime scrollback smoke failed: {reason}\n"),
    }
}

fn clipboard_smoke_exit<C: HostClipboard>(clipboard: &mut C) -> CliExit {
    let previous_text = clipboard.read_text();
    clipboard.write_text(CLIPBOARD_SMOKE_TEXT);
    let observed = clipboard.read_text();
    let restored_previous_text =
        restore_clipboard_after_smoke(clipboard, previous_text, CLIPBOARD_SMOKE_TEXT);

    match observed {
        Some(text) if text == CLIPBOARD_SMOKE_TEXT => CliExit {
            code: 0,
            stdout: format!(
                "clipboard smoke: ok\nroundtrip bytes: {}\nprevious text restored: {}\n",
                CLIPBOARD_SMOKE_TEXT.len(),
                restored_previous_text
            ),
            stderr: String::new(),
        },
        Some(text) => CliExit {
            code: 1,
            stdout: String::new(),
            stderr: format!(
                "clipboard smoke failed: expected {CLIPBOARD_SMOKE_TEXT:?}, read {text:?}\n"
            ),
        },
        None => CliExit {
            code: 1,
            stdout: String::new(),
            stderr: "clipboard smoke failed: read no text after write\n".to_owned(),
        },
    }
}

fn osc52_clipboard_smoke_exit<C: HostClipboard>(clipboard: &mut C) -> CliExit {
    let previous_text = clipboard.read_text();
    let config = match TerminalConfig::new(24, 3) {
        Ok(config) => config,
        Err(error) => {
            return CliExit {
                code: 1,
                stdout: String::new(),
                stderr: format!("OSC 52 clipboard smoke failed: {error}\n"),
            };
        }
    };
    let mut terminal = Terminal::new(config);
    let payload = general_purpose::STANDARD.encode(OSC52_CLIPBOARD_SMOKE_TEXT);
    let sequence = format!("\x1b]52;c;{payload}\x07");
    if let Err(error) = terminal.write_str(&sequence) {
        return CliExit {
            code: 1,
            stdout: String::new(),
            stderr: format!("OSC 52 clipboard smoke failed: {error}\n"),
        };
    }
    let Some(decoded_text) = terminal.dump_clipboard_text() else {
        restore_clipboard_after_smoke(clipboard, previous_text, OSC52_CLIPBOARD_SMOKE_TEXT);
        return CliExit {
            code: 1,
            stdout: String::new(),
            stderr: "OSC 52 clipboard smoke failed: terminal decoded no clipboard text\n"
                .to_owned(),
        };
    };
    if decoded_text != OSC52_CLIPBOARD_SMOKE_TEXT {
        restore_clipboard_after_smoke(clipboard, previous_text, OSC52_CLIPBOARD_SMOKE_TEXT);
        return CliExit {
            code: 1,
            stdout: String::new(),
            stderr: format!(
                "OSC 52 clipboard smoke failed: expected decoded text {OSC52_CLIPBOARD_SMOKE_TEXT:?}, got {decoded_text:?}\n"
            ),
        };
    }

    clipboard.write_text(&decoded_text);
    let observed = clipboard.read_text();
    let restored_previous_text =
        restore_clipboard_after_smoke(clipboard, previous_text, OSC52_CLIPBOARD_SMOKE_TEXT);

    match observed {
        Some(text) if text == OSC52_CLIPBOARD_SMOKE_TEXT => CliExit {
            code: 0,
            stdout: format!(
                "OSC 52 clipboard smoke: ok\ndecoded bytes: {}\nprevious text restored: {}\n",
                OSC52_CLIPBOARD_SMOKE_TEXT.len(),
                restored_previous_text
            ),
            stderr: String::new(),
        },
        Some(text) => CliExit {
            code: 1,
            stdout: String::new(),
            stderr: format!(
                "OSC 52 clipboard smoke failed: expected clipboard text {OSC52_CLIPBOARD_SMOKE_TEXT:?}, read {text:?}\n"
            ),
        },
        None => CliExit {
            code: 1,
            stdout: String::new(),
            stderr: "OSC 52 clipboard smoke failed: read no text after write\n".to_owned(),
        },
    }
}

fn restore_clipboard_after_smoke<C: HostClipboard>(
    clipboard: &mut C,
    previous_text: Option<String>,
    sentinel_text: &str,
) -> bool {
    let restorable_previous_text = previous_text
        .as_deref()
        .filter(|text| *text != sentinel_text);
    let restored_previous_text = restorable_previous_text.is_some();
    match restorable_previous_text {
        Some(previous_text) => clipboard.write_text(previous_text),
        None => clipboard.write_text(""),
    }
    restored_previous_text
}

impl From<GpuBootstrapError> for CliExit {
    fn from(value: GpuBootstrapError) -> Self {
        Self {
            code: 1,
            stdout: String::new(),
            stderr: format!("{value}\n"),
        }
    }
}
