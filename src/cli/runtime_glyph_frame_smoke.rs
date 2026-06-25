//! Runtime glyph-frame CLI smoke command.

use super::CliExit;

mod prepare;
mod snapshot;

use prepare::prepare_runtime_glyph_frame_smoke;
pub(super) use snapshot::runtime_glyph_frame_snapshot_exit;

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
            "runtime glyph frame smoke: ok\npumped bytes: {}\nplanned glyphs: {}\nselection backgrounds: {}\nrenderer atlas hits: {}\nrenderer atlas misses: {}\nrenderer atlas entries: {}\nrasterized glyphs: {}\nreused glyphs: {}\nprepared quads: {}\nbackground quads: {}\ncursor quads: {}\natlas bytes: {}\nframe size: {}x{}\nline height px: {}\nsurface padding px: {}\ncell spacing px: {}\n",
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
            prepared.surface_padding_px,
            prepared.cell_spacing_px
        ),
        stderr: String::new(),
    }
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
