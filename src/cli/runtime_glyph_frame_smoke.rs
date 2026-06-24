//! Runtime glyph-frame CLI smoke command.

use std::collections::VecDeque;
use std::fs;
use std::path::Path;

use super::CliExit;
use crate::app::{
    NativePtyResize, NativePtySessionIo, NativePtySpawner, NativeTerminalRuntime,
    NativeTerminalRuntimeConfig, load_default_native_glyph_cache,
};
use crate::pty::{PtyConfig, PtyError, ShellCommand};
use crate::renderer::{PreparedSurfaceGlyphFrame, RendererConfig, WgpuRenderer};
use crate::selection::SelectionRange;

const RUNTIME_GLYPH_FRAME_SMOKE_TEXT: &str = "gromaq glyph frame";

#[derive(Debug)]
struct PreparedRuntimeGlyphFrameSmoke {
    pumped_bytes: usize,
    planned_glyphs: usize,
    selection_backgrounds: usize,
    atlas_hits: u64,
    atlas_misses: u64,
    atlas_entries: usize,
    atlas_evictions: u64,
    rasterized_glyphs: usize,
    reused_glyphs: usize,
    line_height_px: u16,
    surface_padding_px: u16,
    expected_selection: [u8; 4],
    prepared: PreparedSurfaceGlyphFrame,
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

pub(super) fn runtime_glyph_frame_smoke_exit() -> CliExit {
    let prepared = match prepare_runtime_glyph_frame_smoke() {
        Ok(prepared) => prepared,
        Err(exit) => return exit,
    };
    let surface_frame = prepared.prepared.as_surface_glyph_frame();

    if prepared.pumped_bytes == 0
        || surface_frame.batch.quads.is_empty()
        || surface_frame.batch.indices.is_empty()
        || surface_frame.background_batch.quads.is_empty()
        || surface_frame.cursor_batch.quads.is_empty()
        || surface_frame.atlas.occupied_slots == 0
        || surface_frame.atlas.rgba.is_empty()
        || prepared.atlas_misses == 0
        || prepared.atlas_hits == 0
        || prepared.atlas_entries == 0
        || prepared.atlas_evictions != 0
    {
        return runtime_glyph_frame_smoke_failure(
            "prepared glyph frame did not contain presentable glyph data",
        );
    }
    let selection_color = surface_frame.background_batch.quads[0].vertices[0].color_rgba;
    if !normalized_color_matches_rgba8(selection_color, prepared.expected_selection) {
        return runtime_glyph_frame_smoke_failure("selection background did not use theme color");
    }

    CliExit {
        code: 0,
        stdout: format!(
            "runtime glyph frame smoke: ok\npumped bytes: {}\nplanned glyphs: {}\nselection backgrounds: {}\nrenderer atlas hits: {}\nrenderer atlas misses: {}\nrenderer atlas entries: {}\nrasterized glyphs: {}\nreused glyphs: {}\nprepared quads: {}\nbackground quads: {}\ncursor quads: {}\natlas bytes: {}\nframe size: {}x{}\nline height px: {}\nsurface padding px: {}\n",
            prepared.pumped_bytes,
            prepared.planned_glyphs,
            prepared.selection_backgrounds,
            prepared.atlas_hits,
            prepared.atlas_misses,
            prepared.atlas_entries,
            prepared.rasterized_glyphs,
            prepared.reused_glyphs,
            surface_frame.batch.quads.len(),
            surface_frame.background_batch.quads.len(),
            surface_frame.cursor_batch.quads.len(),
            surface_frame.atlas.rgba.len(),
            surface_frame.width,
            surface_frame.height,
            prepared.line_height_px,
            prepared.surface_padding_px
        ),
        stderr: String::new(),
    }
}

pub(super) fn runtime_glyph_frame_snapshot_exit(path: &str) -> CliExit {
    let prepared = match prepare_runtime_glyph_frame_smoke() {
        Ok(prepared) => prepared,
        Err(exit) => return exit,
    };
    let preview = match prepared.prepared.preview_rgba8() {
        Ok(preview) => preview,
        Err(error) => return runtime_glyph_frame_smoke_error(error),
    };
    let snapshot = match runtime_glyph_frame_ppm_bytes(preview.width, preview.height, &preview.rgba)
    {
        Ok(snapshot) => snapshot,
        Err(exit) => return exit,
    };
    if let Err(error) = fs::write(Path::new(path), &snapshot) {
        return runtime_glyph_frame_smoke_error(format!(
            "failed to write runtime glyph frame snapshot: {error}"
        ));
    }
    let surface_frame = prepared.prepared.as_surface_glyph_frame();

    CliExit {
        code: 0,
        stdout: format!(
            "runtime glyph frame snapshot: ok\npath: {path}\nbytes written: {}\nframe size: {}x{}\npreview pixels: {}\nprepared quads: {}\nbackground quads: {}\ncursor quads: {}\natlas bytes: {}\n",
            snapshot.len(),
            preview.width,
            preview.height,
            preview.rgba.len() / 4,
            surface_frame.batch.quads.len(),
            surface_frame.background_batch.quads.len(),
            surface_frame.cursor_batch.quads.len(),
            surface_frame.atlas.rgba.len()
        ),
        stderr: String::new(),
    }
}

fn prepare_runtime_glyph_frame_smoke() -> Result<PreparedRuntimeGlyphFrameSmoke, CliExit> {
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
        renderer.config().cell_width_px,
        renderer.config().line_height_px,
        renderer.config().clear_color,
        renderer.config().cursor_color_rgba8,
        renderer.config().surface_padding_px,
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
        expected_selection: renderer.config().selection_background_rgba8,
        prepared,
    })
}

