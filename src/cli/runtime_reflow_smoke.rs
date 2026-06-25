use super::CliExit;
use crate::app::{NativePtyResize, NativeTerminalRuntime, NativeTerminalRuntimeConfig};
use crate::pty::ShellCommand;
use crate::renderer::{RendererConfig, WgpuRenderer};

mod pty;

use pty::RuntimeReflowSmokePtySpawner;

const RUNTIME_REFLOW_SMOKE_LINK: &str = "https://gromaq.dev";

fn runtime_reflow_smoke_payload() -> Vec<u8> {
    format!(
        "\x1b]8;;{RUNTIME_REFLOW_SMOKE_LINK}\x1b\\\x1b[4;58:2:17:34:51mabcdefghij\x1b[0m\x1b]8;;\x1b\\\r\nklmnopqrst\r\nuv"
    )
    .into_bytes()
}

pub(super) fn runtime_reflow_smoke_exit() -> CliExit {
    let payload = runtime_reflow_smoke_payload();
    let expected_bytes = payload.len();
    let spawner = RuntimeReflowSmokePtySpawner::new(payload);
    let mut runtime = match NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 10,
        terminal_rows: 2,
        scrollback_lines: 10,
        pixel_width: 80,
        pixel_height: 32,
        cursor_shape: NativeTerminalRuntimeConfig::default().cursor_shape,
        cursor_blinking: NativeTerminalRuntimeConfig::default().cursor_blinking,
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
