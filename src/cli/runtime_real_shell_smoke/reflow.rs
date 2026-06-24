use std::thread;
use std::time::Instant;

use crate::app::{
    NativePtyResize, NativeTerminalRuntime, NativeTerminalRuntimeConfig, RealNativePtySpawner,
};
use crate::cli::CliExit;
use crate::renderer::{RendererConfig, WgpuRenderer};

use super::support::{real_shell_reflow_command, runtime_transcript};
use super::{REAL_SHELL_SMOKE_POLL_INTERVAL, REAL_SHELL_SMOKE_TIMEOUT};

pub(in crate::cli) fn runtime_real_shell_reflow_smoke_exit() -> CliExit {
    let probe = match run_runtime_real_shell_reflow_smoke() {
        Ok(probe) => probe,
        Err(error) => return runtime_real_shell_reflow_smoke_error(error),
    };

    CliExit {
        code: 0,
        stdout: format!(
            "runtime real-shell reflow smoke: ok\nshell: /bin/sh\npumped bytes: {}\nresize events: {}\nscrollback lines: {}\nscrollback hard breaks: {:?}\nvisible lines: {}\nrendered frames: {}\nrendered dirty regions: {}\n",
            probe.pumped_bytes,
            probe.resize_events,
            probe.scrollback_lines,
            probe.scrollback_hard_breaks,
            probe.visible_lines,
            probe.rendered_frames,
            probe.rendered_dirty_regions
        ),
        stderr: String::new(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RuntimeRealShellReflowSmokeProbe {
    pumped_bytes: usize,
    resize_events: u64,
    scrollback_lines: usize,
    scrollback_hard_breaks: Vec<bool>,
    visible_lines: String,
    rendered_frames: u64,
    rendered_dirty_regions: u64,
}

type RuntimeRealShellReflowResult = Result<RuntimeRealShellReflowSmokeProbe, String>;

fn run_runtime_real_shell_reflow_smoke() -> RuntimeRealShellReflowResult {
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 10,
        terminal_rows: 2,
        scrollback_lines: 10,
        pixel_width: 80,
        pixel_height: 32,
        shell: real_shell_reflow_command(),
    })
    .map_err(|error| error.to_string())?;
    runtime
        .start_shell(&RealNativePtySpawner::default())
        .map_err(|error| error.to_string())?;

    let mut pumped_bytes = 0;
    let deadline = Instant::now() + REAL_SHELL_SMOKE_TIMEOUT;
    loop {
        let pumped = runtime
            .pump_pty_output()
            .map_err(|error| error.to_string())?;
        if pumped > 0 {
            pumped_bytes += pumped;
        }

        if runtime_transcript(&runtime).contains("uv") {
            break;
        }

        if Instant::now() >= deadline {
            return Err(format!(
                "timed out waiting for real shell reflow output; observed: {}",
                runtime_transcript(&runtime).replace('\n', "|")
            ));
        }
        thread::sleep(REAL_SHELL_SMOKE_POLL_INTERVAL);
    }

    runtime
        .resize_terminal(NativePtyResize {
            cols: 5,
            rows: 2,
            pixel_width: 40,
            pixel_height: 32,
        })
        .map_err(|error| error.to_string())?;
    let mut renderer =
        WgpuRenderer::new(RendererConfig::default()).map_err(|error| error.to_string())?;
    let rendered = runtime
        .render_terminal_frame(&mut renderer)
        .map_err(|error| error.to_string())?;
    let metrics = runtime.dump_runtime_perf_metrics();
    let grid = runtime.terminal().dump_grid();
    let scrollback = runtime.terminal().dump_scrollback();
    let visible_lines = format!("{}|{}", grid.line_text(0), grid.line_text(1));
    let planned_text = renderer
        .last_plan()
        .map(|plan| {
            plan.glyphs
                .iter()
                .map(|glyph| glyph.text.as_str())
                .collect::<String>()
        })
        .unwrap_or_default();

    if pumped_bytes == 0
        || metrics.resize_events != 1
        || !rendered
        || metrics.rendered_frames != 1
        || metrics.rendered_dirty_regions == 0
        || scrollback.lines != vec!["abcde".to_owned(), "fghij".to_owned()]
        || scrollback.hard_breaks != vec![false, true]
        || scrollback.logical_line_ids != vec![0, 0]
        || grid.cols != 5
        || grid.rows != 2
        || visible_lines != "klmno|pqrst"
        || !planned_text.contains("klmnopqrst")
    {
        return Err(format!(
            "real shell reflow did not preserve expected resized state; scrollback={:?}, hard_breaks={:?}, visible={visible_lines}, planned={planned_text}",
            scrollback.lines, scrollback.hard_breaks
        ));
    }

    Ok(RuntimeRealShellReflowSmokeProbe {
        pumped_bytes,
        resize_events: metrics.resize_events,
        scrollback_lines: scrollback.lines.len(),
        scrollback_hard_breaks: scrollback.hard_breaks,
        visible_lines,
        rendered_frames: metrics.rendered_frames,
        rendered_dirty_regions: metrics.rendered_dirty_regions,
    })
}

fn runtime_real_shell_reflow_smoke_error(error: impl std::fmt::Display) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime real-shell reflow smoke failed: {error}\n"),
    }
}
