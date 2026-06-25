use crate::app::{NativeTerminalRuntime, NativeTerminalRuntimeConfig};
use crate::cli::CliExit;
use crate::pty::ShellCommand;
use crate::renderer::{RendererConfig, WgpuRenderer};

use super::pty_smoke::RuntimeLargeOutputSmokePtySpawner;
use super::{
    RUNTIME_LARGE_OUTPUT_LINES, RUNTIME_LARGE_OUTPUT_SCROLLBACK_LINES, RUNTIME_OUTPUT_SMOKE_COLS,
    RUNTIME_OUTPUT_SMOKE_ROWS, runtime_output_smoke_viewport_cells,
};

mod output;
mod payload;

use output::{
    RuntimeLargeOutputSmokeReport, runtime_large_output_smoke_error,
    runtime_large_output_smoke_failure, runtime_large_output_smoke_success,
};
use payload::runtime_large_output_payload;

pub(in crate::cli) fn runtime_large_output_smoke_exit() -> CliExit {
    let payload = runtime_large_output_payload(RUNTIME_LARGE_OUTPUT_LINES);
    let expected_bytes = payload.len();
    let last_line = format!("gromaq-runtime-line-{:03}", RUNTIME_LARGE_OUTPUT_LINES - 1);
    let viewport_cells = runtime_output_smoke_viewport_cells();
    let spawner = RuntimeLargeOutputSmokePtySpawner::new(payload);
    let mut runtime = match NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: RUNTIME_OUTPUT_SMOKE_COLS,
        terminal_rows: RUNTIME_OUTPUT_SMOKE_ROWS,
        scrollback_lines: RUNTIME_LARGE_OUTPUT_SCROLLBACK_LINES,
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
        return runtime_large_output_smoke_failure("burst did not reach a rendered visible frame");
    }

    runtime_large_output_smoke_success(&RuntimeLargeOutputSmokeReport {
        lines: RUNTIME_LARGE_OUTPUT_LINES,
        pumped_bytes,
        scrollback_lines: scrollback.lines.len(),
        rendered_frames: metrics.rendered_frames,
        rendered_dirty_regions: metrics.rendered_dirty_regions,
        rendered_dirty_cells: metrics.rendered_dirty_cells,
        rendered_dirty_cells_max: metrics.rendered_dirty_cells_max,
        last_line,
        render_p95_ns: metrics.render_time_p95_ns,
    })
}
