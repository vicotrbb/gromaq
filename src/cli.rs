//! Command-line entry points for the native application.

use base64::{Engine as _, engine::general_purpose};
use thiserror::Error;

use crate::app::{NativeAppConfig, run_native_app};
use crate::clipboard::{HostClipboard, NativeClipboard};
use crate::native_gpu::{
    GpuAdapterSnapshot, GpuBootstrap, GpuBootstrapBackend, GpuBootstrapConfig, GpuBootstrapError,
    GpuGlyphAtlasUploadRunner, GpuSmokeRunner, GpuTerminalTextRunner, GpuTextAtlasUploadRunner,
    GpuTextureUploadRunner, GpuTexturedQuadRunner,
};
use crate::terminal::{Terminal, TerminalConfig};

const CLIPBOARD_SMOKE_TEXT: &str = "gromaq clipboard smoke";
const OSC52_CLIPBOARD_SMOKE_TEXT: &str = "gromaq osc52 smoke";

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

/// Launches the native terminal app for the no-argument CLI path.
pub trait NativeAppLauncher {
    /// Launch the native app using `config`.
    fn launch(&self, config: NativeAppConfig) -> Result<(), NativeAppLaunchError>;
}

/// Production native app launcher.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RealNativeAppLauncher;

impl NativeAppLauncher for RealNativeAppLauncher {
    fn launch(&self, config: NativeAppConfig) -> Result<(), NativeAppLaunchError> {
        run_native_app(config).map_err(|error| NativeAppLaunchError::new(error.to_string()))
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
            return match app_launcher.launch(NativeAppConfig::default()) {
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
            };
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
        && arg != "--osc52-clipboard-smoke"
    {
        return CliExit {
            code: 2,
            stdout: String::new(),
            stderr: format!("{}unknown argument: {arg}\n", usage()),
        };
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
                    "GPU terminal text smoke: ok\nsize: {}x{}\nglyphs: {}\nquads: {}\nrasterized glyphs: {}\nreused glyphs: {}\ndrawn pixels: {}\n",
                    report.width,
                    report.height,
                    report.glyphs,
                    report.quads,
                    report.rasterized_glyphs,
                    report.reused_glyphs,
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
    "usage: gromaq [--gpu-info|--gpu-smoke|--gpu-upload-smoke|--gpu-glyph-atlas-smoke|--gpu-text-atlas-smoke|--gpu-textured-quad-smoke|--gpu-terminal-text-smoke|--clipboard-smoke|--osc52-clipboard-smoke]\n".to_owned()
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
