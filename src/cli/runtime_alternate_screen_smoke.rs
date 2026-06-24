//! Runtime alternate-screen CLI smoke.

use std::collections::VecDeque;

use crate::app::{
    NativePtyResize, NativePtySessionIo, NativePtySpawner, NativeTerminalRuntime,
    NativeTerminalRuntimeConfig,
};
use crate::pty::{PtyConfig, PtyError, ShellCommand};
use crate::renderer::{RendererConfig, WgpuRenderer};

use super::CliExit;

const RUNTIME_ALTERNATE_SCREEN_SMOKE_STAGES: usize = 3;

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

pub(super) fn runtime_alternate_screen_smoke_exit() -> CliExit {
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
