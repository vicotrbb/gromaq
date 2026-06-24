//! Runtime scrollback CLI smoke command.

use std::collections::VecDeque;

use winit::keyboard::{Key, ModifiersState, NamedKey};

use super::CliExit;
use crate::app::{
    NativePtyResize, NativePtySessionIo, NativePtySpawner, NativeTerminalRuntime,
    NativeTerminalRuntimeConfig,
};
use crate::pty::{PtyConfig, PtyError, ShellCommand};
use crate::renderer::{RendererConfig, WgpuRenderer};

const RUNTIME_SCROLLBACK_SMOKE_TEXT: &str = "one\r\ntwo\r\nthree\r\nfour\r\nfive\r\nsix";

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

    fn resize(&mut self, _size: NativePtyResize) -> Result<(), PtyError> {
        Ok(())
    }
}

pub(super) fn runtime_scrollback_smoke_exit() -> CliExit {
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
