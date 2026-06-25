use crate::app::{NativeTerminalRuntime, NativeTerminalRuntimeConfig};
use crate::cli::CliExit;
use crate::pty::ShellCommand;
use crate::renderer::{RendererConfig, WgpuRenderer};

use super::pty_smoke::RuntimeChunkedOutputSmokePtySpawner;
use super::{
    RUNTIME_BOUNDED_STATE_BATCHES, RUNTIME_LARGE_OUTPUT_LINES,
    RUNTIME_LARGE_OUTPUT_SCROLLBACK_LINES, RUNTIME_OUTPUT_SMOKE_COLS, RUNTIME_OUTPUT_SMOKE_ROWS,
    runtime_output_smoke_viewport_cells,
};

mod output;
mod payload;

use output::{
    RuntimeBoundedStateSmokeReport, runtime_bounded_state_smoke_error,
    runtime_bounded_state_smoke_failure, runtime_bounded_state_smoke_success,
};
use payload::runtime_bounded_state_payloads;

pub(in crate::cli) fn runtime_bounded_state_smoke_exit() -> CliExit {
    let payloads = runtime_bounded_state_payloads();
    let expected_bytes: usize = payloads.iter().map(Vec::len).sum();
    let total_lines = RUNTIME_LARGE_OUTPUT_LINES * RUNTIME_BOUNDED_STATE_BATCHES;
    let last_line = format!("gromaq-bounded-line-{:04}", total_lines - 1);
    let viewport_cells = runtime_output_smoke_viewport_cells();
    let spawner = RuntimeChunkedOutputSmokePtySpawner::new(payloads);
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

    runtime_bounded_state_smoke_success(&RuntimeBoundedStateSmokeReport {
        batches: RUNTIME_BOUNDED_STATE_BATCHES,
        total_lines,
        pumped_bytes,
        scrollback_cap: state.scrollback_limit,
        scrollback_lines: state.scrollback_lines,
        scrollback_cell_rows: state.scrollback_cell_rows,
        scrollback_cells: state.scrollback_cells,
        scrollback_cell_limit: state.scrollback_cell_limit,
        rendered_frames: metrics.rendered_frames,
        rendered_dirty_regions: metrics.rendered_dirty_regions,
        rendered_dirty_cells: metrics.rendered_dirty_cells,
        rendered_dirty_cells_max: metrics.rendered_dirty_cells_max,
        last_line,
    })
}
