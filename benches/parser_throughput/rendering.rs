use std::hint::black_box;
use std::time::{Duration, Instant};

use criterion::Criterion;
use gromaq::app::{
    NativeTerminalRuntime, NativeTerminalRuntimeConfig, load_default_native_glyph_cache,
};
use gromaq::font::FontRasterizer;
use gromaq::pty::ShellCommand;
use gromaq::renderer::{
    FrameScheduler, GlyphAtlas, GlyphAtlasConfig, GlyphEntry, GlyphQuadConfig, GlyphQuadPlanner,
    PreparedSurfaceGlyphFrame, RenderPlanner, RendererConfig, WgpuRenderer,
};
use gromaq::{Terminal, TerminalConfig};
use winit::keyboard::{Key, ModifiersState};

use crate::support::{
    ASCII_RENDER_OUTPUT, BenchPtySpawner, FRAME_SCHEDULER_TIMELINE_STEPS, GLYPH_ATLAS_CHURN_KEYS,
    GLYPH_ATLAS_HOT_KEYS, GLYPH_ATLAS_LOOKUPS, LARGE_OUTPUT, bench_monospace_font_bytes,
    glyph_atlas_bench_keys, skip_benchmark,
};

pub(crate) fn glyph_atlas_cache_churn(c: &mut Criterion) {
    let hot_keys = glyph_atlas_bench_keys(GLYPH_ATLAS_HOT_KEYS);
    let churn_keys = glyph_atlas_bench_keys(GLYPH_ATLAS_CHURN_KEYS);

    c.bench_function("glyph_atlas_cache_churn", |b| {
        b.iter(|| {
            let mut atlas = GlyphAtlas::new(GlyphAtlasConfig::new(128).unwrap());
            for key in &hot_keys {
                atlas.lookup_or_insert(key.clone()).unwrap();
            }

            for index in 0..GLYPH_ATLAS_LOOKUPS {
                let hot = &hot_keys[index % hot_keys.len()];
                black_box(atlas.lookup_or_insert(black_box(hot.clone())).unwrap());

                if index % 4 == 0 {
                    let churn = &churn_keys[index % churn_keys.len()];
                    black_box(atlas.lookup_or_insert(black_box(churn.clone())).unwrap());
                }
            }

            let metrics = atlas.metrics();
            black_box(metrics.hits);
            black_box(metrics.misses);
            black_box(metrics.evictions);
            black_box(metrics.entries);
        });
    });
}

pub(crate) fn frame_scheduler_144hz_timeline(c: &mut Criterion) {
    c.bench_function("frame_scheduler_144hz_timeline", |b| {
        b.iter(|| {
            let mut scheduler = FrameScheduler::new(144).unwrap();
            let target_interval = scheduler.target_interval();
            let start = Instant::now();
            let first = scheduler.decide(start, true);
            scheduler.record_presented(start);
            let mut now = start;
            let mut render_decisions = usize::from(first.should_render);
            let mut paced_decisions = 0_usize;

            for step in 1..FRAME_SCHEDULER_TIMELINE_STEPS {
                let paced = scheduler.decide(now + Duration::from_millis(2), true);
                if paced.wait_for.is_some() {
                    paced_decisions += 1;
                }

                now = if step % 32 == 0 {
                    now + target_interval + target_interval + target_interval
                } else {
                    now + target_interval
                };
                let decision = scheduler.decide(now, true);
                if decision.should_render {
                    render_decisions += 1;
                    scheduler.record_presented(now);
                }
            }

            let idle = scheduler.decide(now + Duration::from_nanos(1), false);
            let metrics = scheduler.metrics();
            black_box(render_decisions);
            black_box(paced_decisions);
            black_box(idle);
            black_box(metrics.frames_presented);
            black_box(metrics.dropped_frames);
        });
    });
}

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
                black_box(14),
                black_box(18),
                black_box([0.0, 0.0, 0.0, 1.0]),
                black_box([244, 192, 106, 255]),
                black_box(12),
            )
            .unwrap();
            let frame = prepared.as_surface_glyph_frame();
            black_box(frame.batch.quads.len());
            black_box(frame.batch.indices.len());
            black_box(frame.atlas.rgba.len());
        });
    });
}

pub(crate) fn native_input_echo_render_cycle(c: &mut Criterion) {
    let spawner = BenchPtySpawner {
        chunks: 0,
        echo_input: true,
    };
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 120,
        terminal_rows: 36,
        scrollback_lines: 20_000,
        pixel_width: 0,
        pixel_height: 0,
        cursor_shape: NativeTerminalRuntimeConfig::default().cursor_shape,
        cursor_blinking: NativeTerminalRuntimeConfig::default().cursor_blinking,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    })
    .unwrap();
    runtime.start_shell(&spawner).unwrap();
    let mut renderer = WgpuRenderer::new(RendererConfig::default()).unwrap();
    let key = Key::Character("x".into());

    c.bench_function("native_input_echo_render_cycle", |b| {
        b.iter(|| {
            let sent = runtime
                .send_winit_key_input(black_box(&key), black_box(ModifiersState::empty()))
                .unwrap();
            let pumped = runtime.pump_pty_output().unwrap();
            let rendered = runtime.render_terminal_frame(&mut renderer).unwrap();
            black_box(sent);
            black_box(pumped);
            black_box(rendered);
            black_box(renderer.glyph_atlas_metrics());
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
