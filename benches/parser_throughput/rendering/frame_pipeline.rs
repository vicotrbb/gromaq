use std::hint::black_box;

use criterion::Criterion;
use gromaq::app::load_default_native_glyph_cache;
use gromaq::font::FontRasterizer;
use gromaq::renderer::{
    GlyphAtlas, GlyphAtlasConfig, GlyphEntry, GlyphQuadConfig, GlyphQuadPlanner,
    PreparedSurfaceGlyphFrame, PreparedSurfaceGlyphFrameConfig, RenderPlanner,
};
use gromaq::{Terminal, TerminalConfig};

use crate::support::{
    ASCII_RENDER_OUTPUT, LARGE_OUTPUT, bench_monospace_font_bytes, skip_benchmark,
};

pub(crate) fn render_plan_large_dirty_region(c: &mut Criterion) {
    let mut terminal = Terminal::new(TerminalConfig::new(120, 36).unwrap());
    for _ in 0..128 {
        terminal.write_str(LARGE_OUTPUT).unwrap();
    }
    let grid = terminal.dump_grid();
    let cursor = terminal.dump_cursor();
    let dirty_regions = terminal.take_dirty_regions();

    c.bench_function("render_plan_large_dirty_region", |b| {
        b.iter(|| {
            let mut atlas = GlyphAtlas::new(GlyphAtlasConfig::new(4096).unwrap());
            let mut planner = RenderPlanner::new(14);
            let plan = planner
                .plan_frame(
                    black_box(&grid),
                    black_box(cursor),
                    black_box(&dirty_regions),
                    black_box(&mut atlas),
                )
                .unwrap();
            black_box(plan.glyphs.len());
            black_box(atlas.metrics());
        });
    });
}

pub(crate) fn glyph_quad_generation_large_plan(c: &mut Criterion) {
    let mut terminal = Terminal::new(TerminalConfig::new(120, 36).unwrap());
    for _ in 0..128 {
        terminal.write_str(LARGE_OUTPUT).unwrap();
    }
    let dirty_regions = terminal.take_dirty_regions();
    let mut atlas = GlyphAtlas::new(GlyphAtlasConfig::new(4096).unwrap());
    let mut render_planner = RenderPlanner::new(14);
    let plan = render_planner
        .plan_frame(
            &terminal.dump_grid(),
            terminal.dump_cursor(),
            &dirty_regions,
            &mut atlas,
        )
        .unwrap();
    let quad_planner = GlyphQuadPlanner::new(GlyphQuadConfig {
        cell_width_px: 8,
        cell_height_px: 16,
        atlas_slot_width_px: 12,
        atlas_slot_height_px: 20,
        atlas_columns: 64,
        atlas_width_px: 768,
        atlas_height_px: 1280,
    });

    c.bench_function("glyph_quad_generation_large_plan", |b| {
        b.iter(|| {
            let batch = quad_planner.plan(black_box(&plan)).unwrap();
            black_box(batch.quads.len());
            black_box(batch.indices.len());
        });
    });
}

pub(crate) fn rasterized_glyph_cache_hot_plan(c: &mut Criterion) {
    let mut terminal = Terminal::new(TerminalConfig::new(120, 36).unwrap());
    for _ in 0..128 {
        terminal.write_str(ASCII_RENDER_OUTPUT).unwrap();
    }
    let dirty_regions = terminal.take_dirty_regions();
    let mut atlas = GlyphAtlas::new(GlyphAtlasConfig::new(4096).unwrap());
    let mut render_planner = RenderPlanner::new(14);
    let plan = render_planner
        .plan_frame(
            &terminal.dump_grid(),
            terminal.dump_cursor(),
            &dirty_regions,
            &mut atlas,
        )
        .unwrap();
    let mut glyph_cache = match load_default_native_glyph_cache() {
        Ok(glyph_cache) => glyph_cache,
        Err(error) => {
            skip_benchmark(c, "rasterized_glyph_cache_hot_plan", &error.to_string());
            return;
        }
    };
    glyph_cache.rasterize_plan(&plan).unwrap();

    c.bench_function("rasterized_glyph_cache_hot_plan", |b| {
        b.iter(|| {
            let batch = glyph_cache.rasterize_plan(black_box(&plan)).unwrap();
            black_box(batch.bitmaps.len());
            black_box(batch.reused);
        });
    });
}

pub(crate) fn prepared_surface_glyph_frame_large_plan(c: &mut Criterion) {
    let mut terminal = Terminal::new(TerminalConfig::new(120, 36).unwrap());
    for _ in 0..128 {
        terminal.write_str(ASCII_RENDER_OUTPUT).unwrap();
    }
    let dirty_regions = terminal.take_dirty_regions();
    let mut atlas = GlyphAtlas::new(GlyphAtlasConfig::new(4096).unwrap());
    let mut render_planner = RenderPlanner::new(14);
    let plan = render_planner
        .plan_frame(
            &terminal.dump_grid(),
            terminal.dump_cursor(),
            &dirty_regions,
            &mut atlas,
        )
        .unwrap();
    let mut glyph_cache = match load_default_native_glyph_cache() {
        Ok(glyph_cache) => glyph_cache,
        Err(error) => {
            skip_benchmark(
                c,
                "prepared_surface_glyph_frame_large_plan",
                &error.to_string(),
            );
            return;
        }
    };
    let glyphs = glyph_cache.rasterize_plan(&plan).unwrap();

    c.bench_function("prepared_surface_glyph_frame_large_plan", |b| {
        b.iter(|| {
            let prepared = PreparedSurfaceGlyphFrame::from_render_plan(
                black_box(&plan),
                black_box(&glyphs.bitmaps),
                black_box(PreparedSurfaceGlyphFrameConfig {
                    cell_width_px: 14,
                    line_height_px: 18,
                    clear_color: [0.0, 0.0, 0.0, 1.0],
                    cursor_color_rgba8: [244, 192, 106, 255],
                    surface_padding_px: 12,
                    cell_spacing_px: 0,
                }),
            )
            .unwrap();
            let frame = prepared.as_surface_glyph_frame();
            black_box(frame.batch.quads.len());
            black_box(frame.batch.indices.len());
            black_box(frame.atlas.rgba.len());
        });
    });
}

pub(crate) fn font_rasterizer_combining_cell(c: &mut Criterion) {
    let font_bytes = match bench_monospace_font_bytes() {
        Ok(font_bytes) => font_bytes,
        Err(error) => {
            skip_benchmark(c, "font_rasterizer_combining_cell", &error);
            return;
        }
    };
    let mut rasterizer = match FontRasterizer::from_bytes(font_bytes) {
        Ok(rasterizer) => rasterizer,
        Err(error) => {
            skip_benchmark(c, "font_rasterizer_combining_cell", &error.to_string());
            return;
        }
    };
    let mut slot = 0_u32;

    c.bench_function("font_rasterizer_combining_cell", |b| {
        b.iter(|| {
            slot += 1;
            let bitmap = rasterizer
                .rasterize_text(
                    black_box("A\u{0301}"),
                    black_box(24.0),
                    GlyphEntry {
                        slot,
                        generation: 0,
                    },
                )
                .unwrap();
            black_box(bitmap.rgba.len());
        });
    });
}