fn runtime_glyph_frame_ppm_bytes(
    width: u32,
    height: u32,
    pixels: &[u8],
) -> Result<Vec<u8>, CliExit> {
    let expected_rgba_len = match usize::try_from(u64::from(width) * u64::from(height) * 4) {
        Ok(len) => len,
        Err(_) => {
            return Err(runtime_glyph_frame_smoke_failure(
                "runtime glyph frame snapshot is too large",
            ));
        }
    };
    if pixels.len() != expected_rgba_len {
        return Err(runtime_glyph_frame_smoke_failure(&format!(
            "runtime glyph frame snapshot expected {expected_rgba_len} RGBA bytes, got {}",
            pixels.len()
        )));
    }
    let header = format!("P6\n{width} {height}\n255\n");
    let rgb_len = match usize::try_from(u64::from(width) * u64::from(height) * 3) {
        Ok(len) => len,
        Err(_) => {
            return Err(runtime_glyph_frame_smoke_failure(
                "runtime glyph frame snapshot RGB buffer is too large",
            ));
        }
    };
    let mut snapshot = Vec::new();
    if snapshot.try_reserve_exact(header.len() + rgb_len).is_err() {
        return Err(runtime_glyph_frame_smoke_failure(
            "runtime glyph frame snapshot allocation failed",
        ));
    }
    snapshot.extend_from_slice(header.as_bytes());
    for pixel in pixels.chunks_exact(4) {
        snapshot.extend_from_slice(&pixel[..3]);
    }
    Ok(snapshot)
}

fn normalized_color_matches_rgba8(actual: [f32; 4], expected: [u8; 4]) -> bool {
    actual
        .into_iter()
        .zip(expected)
        .all(|(actual, expected)| (actual - srgb8_to_linear_f32(expected)).abs() <= 0.001)
}

fn srgb8_to_linear_f32(value: u8) -> f32 {
    let srgb = f32::from(value) / 255.0;
    if srgb <= 0.04045 {
        srgb / 12.92
    } else {
        ((srgb + 0.055) / 1.055).powf(2.4)
    }
}

fn runtime_glyph_frame_smoke_error(error: impl std::fmt::Display) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime glyph frame smoke failed: {error}\n"),
    }
}

fn runtime_glyph_frame_smoke_failure(reason: &str) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime glyph frame smoke failed: {reason}\n"),
    }
}
