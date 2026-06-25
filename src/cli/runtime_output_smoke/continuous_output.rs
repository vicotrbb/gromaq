use crate::app::{NativeTerminalRuntime, NativeTerminalRuntimeConfig};
use crate::cli::CliExit;
use crate::pty::ShellCommand;
use crate::renderer::{RendererConfig, WgpuRenderer};

use super::pty_smoke::RuntimeChunkedOutputSmokePtySpawner;
use super::{
    RUNTIME_CONTINUOUS_OUTPUT_BATCHES, RUNTIME_CONTINUOUS_OUTPUT_LINES_PER_BATCH,
    RUNTIME_CONTINUOUS_OUTPUT_SCROLLBACK_LINES, RUNTIME_OUTPUT_SMOKE_COLS,
    RUNTIME_OUTPUT_SMOKE_ROWS, runtime_output_smoke_viewport_cells,
};

mod payload;

use payload::runtime_continuous_output_payloads;

pub(in crate::cli) fn runtime_continuous_output_smoke_exit() -> CliExit {
    let payloads = runtime_continuous_output_payloads();
    let expected_bytes: usize = payloads.iter().map(Vec::len).sum();
    let total_lines = RUNTIME_CONTINUOUS_OUTPUT_BATCHES * RUNTIME_CONTINUOUS_OUTPUT_LINES_PER_BATCH;
    let last_line = format!("gromaq-continuous-line-{:03}", total_lines - 1);
    let viewport_cells = runtime_output_smoke_viewport_cells();
    let spawner = RuntimeChunkedOutputSmokePtySpawner::new(payloads);
    let mut runtime = match NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: RUNTIME_OUTPUT_SMOKE_COLS,
        terminal_rows: RUNTIME_OUTPUT_SMOKE_ROWS,
        scrollback_lines: RUNTIME_CONTINUOUS_OUTPUT_SCROLLBACK_LINES,
        pixel_width: 0,
        pixel_height: 0,
        cursor_shape: NativeTerminalRuntimeConfig::default().cursor_shape,
        cursor_blinking: NativeTerminalRuntimeConfig::default().cursor_blinking,
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
