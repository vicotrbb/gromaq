use std::collections::VecDeque;
use std::hint::black_box;
use std::path::Path;

use criterion::{Criterion, criterion_group, criterion_main};
use gromaq::app::{
    NativePtyResize, NativePtySessionIo, NativePtySpawner, NativeTerminalRuntime,
    NativeTerminalRuntimeConfig, load_default_native_glyph_cache,
};
use gromaq::font::FontRasterizer;
use gromaq::pty::{PtyConfig, PtyError, ShellCommand};
use gromaq::renderer::{
    GlyphAtlas, GlyphAtlasConfig, GlyphEntry, GlyphQuadConfig, GlyphQuadPlanner,
    PreparedSurfaceGlyphFrame, RenderPlanner,
};
use gromaq::{DirtyRegion, DirtyTracker, Terminal, TerminalConfig};

const LARGE_OUTPUT: &str = "\
\x1b[31;1merror\x1b[0m line one\n\
normal log line with unicode 界 and attributes\n\
\x1b[32mok\x1b[0m line three\n\
";

const ASCII_RENDER_OUTPUT: &str = "\
error status 0123456789 ABC xyz\n\
normal log line with attributes\n\
prompt $ cargo test --all\n\
";

const BENCH_MONOSPACE_FONT_CANDIDATES: &[&str] = &[
    "/System/Library/Fonts/SFNSMono.ttf",
    "/System/Library/Fonts/Menlo.ttc",
    "/System/Library/Fonts/Supplemental/Courier New.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
    "/usr/share/fonts/dejavu-sans-fonts/DejaVuSansMono.ttf",
    "/usr/share/fonts/truetype/liberation2/LiberationMono-Regular.ttf",
    "/usr/share/fonts/liberation/LiberationMono-Regular.ttf",
    "/usr/share/fonts/truetype/noto/NotoSansMono-Regular.ttf",
];

#[derive(Debug)]
struct BenchPtySession {
    output: VecDeque<Vec<u8>>,
}

impl NativePtySessionIo for BenchPtySession {
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

#[derive(Debug, Clone, Copy)]
struct BenchPtySpawner {
    chunks: usize,
}

impl NativePtySpawner for BenchPtySpawner {
    type Session = BenchPtySession;

    fn spawn(&self, _config: PtyConfig) -> Result<Self::Session, PtyError> {
        let mut output = VecDeque::with_capacity(self.chunks);
        for _ in 0..self.chunks {
            output.push_back(LARGE_OUTPUT.as_bytes().to_vec());
        }
        Ok(BenchPtySession { output })
    }
}

fn parser_large_output(c: &mut Criterion) {
    c.bench_function("parser_large_output", |b| {
        b.iter(|| {
            let mut terminal = Terminal::new(TerminalConfig::new(120, 36).unwrap());
            for _ in 0..256 {
                terminal.write_str(black_box(LARGE_OUTPUT)).unwrap();
            }
            black_box(terminal.dump_perf_metrics());
        });
    });
}

fn scrollback_large_output(c: &mut Criterion) {
    let mut output = String::with_capacity(20_000);
    for line in 0..2_000 {
        use std::fmt::Write as _;
        writeln!(&mut output, "line {line:04}").expect("writing to a String is infallible");
    }

    c.bench_function("scrollback_large_output", |b| {
        b.iter(|| {
            let mut terminal = Terminal::new(
                TerminalConfig::new(80, 12)
                    .unwrap()
                    .with_scrollback_limit(20_000)
                    .unwrap(),
            );
            terminal.write_bytes(black_box(output.as_bytes())).unwrap();
            black_box(terminal.dump_scrollback());
        });
    });
}

fn dirty_region_coalescing(c: &mut Criterion) {
    c.bench_function("dirty_region_coalescing", |b| {
        b.iter(|| {
            let mut dirty = DirtyTracker::default();
            for row in 0..36 {
                dirty.mark_span(row, 0, 80);
                dirty.mark_cell(row, row % 120);
                dirty.mark_region(DirtyRegion {
                    row,
                    col: 40,
                    rows: 1,
                    cols: 40,
                });
            }
            black_box(dirty.contains_region(DirtyRegion {
                row: 0,
                col: 0,
                rows: 36,
                cols: 80,
            }));
            black_box(dirty.take());
        });
    });
}

fn render_plan_large_dirty_region(c: &mut Criterion) {
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

fn glyph_quad_generation_large_plan(c: &mut Criterion) {
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

fn rasterized_glyph_cache_hot_plan(c: &mut Criterion) {
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
    let mut glyph_cache = load_default_native_glyph_cache().unwrap();
    glyph_cache.rasterize_plan(&plan).unwrap();

    c.bench_function("rasterized_glyph_cache_hot_plan", |b| {
        b.iter(|| {
            let batch = glyph_cache.rasterize_plan(black_box(&plan)).unwrap();
            black_box(batch.bitmaps.len());
            black_box(batch.reused);
        });
    });
}

fn prepared_surface_glyph_frame_large_plan(c: &mut Criterion) {
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
    let mut glyph_cache = load_default_native_glyph_cache().unwrap();
    let glyphs = glyph_cache.rasterize_plan(&plan).unwrap();

    c.bench_function("prepared_surface_glyph_frame_large_plan", |b| {
        b.iter(|| {
            let prepared = PreparedSurfaceGlyphFrame::from_render_plan(
                black_box(&plan),
                black_box(&glyphs.bitmaps),
                black_box([0.0, 0.0, 0.0, 1.0]),
            )
            .unwrap();
            let frame = prepared.as_surface_glyph_frame();
            black_box(frame.batch.quads.len());
            black_box(frame.batch.indices.len());
            black_box(frame.atlas.rgba.len());
        });
    });
}

fn font_rasterizer_combining_cell(c: &mut Criterion) {
    let font_bytes = bench_monospace_font_bytes();
    let mut rasterizer = FontRasterizer::from_bytes(font_bytes).unwrap();
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

fn pty_runtime_pump_large_output(c: &mut Criterion) {
    c.bench_function("pty_runtime_pump_large_output", |b| {
        b.iter(|| {
            let spawner = BenchPtySpawner { chunks: 256 };
            let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
                terminal_cols: 120,
                terminal_rows: 36,
                scrollback_lines: 20_000,
                pixel_width: 0,
                pixel_height: 0,
                shell: ShellCommand {
                    program: "/bin/sh".into(),
                    args: Vec::new(),
                    cwd: None,
                },
            })
            .unwrap();
            runtime.start_shell(&spawner).unwrap();

            let mut bytes = 0;
            loop {
                let pumped = runtime.pump_pty_output().unwrap();
                if pumped == 0 {
                    break;
                }
                bytes += pumped;
            }

            black_box(bytes);
            black_box(runtime.terminal().dump_perf_metrics());
        });
    });
}

criterion_group!(
    benches,
    parser_large_output,
    scrollback_large_output,
    dirty_region_coalescing,
    render_plan_large_dirty_region,
    glyph_quad_generation_large_plan,
    rasterized_glyph_cache_hot_plan,
    prepared_surface_glyph_frame_large_plan,
    font_rasterizer_combining_cell,
    pty_runtime_pump_large_output
);
criterion_main!(benches);

fn bench_monospace_font_bytes() -> Vec<u8> {
    let Some(path) = BENCH_MONOSPACE_FONT_CANDIDATES
        .iter()
        .map(Path::new)
        .find(|path| path.exists())
    else {
        panic!("expected a local monospace font for font rasterization benchmark");
    };
    std::fs::read(path).expect("expected readable monospace font bytes")
}
