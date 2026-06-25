use std::collections::VecDeque;

use super::{runtime_glyph_frame_smoke_error, runtime_glyph_frame_smoke_failure};
use crate::app::{
    NativePtyResize, NativePtySessionIo, NativePtySpawner, NativeTerminalRuntime,
    NativeTerminalRuntimeConfig, load_default_native_glyph_cache,
};
use crate::cli::CliExit;
use crate::pty::{PtyConfig, PtyError, ShellCommand};
use crate::renderer::{
    PreparedSurfaceGlyphFrame, PreparedSurfaceGlyphFrameConfig, RendererConfig, WgpuRenderer,
};
use crate::selection::SelectionRange;

const RUNTIME_GLYPH_FRAME_SMOKE_TEXT: &str = "gromaq glyph frame";

#[derive(Debug)]
pub(super) struct PreparedRuntimeGlyphFrameSmoke {
    pub(super) pumped_bytes: usize,
    pub(super) planned_glyphs: usize,
    pub(super) selection_backgrounds: usize,
    pub(super) atlas_hits: u64,
    pub(super) atlas_misses: u64,
    pub(super) atlas_entries: usize,
    pub(super) atlas_evictions: u64,
    pub(super) rasterized_glyphs: usize,
    pub(super) reused_glyphs: usize,
    pub(super) line_height_px: u16,
    pub(super) surface_padding_px: u16,
    pub(super) cell_spacing_px: u16,
    pub(super) expected_selection: [u8; 4],
    pub(super) prepared: PreparedSurfaceGlyphFrame,
}

#[derive(Debug, Clone, Copy, Default)]
struct RuntimeGlyphFrameSmokePtySpawner;

#[derive(Debug)]
struct RuntimeGlyphFrameSmokePtySession {
    output: VecDeque<Vec<u8>>,
}

impl NativePtySpawner for RuntimeGlyphFrameSmokePtySpawner {
    type Session = RuntimeGlyphFrameSmokePtySession;

    fn spawn(&self, _config: PtyConfig) -> Result<Self::Session, PtyError> {
        Ok(RuntimeGlyphFrameSmokePtySession {
            output: VecDeque::from([format!("{RUNTIME_GLYPH_FRAME_SMOKE_TEXT}\n").into_bytes()]),
        })
    }
}

impl NativePtySessionIo for RuntimeGlyphFrameSmokePtySession {
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

pub(super) fn prepare_runtime_glyph_frame_smoke() -> Result<PreparedRuntimeGlyphFrameSmoke, CliExit>
{
    let mut runtime = match NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 32,
        terminal_rows: 4,
        scrollback_lines: 128,
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
        Err(error) => return Err(runtime_glyph_frame_smoke_error(error)),
    };
    if let Err(error) = runtime.start_shell(&RuntimeGlyphFrameSmokePtySpawner) {
        return Err(runtime_glyph_frame_smoke_error(error));
    }
    let pumped_bytes = match runtime.pump_pty_output() {
        Ok(bytes) => bytes,
        Err(error) => return Err(runtime_glyph_frame_smoke_error(error)),
    };
    runtime.set_selection(SelectionRange::new((0, 0), (0, 5)));
    let mut renderer = match WgpuRenderer::new(RendererConfig::default()) {
        Ok(renderer) => renderer,
        Err(error) => return Err(runtime_glyph_frame_smoke_error(error)),
    };
    let rendered = match runtime.render_terminal_frame(&mut renderer) {
        Ok(rendered) => rendered,
        Err(error) => return Err(runtime_glyph_frame_smoke_error(error)),
    };
    if !rendered {
        return Err(runtime_glyph_frame_smoke_failure(
            "runtime output did not produce a dirty frame",
        ));
    }
    let atlas_metrics = renderer.glyph_atlas_metrics();
    let Some(plan) = renderer.last_plan() else {
        return Err(runtime_glyph_frame_smoke_failure(
            "renderer did not retain a frame plan",
        ));
    };
    if plan.glyphs.is_empty() {
        return Err(runtime_glyph_frame_smoke_failure(
            "render plan contained no glyphs",
        ));
    }
    if plan.backgrounds.is_empty() {
        return Err(runtime_glyph_frame_smoke_failure(
            "render plan contained no selection background",
        ));
    }
    let mut glyph_cache = match load_default_native_glyph_cache() {
        Ok(glyph_cache) => glyph_cache,
        Err(error) => return Err(runtime_glyph_frame_smoke_error(error)),
    };
    let glyphs = match glyph_cache.rasterize_plan(plan) {
        Ok(glyphs) => glyphs,
        Err(error) => return Err(runtime_glyph_frame_smoke_error(error)),
    };
    let prepared = match PreparedSurfaceGlyphFrame::from_render_plan(
        plan,
        &glyphs.bitmaps,
        PreparedSurfaceGlyphFrameConfig {
            cell_width_px: renderer.config().cell_width_px,
            line_height_px: renderer.config().line_height_px,
            clear_color: renderer.config().clear_color,
            cursor_color_rgba8: renderer.config().cursor_color_rgba8,
            surface_padding_px: renderer.config().surface_padding_px,
            cell_spacing_px: renderer.config().cell_spacing_px,
        },
    ) {
        Ok(prepared) => prepared,
        Err(error) => return Err(runtime_glyph_frame_smoke_error(error)),
    };
    Ok(PreparedRuntimeGlyphFrameSmoke {
        pumped_bytes,
        planned_glyphs: plan.glyphs.len(),
        selection_backgrounds: plan.backgrounds.len(),
        atlas_hits: atlas_metrics.hits,
        atlas_misses: atlas_metrics.misses,
        atlas_entries: atlas_metrics.entries,
        atlas_evictions: atlas_metrics.evictions,
        rasterized_glyphs: glyphs.rasterized,
        reused_glyphs: glyphs.reused,
        line_height_px: renderer.config().line_height_px,
        surface_padding_px: renderer.config().surface_padding_px,
        cell_spacing_px: renderer.config().cell_spacing_px,
        expected_selection: renderer.config().selection_background_rgba8,
        prepared,
    })
}
