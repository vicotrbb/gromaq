use std::collections::VecDeque;
use std::hint::black_box;
use std::path::Path;
use std::time::{Duration, Instant};

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use gromaq::app::{
    NativePtyResize, NativePtySessionIo, NativePtySpawner, NativeTerminalRuntime,
    NativeTerminalRuntimeConfig, load_default_native_glyph_cache,
};
use gromaq::font::FontRasterizer;
use gromaq::pty::{PtyConfig, PtyError, ShellCommand};
use gromaq::renderer::{
    FrameScheduler, GlyphAtlas, GlyphAtlasConfig, GlyphEntry, GlyphQuadConfig, GlyphQuadPlanner,
    PreparedSurfaceGlyphFrame, RenderPlanner, RendererConfig, WgpuRenderer,
};
use gromaq::{
    DirtyRegion, DirtyTracker, MouseButton, MouseEvent, MouseEventKind, Terminal, TerminalConfig,
};
use winit::keyboard::{Key, ModifiersState};

const LARGE_OUTPUT: &str = "\
\x1b[31;1merror\x1b[0m line one\r\n\
normal log line with unicode 界 and attributes\r\n\
\x1b[32mok\x1b[0m line three\r\n\
";

const ASCII_RENDER_OUTPUT: &str = "\
error status 0123456789 ABC xyz\r\n\
normal log line with attributes\r\n\
prompt $ cargo test --all\r\n\
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

const BOUNDED_STATE_BATCHES: usize = 4;
const BOUNDED_STATE_LINES_PER_BATCH: usize = 512;
const BOUNDED_STATE_SCROLLBACK_LINES: usize = 128;
const CONTINUOUS_OUTPUT_BATCHES: usize = 32;
const CONTINUOUS_OUTPUT_LINES_PER_BATCH: usize = 8;
const CONTINUOUS_OUTPUT_SCROLLBACK_LINES: usize = 64;
const SCROLLBACK_NAVIGATION_LINES: usize = 4_096;
const SCROLLBACK_NAVIGATION_STEPS: usize = 512;
const ALTERNATE_SCREEN_STAGES: usize = 3;
const FRAME_SCHEDULER_TIMELINE_STEPS: usize = 512;
const RUNTIME_PROTOCOL_INPUT_PAYLOAD: &[u8] =
    b"\x1b[?1004h\x1b[?1000h\x1b[?1006h\x1b[3;5H\x1b[6n\x1b[5n\x1b[c\x1b[>c";

#[derive(Debug)]
struct BenchPtySession {
    output: VecDeque<Vec<u8>>,
    echo_input: bool,
}

#[derive(Debug)]
struct BenchPayloadPtySession {
    output: VecDeque<Vec<u8>>,
}

impl NativePtySessionIo for BenchPayloadPtySession {
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

impl NativePtySessionIo for BenchPtySession {
    fn drain_output(&mut self) -> Result<Vec<u8>, PtyError> {
        Ok(self.output.pop_front().unwrap_or_default())
    }

    fn write_input(&mut self, bytes: &[u8]) -> Result<(), PtyError> {
        if self.echo_input {
            self.output.push_back(bytes.to_vec());
        }
        Ok(())
    }

