//! Runtime repaint CLI smoke command.

use std::ffi::OsString;

use crate::app::{NativeTerminalRuntime, NativeTerminalRuntimeConfig};
use crate::dirty::DirtyRegion;
use crate::pty::ShellCommand;
use crate::renderer::{RendererConfig, WgpuRenderer};

use super::CliExit;

mod pty;

use pty::RepaintSmokePtySpawner;

const REPAINT_COLS: u16 = 80;
const REPAINT_ROWS: u16 = 8;

pub(super) fn runtime_repaint_smoke_exit() -> CliExit {
    match runtime_repaint_smoke_report() {
        Ok(report) => CliExit {
            code: 0,
            stdout: format!(
                "runtime repaint smoke: ok\npumped bytes: {}\nrendered: {}\nfull viewport repainted: {}\ncommand preserved: {}\nfirst output row preserved: {}\nsecond output row preserved: {}\nprompt preserved: {}\nplanned glyphs: {}\nclear regions: {}\n",
                report.pumped_bytes,
                report.rendered,
                report.full_viewport_repainted,
                report.command_preserved,
                report.first_output_row_preserved,
                report.second_output_row_preserved,
                report.prompt_preserved,
                report.planned_glyphs,
                report.clear_regions
            ),
            stderr: String::new(),
        },
        Err(error) => CliExit {
            code: 1,
            stdout: String::new(),
            stderr: format!("runtime repaint smoke failed: {error}\n"),
        },
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RuntimeRepaintSmokeReport {
    pumped_bytes: usize,
    rendered: bool,
    full_viewport_repainted: bool,
    command_preserved: bool,
    first_output_row_preserved: bool,
    second_output_row_preserved: bool,
    prompt_preserved: bool,
    planned_glyphs: usize,
    clear_regions: usize,
}

fn runtime_repaint_smoke_report() -> Result<RuntimeRepaintSmokeReport, String> {
    let spawner = RepaintSmokePtySpawner::new(vec![zsh_repaint_payload()]);
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: REPAINT_COLS,
        terminal_rows: REPAINT_ROWS,
        scrollback_lines: 100,
        pixel_width: 0,
        pixel_height: 0,
        cursor_shape: NativeTerminalRuntimeConfig::default().cursor_shape,
        cursor_blinking: NativeTerminalRuntimeConfig::default().cursor_blinking,
        shell: ShellCommand {
            program: OsString::from("/bin/zsh"),
            args: Vec::new(),
            cwd: None,
        },
    })
    .map_err(|error| error.to_string())?;
    runtime
        .start_shell(&spawner)
        .map_err(|error| error.to_string())?;
    let pumped_bytes = runtime
        .pump_pty_output()
        .map_err(|error| error.to_string())?;

    let mut renderer =
        WgpuRenderer::new(RendererConfig::default()).map_err(|error| error.to_string())?;
    runtime.invalidate_terminal_frame();
    let rendered = runtime
        .render_terminal_frame(&mut renderer)
        .map_err(|error| error.to_string())?;
    let plan = renderer
        .last_plan()
        .ok_or_else(|| "renderer did not retain a repaint plan".to_owned())?;
    let planned_text = plan
        .glyphs
        .iter()
        .map(|glyph| glyph.text.as_str())
        .collect::<String>();
    let full_viewport = [DirtyRegion {
        row: 0,
        col: 0,
        rows: REPAINT_ROWS,
        cols: REPAINT_COLS,
    }];

    let report = RuntimeRepaintSmokeReport {
        pumped_bytes,
        rendered,
        full_viewport_repainted: plan.clear_regions == full_viewport,
        command_preserved: planned_text.contains(">ls"),
        first_output_row_preserved: planned_text.contains("Applications"),
        second_output_row_preserved: planned_text.contains("Documents"),
        prompt_preserved: planned_text.contains("~/Daedalus/gromaq"),
        planned_glyphs: plan.glyphs.len(),
        clear_regions: plan.clear_regions.len(),
    };
    if report.pumped_bytes == 0
        || !report.rendered
        || !report.full_viewport_repainted
        || !report.command_preserved
        || !report.first_output_row_preserved
        || !report.second_output_row_preserved
        || !report.prompt_preserved
        || report.planned_glyphs == 0
    {
        return Err(format!(
            "repaint report did not preserve visible terminal output: {report:?}"
        ));
    }
    Ok(report)
}

fn zsh_repaint_payload() -> Vec<u8> {
    b"\r\x1b[2K\x1b[1G> ls\x1b[K\r\n\
      Applications    Downloads\r\n\
      Documents       Projects\r\n\
      \r\x1b[J\r\n\
      \x1b[A~/Daedalus/gromaq ................................ rb 2.7.5 15:11\r\n\
      \x1b[2K\x1b[1G\x1b[38;5;76m>\x1b[39m \x1b[K\x1b[?2004h"
        .to_vec()
}
