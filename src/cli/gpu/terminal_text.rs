use std::path::Path;

use crate::cli::CliExit;
use crate::native_gpu::{
    GpuBootstrap, GpuBootstrapBackend, GpuBootstrapConfig, GpuTerminalTextPerfRunner,
    GpuTerminalTextRunner, GpuTerminalTextSnapshotRunner,
};

use super::GpuCommandContext;

pub(super) fn gpu_terminal_text_smoke_exit<C>(context: C) -> CliExit
where
    C: GpuTerminalTextRunner,
{
    match context.run_terminal_text_smoke() {
        Ok(report) => CliExit {
            code: 0,
            stdout: format!(
                "GPU terminal text smoke: ok\nsize: {}x{}\nglyphs: {}\nbackground quads: {}\nquads: {}\ndecoration quads: {}\ncursor quads: {}\nrasterized glyphs: {}\nreused glyphs: {}\nfirst drawn pixel: {:?}\nbackground pixel: {:?}\nglyph pixel: {:?}\nglyph/background contrast x100: {}\ncursor pixel: {:?}\ndrawn pixels: {}\n",
                report.width,
                report.height,
                report.glyphs,
                report.background_quads,
                report.quads,
                report.decoration_quads,
                report.cursor_quads,
                report.rasterized_glyphs,
                report.reused_glyphs,
                report.first_drawn_pixel,
                report.background_pixel,
                report.glyph_pixel,
                report.glyph_background_contrast_x100,
                report.cursor_pixel,
                report.drawn_pixels
            ),
            stderr: String::new(),
        },
        Err(error) => CliExit::from(error),
    }
}

pub(super) fn gpu_terminal_text_perf_smoke_exit<C>(context: C) -> CliExit
where
    C: GpuTerminalTextPerfRunner,
{
    match context.run_terminal_text_perf_smoke() {
        Ok(report) => CliExit {
            code: 0,
            stdout: format!(
                "GPU terminal text perf smoke: ok\nframes: {}\nsize: {}x{}\ndrawn pixels: {}\nmin ns: {}\navg ns: {}\nmax ns: {}\np95 ns: {}\n",
                report.frames,
                report.width,
                report.height,
                report.drawn_pixels,
                report.min_ns,
                report.avg_ns,
                report.max_ns,
                report.p95_ns
            ),
            stderr: String::new(),
        },
        Err(error) => CliExit::from(error),
    }
}

pub(in crate::cli) fn gpu_terminal_text_snapshot_exit<B>(path: &str, backend: &B) -> CliExit
where
    B: GpuBootstrapBackend,
    B::Context: GpuCommandContext,
{
    let snapshot_path = Path::new(path);
    if snapshot_path.as_os_str().is_empty() {
        return CliExit {
            code: 2,
            stdout: String::new(),
            stderr: "snapshot path must not be empty\n".to_owned(),
        };
    }
    let bootstrap = GpuBootstrap::new(GpuBootstrapConfig::native_default());
    match bootstrap.initialize_with(backend) {
        Ok(context) => match context.run_terminal_text_snapshot(snapshot_path) {
            Ok(report) => CliExit {
                code: 0,
                stdout: format!(
                    "GPU terminal text snapshot: ok\npath: {}\nsize: {}x{}\nbytes written: {}\nglyphs: {}\nbackground pixel: {:?}\nglyph pixel: {:?}\nglyph/background contrast x100: {}\ncursor pixel: {:?}\ndrawn pixels: {}\n",
                    snapshot_path.display(),
                    report.width,
                    report.height,
                    report.bytes_written,
                    report.glyphs,
                    report.background_pixel,
                    report.glyph_pixel,
                    report.glyph_background_contrast_x100,
                    report.cursor_pixel,
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
