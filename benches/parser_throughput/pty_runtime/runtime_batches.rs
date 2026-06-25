use std::hint::black_box;

use criterion::Criterion;
use gromaq::app::{NativeTerminalRuntime, NativeTerminalRuntimeConfig};
use gromaq::pty::ShellCommand;
use gromaq::renderer::{RendererConfig, WgpuRenderer};
use gromaq::{MouseButton, MouseEvent, MouseEventKind};

use crate::support::{
    ALTERNATE_SCREEN_STAGES, BOUNDED_STATE_BATCHES, BOUNDED_STATE_SCROLLBACK_LINES,
    BenchPayloadPtySpawner, CONTINUOUS_OUTPUT_BATCHES, CONTINUOUS_OUTPUT_SCROLLBACK_LINES,
    RUNTIME_PROTOCOL_INPUT_PAYLOAD, alternate_screen_payloads, bounded_state_payloads,
    continuous_output_payloads,
};

pub(crate) fn runtime_bounded_state_batches(c: &mut Criterion) {
    let payloads = bounded_state_payloads();
    c.bench_function("runtime_bounded_state_batches", |b| {
        b.iter(|| {
            let spawner = BenchPayloadPtySpawner {
                payloads: payloads.clone(),
            };
            let mut runtime =
                NativeTerminalRuntime::new(runtime_config(32, 8, BOUNDED_STATE_SCROLLBACK_LINES))
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

pub(crate) fn runtime_state_snapshot_bounded_session(c: &mut Criterion) {
    let payloads = bounded_state_payloads();
    let spawner = BenchPayloadPtySpawner { payloads };
    let mut runtime =
        NativeTerminalRuntime::new(runtime_config(32, 8, BOUNDED_STATE_SCROLLBACK_LINES)).unwrap();
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

pub(crate) fn runtime_continuous_output_batches(c: &mut Criterion) {
    let payloads = continuous_output_payloads();
    c.bench_function("runtime_continuous_output_batches", |b| {
        b.iter(|| {
            let spawner = BenchPayloadPtySpawner {
                payloads: payloads.clone(),
            };
            let mut runtime = NativeTerminalRuntime::new(runtime_config(
                32,
                8,
                CONTINUOUS_OUTPUT_SCROLLBACK_LINES,
            ))
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

pub(crate) fn runtime_alternate_screen_stages(c: &mut Criterion) {
    let payloads = alternate_screen_payloads();
    c.bench_function("runtime_alternate_screen_stages", |b| {
        b.iter(|| {
            let spawner = BenchPayloadPtySpawner {
                payloads: payloads.clone(),
            };
            let mut runtime = NativeTerminalRuntime::new(runtime_config(24, 4, 16)).unwrap();
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

pub(crate) fn runtime_protocol_input_reports(c: &mut Criterion) {
    let payloads = vec![RUNTIME_PROTOCOL_INPUT_PAYLOAD.to_vec()];
    c.bench_function("runtime_protocol_input_reports", |b| {
        b.iter(|| {
            let spawner = BenchPayloadPtySpawner {
                payloads: payloads.clone(),
            };
            let mut runtime = NativeTerminalRuntime::new(runtime_config(24, 4, 128)).unwrap();
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

fn runtime_config(
    terminal_cols: u16,
    terminal_rows: u16,
    scrollback_lines: usize,
) -> NativeTerminalRuntimeConfig {
    NativeTerminalRuntimeConfig {
        terminal_cols,
        terminal_rows,
        scrollback_lines,
        pixel_width: 0,
        pixel_height: 0,
        cursor_shape: NativeTerminalRuntimeConfig::default().cursor_shape,
        cursor_blinking: NativeTerminalRuntimeConfig::default().cursor_blinking,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    }
}