    fn resize(&mut self, _size: NativePtyResize) -> Result<(), PtyError> {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
struct BenchPtySpawner {
    chunks: usize,
    echo_input: bool,
}

#[derive(Debug, Clone)]
struct BenchPayloadPtySpawner {
    payloads: Vec<Vec<u8>>,
}

impl NativePtySpawner for BenchPayloadPtySpawner {
    type Session = BenchPayloadPtySession;

    fn spawn(&self, _config: PtyConfig) -> Result<Self::Session, PtyError> {
        Ok(BenchPayloadPtySession {
            output: VecDeque::from(self.payloads.clone()),
        })
    }
}

impl NativePtySpawner for BenchPtySpawner {
    type Session = BenchPtySession;

    fn spawn(&self, _config: PtyConfig) -> Result<Self::Session, PtyError> {
        let mut output = VecDeque::with_capacity(self.chunks);
        for _ in 0..self.chunks {
            output.push_back(LARGE_OUTPUT.as_bytes().to_vec());
        }
        Ok(BenchPtySession {
            output,
            echo_input: self.echo_input,
        })
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
        write!(&mut output, "line {line:04}\r\n").expect("writing to a String is infallible");
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

fn scrollback_view_navigation(c: &mut Criterion) {
    let payload = scrollback_navigation_payload();

    c.bench_function("scrollback_view_navigation", |b| {
        b.iter_batched(
            || {
                let mut terminal = Terminal::new(
                    TerminalConfig::new(80, 24)
                        .unwrap()
                        .with_scrollback_limit(SCROLLBACK_NAVIGATION_LINES)
                        .unwrap(),
                );
                terminal.write_bytes(&payload).unwrap();
                terminal.take_dirty_regions();
                terminal
            },
            |mut terminal| {
                let mut moved_rows = 0_usize;

                for _ in 0..SCROLLBACK_NAVIGATION_STEPS {
                    moved_rows += usize::from(terminal.scroll_display_up(1));
                    let grid = terminal.dump_grid();
                    black_box(grid.line_text(0));
                    black_box(terminal.take_dirty_regions());
                }

                for _ in 0..SCROLLBACK_NAVIGATION_STEPS {
                    moved_rows += usize::from(terminal.scroll_display_down(1));
                    let grid = terminal.dump_grid();
                    black_box(grid.line_text(0));
                    black_box(terminal.take_dirty_regions());
                }

                black_box(moved_rows);
                black_box(terminal.dump_perf_metrics());
            },
            BatchSize::SmallInput,
        );
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

fn frame_scheduler_144hz_timeline(c: &mut Criterion) {
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

fn native_input_echo_render_cycle(c: &mut Criterion) {
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

fn font_rasterizer_combining_cell(c: &mut Criterion) {
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

fn pty_runtime_pump_large_output(c: &mut Criterion) {
    c.bench_function("pty_runtime_pump_large_output", |b| {
        b.iter(|| {
            let spawner = BenchPtySpawner {
                chunks: 256,
                echo_input: false,
            };
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

fn runtime_bounded_state_batches(c: &mut Criterion) {
    let payloads = bounded_state_payloads();
    c.bench_function("runtime_bounded_state_batches", |b| {
        b.iter(|| {
            let spawner = BenchPayloadPtySpawner {
                payloads: payloads.clone(),
            };
            let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
                terminal_cols: 32,
                terminal_rows: 8,
                scrollback_lines: BOUNDED_STATE_SCROLLBACK_LINES,
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
            let mut renderer = WgpuRenderer::new(RendererConfig::default()).unwrap();
            let mut bytes = 0_usize;
            let mut frames = 0_u64;

            for _ in 0..BOUNDED_STATE_BATCHES {
                let pumped = runtime.pump_pty_output().unwrap();
                bytes = bytes.saturating_add(pumped);
                if runtime.render_terminal_frame(&mut renderer).unwrap() {
                    frames += 1;
                }
            }

            let scrollback = runtime.terminal().dump_scrollback();
            black_box(bytes);
            black_box(frames);
            black_box(scrollback.lines.len());
            black_box(runtime.dump_runtime_perf_metrics());
        });
    });
}

fn runtime_state_snapshot_bounded_session(c: &mut Criterion) {
    let payloads = bounded_state_payloads();
    let spawner = BenchPayloadPtySpawner { payloads };
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 32,
        terminal_rows: 8,
        scrollback_lines: BOUNDED_STATE_SCROLLBACK_LINES,
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
    for _ in 0..BOUNDED_STATE_BATCHES {
        black_box(runtime.pump_pty_output().unwrap());
    }

    c.bench_function("runtime_state_snapshot_bounded_session", |b| {
        b.iter(|| {
            let snapshot = runtime.dump_runtime_state_snapshot();
            black_box(snapshot.scrollback_lines);
            black_box(snapshot.scrollback_cells);
            black_box(snapshot.scrollback_cell_limit);
        });
    });
}

fn runtime_continuous_output_batches(c: &mut Criterion) {
    let payloads = continuous_output_payloads();
    c.bench_function("runtime_continuous_output_batches", |b| {
        b.iter(|| {
            let spawner = BenchPayloadPtySpawner {
                payloads: payloads.clone(),
            };
            let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
                terminal_cols: 32,
                terminal_rows: 8,
                scrollback_lines: CONTINUOUS_OUTPUT_SCROLLBACK_LINES,
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
            let mut renderer = WgpuRenderer::new(RendererConfig::default()).unwrap();
            let mut bytes = 0_usize;
            let mut frames = 0_u64;

            for _ in 0..CONTINUOUS_OUTPUT_BATCHES {
                let pumped = runtime.pump_pty_output().unwrap();
                bytes = bytes.saturating_add(pumped);
                if runtime.render_terminal_frame(&mut renderer).unwrap() {
                    frames += 1;
                }
            }

            let scrollback = runtime.terminal().dump_scrollback();
            black_box(bytes);
            black_box(frames);
            black_box(scrollback.lines.len());
            black_box(runtime.dump_runtime_perf_metrics());
        });
    });
}

fn runtime_alternate_screen_stages(c: &mut Criterion) {
    let payloads = alternate_screen_payloads();
    c.bench_function("runtime_alternate_screen_stages", |b| {
        b.iter(|| {
            let spawner = BenchPayloadPtySpawner {
                payloads: payloads.clone(),
            };
            let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
                terminal_cols: 24,
                terminal_rows: 4,
                scrollback_lines: 16,
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
            let mut renderer = WgpuRenderer::new(RendererConfig::default()).unwrap();
            let mut bytes = 0_usize;
            let mut frames = 0_u64;
            let mut alt_rendered = false;

            for stage in 0..ALTERNATE_SCREEN_STAGES {
                let pumped = runtime.pump_pty_output().unwrap();
                bytes = bytes.saturating_add(pumped);
                if runtime.render_terminal_frame(&mut renderer).unwrap() {
                    frames += 1;
                }
                if stage == 1 {
                    alt_rendered = renderer
                        .last_plan()
                        .map(|plan| {
                            plan.glyphs
                                .iter()
                                .map(|glyph| glyph.text.as_str())
                                .collect::<String>()
                                .contains("alt-view")
                        })
                        .unwrap_or(false);
                }
            }

            let grid = runtime.terminal().dump_grid();
            let scrollback = runtime.terminal().dump_scrollback();
            black_box(bytes);
            black_box(frames);
            black_box(alt_rendered);
            black_box(grid.line_text(0));
            black_box(grid.line_text(1));
            black_box(scrollback.lines.len());
            black_box(runtime.dump_runtime_perf_metrics());
        });
    });
}

fn runtime_protocol_input_reports(c: &mut Criterion) {
    let payloads = vec![RUNTIME_PROTOCOL_INPUT_PAYLOAD.to_vec()];
    c.bench_function("runtime_protocol_input_reports", |b| {
        b.iter(|| {
            let spawner = BenchPayloadPtySpawner {
                payloads: payloads.clone(),
            };
            let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
                terminal_cols: 24,
                terminal_rows: 4,
                scrollback_lines: 128,
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

            let pumped = runtime.pump_pty_output().unwrap();
            let focused = runtime.send_focus_event(true).unwrap();
            let blurred = runtime.send_focus_event(false).unwrap();
            let pressed = runtime
                .send_mouse_input(MouseEvent::new(
                    MouseEventKind::Press,
                    MouseButton::Left,
                    2,
                    1,
                ))
                .unwrap();
            let released = runtime
                .send_mouse_input(MouseEvent::new(
                    MouseEventKind::Release,
                    MouseButton::Left,
                    2,
                    1,
                ))
                .unwrap();
            let wheel = runtime
                .send_mouse_input(MouseEvent::new(
                    MouseEventKind::Press,
                    MouseButton::WheelUp,
                    0,
                    0,
                ))
                .unwrap();
            let metrics = runtime.dump_runtime_perf_metrics();

            black_box(pumped);
            black_box(focused);
            black_box(blurred);
            black_box(pressed);
            black_box(released);
            black_box(wheel);
            black_box(metrics.pty_response_writes);
            black_box(metrics.pty_response_bytes);
            black_box(metrics.focus_inputs);
            black_box(metrics.mouse_inputs);
            black_box(metrics.pty_input_writes);
            black_box(metrics.pty_input_bytes);
        });
    });
}

criterion_group!(
    benches,
    parser_large_output,
    scrollback_large_output,
    scrollback_view_navigation,
    dirty_region_coalescing,
    frame_scheduler_144hz_timeline,
    render_plan_large_dirty_region,
    glyph_quad_generation_large_plan,
    rasterized_glyph_cache_hot_plan,
    prepared_surface_glyph_frame_large_plan,
    native_input_echo_render_cycle,
    font_rasterizer_combining_cell,
    pty_runtime_pump_large_output,
    runtime_bounded_state_batches,
    runtime_state_snapshot_bounded_session,
    runtime_continuous_output_batches,
    runtime_alternate_screen_stages,
    runtime_protocol_input_reports
);
criterion_main!(benches);

fn skip_benchmark(c: &mut Criterion, name: &'static str, reason: &str) {
    eprintln!("skipping {name}: {reason}");
    c.bench_function(name, |b| b.iter(|| black_box(())));
}

fn bench_monospace_font_bytes() -> Result<Vec<u8>, String> {
    let Some(path) = BENCH_MONOSPACE_FONT_CANDIDATES
        .iter()
        .map(Path::new)
        .find(|path| path.exists())
    else {
        return Err("no local monospace font candidate found".to_owned());
    };
    std::fs::read(path).map_err(|error| {
        format!(
            "failed to read monospace font candidate {}: {error}",
            path.display()
        )
    })
}

fn bounded_state_payloads() -> Vec<Vec<u8>> {
    (0..BOUNDED_STATE_BATCHES)
        .map(|batch| {
            let start = batch * BOUNDED_STATE_LINES_PER_BATCH;
            let end = start + BOUNDED_STATE_LINES_PER_BATCH;
            let mut payload = Vec::new();
            for line in start..end {
                payload.extend_from_slice(format!("gromaq-bounded-line-{line:04}\n").as_bytes());
            }
            payload
        })
        .collect()
}

fn continuous_output_payloads() -> Vec<Vec<u8>> {
    (0..CONTINUOUS_OUTPUT_BATCHES)
        .map(|batch| {
            let start = batch * CONTINUOUS_OUTPUT_LINES_PER_BATCH;
            let end = start + CONTINUOUS_OUTPUT_LINES_PER_BATCH;
            let mut payload = Vec::new();
            for line in start..end {
                payload.extend_from_slice(format!("gromaq-continuous-line-{line:03}\n").as_bytes());
            }
            payload
        })
        .collect()
}

fn scrollback_navigation_payload() -> Vec<u8> {
    let mut payload = Vec::new();
    for line in 0..SCROLLBACK_NAVIGATION_LINES {
        payload.extend_from_slice(format!("gromaq-scrollback-nav-line-{line:04}\n").as_bytes());
    }
    payload
}

fn alternate_screen_payloads() -> Vec<Vec<u8>> {
    vec![
        b"primary\n".to_vec(),
        b"\x1b[?1049halt-view\n".to_vec(),
        b"\x1b[?1049lrestored\n".to_vec(),
    ]
}
