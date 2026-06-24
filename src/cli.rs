//! Command-line entry points for the native application.

use std::collections::VecDeque;

use base64::{Engine as _, engine::general_purpose};
use winit::keyboard::{Key, ModifiersState, NamedKey};

mod config_commands;
mod frame_scheduler_smoke;
mod gpu;
mod runtime_alternate_screen_smoke;
mod runtime_config_reload_smoke;
mod runtime_input_smoke;
mod runtime_output_smoke;
mod runtime_reflow_smoke;
pub use config_commands::{
    NativeAppLaunchConfig, NativeAppLaunchError, NativeAppLauncher, RealNativeAppLauncher,
};
use config_commands::{
    config_check_exit, config_template_exit, launch_config_file_exit, launch_native_app_exit,
};
use frame_scheduler_smoke::frame_scheduler_smoke_exit;
use gpu::gpu_command_exit;
pub use gpu::{AdapterReport, GpuCommandContext};
use runtime_alternate_screen_smoke::runtime_alternate_screen_smoke_exit;
use runtime_config_reload_smoke::runtime_config_reload_smoke_exit;
use runtime_input_smoke::{
    runtime_focus_smoke_exit, runtime_idle_smoke_exit, runtime_mouse_smoke_exit,
    runtime_perf_smoke_exit, runtime_response_smoke_exit,
};
use runtime_output_smoke::{
    runtime_bounded_state_smoke_exit, runtime_continuous_output_smoke_exit,
    runtime_large_output_smoke_exit,
};
use runtime_reflow_smoke::runtime_reflow_smoke_exit;

use crate::app::{
    NativePtySessionIo, NativePtySpawner, NativeTerminalRuntime, NativeTerminalRuntimeConfig,
    is_native_paste_shortcut, load_default_native_glyph_cache,
};
use crate::clipboard::{HostClipboard, NativeClipboard};
use crate::native_gpu::GpuBootstrapBackend;
use crate::pty::{PtyConfig, PtyError, ShellCommand};
use crate::renderer::{PreparedSurfaceGlyphFrame, RendererConfig, WgpuRenderer};
use crate::terminal::{Terminal, TerminalConfig};

const CLIPBOARD_SMOKE_TEXT: &str = "gromaq clipboard smoke";
const OSC52_CLIPBOARD_SMOKE_TEXT: &str = "gromaq osc52 smoke";
const RUNTIME_CLIPBOARD_PASTE_SMOKE_TEXT: &str = "gromaq runtime clipboard paste";
const RUNTIME_GLYPH_FRAME_SMOKE_TEXT: &str = "gromaq glyph frame";
const RUNTIME_SCROLLBACK_SMOKE_TEXT: &str = "one\r\ntwo\r\nthree\r\nfour\r\nfive\r\nsix";

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

/// Run the CLI with an injected GPU backend.
pub fn run_with_backend<I, S, B>(args: I, backend: &B) -> CliExit
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
    B: GpuBootstrapBackend,
    B::Context: GpuCommandContext,
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
    B::Context: GpuCommandContext,
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
    B::Context: GpuCommandContext,
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
    B::Context: GpuCommandContext,
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

    gpu_command_exit(arg, backend)
}

fn usage() -> String {
    "usage: gromaq [--gpu-info|--gpu-smoke|--gpu-upload-smoke|--gpu-glyph-atlas-smoke|--gpu-text-atlas-smoke|--gpu-textured-quad-smoke|--gpu-terminal-text-smoke|--clipboard-smoke|--config <path>|--config-check <path>|--config-template|--osc52-clipboard-smoke|--runtime-clipboard-paste-smoke|--runtime-glyph-frame-smoke|--runtime-scrollback-smoke|--runtime-perf-smoke|--runtime-large-output-smoke|--runtime-bounded-state-smoke|--runtime-continuous-output-smoke|--runtime-alternate-screen-smoke|--runtime-reflow-smoke|--runtime-config-reload-smoke|--runtime-focus-smoke|--runtime-mouse-smoke|--runtime-response-smoke|--runtime-idle-smoke|--frame-scheduler-smoke]\n".to_owned()
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
