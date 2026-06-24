use std::process::Command;

use crate::app::{NativeTerminalRuntime, NativeTerminalRuntimeConfig};
use crate::cli::CliExit;
use crate::pty::ShellCommand;
use crate::renderer::{RendererConfig, WgpuRenderer};

use super::pty_smoke::RuntimeChunkedOutputSmokePtySpawner;
use super::{
    RUNTIME_LARGE_OUTPUT_LINES, RUNTIME_LARGE_OUTPUT_SCROLLBACK_LINES,
    RUNTIME_MEMORY_SMOKE_MEASURED_BATCHES, RUNTIME_MEMORY_SMOKE_RSS_GROWTH_LIMIT_KIB,
    RUNTIME_OUTPUT_SMOKE_COLS, RUNTIME_OUTPUT_SMOKE_ROWS, runtime_output_smoke_viewport_cells,
};

const RUNTIME_MEMORY_SMOKE_WARMUP_BATCHES: usize = 1;

fn runtime_memory_payloads() -> Vec<Vec<u8>> {
    let total_batches = RUNTIME_MEMORY_SMOKE_WARMUP_BATCHES + RUNTIME_MEMORY_SMOKE_MEASURED_BATCHES;
    (0..total_batches)
        .map(|batch| {
            let start = batch * RUNTIME_LARGE_OUTPUT_LINES;
            let end = start + RUNTIME_LARGE_OUTPUT_LINES;
            let mut payload = Vec::new();
            for line in start..end {
                payload.extend_from_slice(format!("gromaq-memory-line-{line:04}\n").as_bytes());
            }
            payload
        })
        .collect()
}

pub(in crate::cli) fn runtime_memory_smoke_exit() -> CliExit {
    runtime_memory_smoke_exit_with_sampler(current_process_rss_kib)
}

fn runtime_memory_smoke_exit_with_sampler(
    mut sample_rss_kib: impl FnMut() -> Result<u64, String>,
) -> CliExit {
    let payloads = runtime_memory_payloads();
    let expected_bytes: usize = payloads.iter().map(Vec::len).sum();
    let total_batches = payloads.len();
    let total_lines = total_batches * RUNTIME_LARGE_OUTPUT_LINES;
    let last_line = format!("gromaq-memory-line-{:04}", total_lines - 1);
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
        Err(error) => return runtime_memory_smoke_error(error),
    };
    if let Err(error) = runtime.start_shell(&spawner) {
        return runtime_memory_smoke_error(error);
    }
    let mut renderer = match WgpuRenderer::new(RendererConfig::default()) {
        Ok(renderer) => renderer,
        Err(error) => return runtime_memory_smoke_error(error),
    };

    let mut pumped_bytes = 0_usize;
    let mut baseline_rss_kib = None;
    let mut peak_rss_kib = 0_u64;
    for batch in 0..total_batches {
        let batch_bytes = match runtime.pump_pty_output() {
            Ok(bytes) => bytes,
            Err(error) => return runtime_memory_smoke_error(error),
        };
        pumped_bytes = pumped_bytes.saturating_add(batch_bytes);
        let rendered = match runtime.render_terminal_frame(&mut renderer) {
            Ok(rendered) => rendered,
            Err(error) => return runtime_memory_smoke_error(error),
        };
        if batch_bytes == 0 || !rendered {
            return runtime_memory_smoke_failure("output batch did not render a dirty frame");
        }

        let state = runtime.dump_runtime_state_snapshot();
        if state.scrollback_lines > RUNTIME_LARGE_OUTPUT_SCROLLBACK_LINES
            || state.scrollback_cell_rows > RUNTIME_LARGE_OUTPUT_SCROLLBACK_LINES
            || state.scrollback_cells > state.scrollback_cell_limit
        {
            return runtime_memory_smoke_failure("scrollback state exceeded configured cap");
        }

        let rss = match sample_rss_kib() {
            Ok(rss) => rss,
            Err(error) => return runtime_memory_smoke_error(error),
        };
        if batch + 1 == RUNTIME_MEMORY_SMOKE_WARMUP_BATCHES {
            baseline_rss_kib = Some(rss);
            peak_rss_kib = rss;
        } else if baseline_rss_kib.is_some() {
            peak_rss_kib = peak_rss_kib.max(rss);
        }
    }

    let Some(baseline_rss_kib) = baseline_rss_kib else {
        return runtime_memory_smoke_failure("rss baseline was not sampled after warmup");
    };
    let rss_growth_kib = peak_rss_kib.saturating_sub(baseline_rss_kib);
    if rss_growth_kib > RUNTIME_MEMORY_SMOKE_RSS_GROWTH_LIMIT_KIB {
        return runtime_memory_smoke_failure("process rss growth exceeded configured cap");
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
        || metrics.pty_output_batches != total_batches as u64
        || metrics.pty_output_bytes != expected_bytes as u64
        || metrics.rendered_frames != total_batches as u64
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
            .any(|line| line == "gromaq-memory-line-0000")
        || !visible_text.contains(&last_line)
    {
        return runtime_memory_smoke_failure(
            "long-session output did not stay memory-bounded while preserving latest content",
        );
    }

    CliExit {
        code: 0,
        stdout: format!(
            "runtime memory smoke: ok\nwarmup batches: {}\nmeasured batches: {}\nlines: {}\npumped bytes: {}\nscrollback cap: {}\nscrollback lines: {}\nscrollback cells: {}\nscrollback max cells: {}\nrendered frames: {}\nrendered dirty cells max: {}\nrss baseline kib: {}\nrss peak kib: {}\nrss growth kib: {}\nrss growth cap kib: {}\nlast visible line: {}\n",
            RUNTIME_MEMORY_SMOKE_WARMUP_BATCHES,
            RUNTIME_MEMORY_SMOKE_MEASURED_BATCHES,
            total_lines,
            pumped_bytes,
            state.scrollback_limit,
            state.scrollback_lines,
            state.scrollback_cells,
            state.scrollback_cell_limit,
            metrics.rendered_frames,
            metrics.rendered_dirty_cells_max,
            baseline_rss_kib,
            peak_rss_kib,
            rss_growth_kib,
            RUNTIME_MEMORY_SMOKE_RSS_GROWTH_LIMIT_KIB,
            last_line
        ),
        stderr: String::new(),
    }
}

fn current_process_rss_kib() -> Result<u64, String> {
    let pid = std::process::id().to_string();
    let output = Command::new("ps")
        .args(["-o", "rss=", "-p", &pid])
        .output()
        .map_err(|error| format!("process rss sampling failed to start: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "process rss sampling failed with status {}",
            output.status
        ));
    }
    let stdout = String::from_utf8(output.stdout)
        .map_err(|error| format!("process rss output was not utf-8: {error}"))?;
    stdout
        .split_whitespace()
        .next()
        .ok_or_else(|| "process rss output was empty".to_owned())?
        .parse::<u64>()
        .map_err(|error| format!("process rss output was not numeric: {error}"))
}

fn runtime_memory_smoke_error(error: impl std::fmt::Display) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime memory smoke failed: {error}\n"),
    }
}

fn runtime_memory_smoke_failure(reason: &str) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime memory smoke failed: {reason}\n"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_memory_smoke_reports_bounded_rss_growth() {
        let mut rss = 10_000_u64;

        let exit = runtime_memory_smoke_exit_with_sampler(|| {
            rss = rss.saturating_add(128);
            Ok(rss)
        });

        assert_eq!(exit.code, 0, "{exit:?}");
        assert!(exit.stdout.contains("runtime memory smoke: ok"));
        assert!(exit.stdout.contains("warmup batches: 1"));
        assert!(exit.stdout.contains("measured batches: 8"));
        assert!(exit.stdout.contains("lines: 4608"));
        assert!(exit.stdout.contains("rss growth cap kib: 65536"));
        assert!(
            exit.stdout
                .contains("last visible line: gromaq-memory-line-4607")
        );
        assert!(exit.stderr.is_empty());
    }

    #[test]
    fn runtime_memory_smoke_rejects_rss_growth_over_cap() {
        let mut samples = 0_u64;

        let exit = runtime_memory_smoke_exit_with_sampler(|| {
            samples = samples.saturating_add(1);
            if samples == 1 {
                Ok(10_000)
            } else {
                Ok(10_000 + RUNTIME_MEMORY_SMOKE_RSS_GROWTH_LIMIT_KIB + 1)
            }
        });

        assert_eq!(exit.code, 1);
        assert!(exit.stdout.is_empty());
        assert!(
            exit.stderr
                .contains("process rss growth exceeded configured cap")
        );
    }
}
