use std::hint::black_box;
use std::path::Path;
use std::time::{Duration, Instant};

use criterion::Criterion;
use gromaq::app::{NativeTerminalRuntime, NativeTerminalRuntimeConfig};
use gromaq::pty::{PtyConfig, PtySession, ShellCommand};
use gromaq::renderer::{RendererConfig, WgpuRenderer};
use gromaq::{MouseButton, MouseEvent, MouseEventKind};

use crate::support::{
    ALTERNATE_SCREEN_STAGES, BOUNDED_STATE_BATCHES, BOUNDED_STATE_SCROLLBACK_LINES,
    BenchPayloadPtySpawner, BenchPtySpawner, CONTINUOUS_OUTPUT_BATCHES,
    CONTINUOUS_OUTPUT_SCROLLBACK_LINES, REAL_PTY_BENCH_LINES, RUNTIME_PROTOCOL_INPUT_PAYLOAD,
    alternate_screen_payloads, bounded_state_payloads, contains_bytes, continuous_output_payloads,
    real_pty_large_output_script, skip_benchmark,
};

pub(crate) fn pty_runtime_pump_large_output(c: &mut Criterion) {
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

pub(crate) fn real_pty_shell_large_output_burst(c: &mut Criterion) {
    if !Path::new("/bin/sh").exists() {
        skip_benchmark(c, "real_pty_shell_large_output_burst", "/bin/sh not found");
        return;
    }

    c.bench_function("real_pty_shell_large_output_burst", |b| {
        b.iter(|| {
            let mut session = PtySession::spawn(PtyConfig {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
                shell: ShellCommand {
                    program: "/bin/sh".into(),
                    args: vec!["-lc".into(), real_pty_large_output_script().into()],
                    cwd: None,
                },
            })
            .unwrap();
            session.start_output_reader().unwrap();

            let marker = format!("gromaq-real-pty-{:04}", REAL_PTY_BENCH_LINES - 1);
            let mut output = Vec::new();
            let deadline = Instant::now() + Duration::from_secs(5);
            while Instant::now() < deadline {
                output.extend(session.drain_available_output().unwrap());
                if contains_bytes(&output, marker.as_bytes()) {
                    break;
                }
                std::thread::sleep(Duration::from_millis(1));
            }

            assert!(
                contains_bytes(&output, marker.as_bytes()),
                "real PTY benchmark did not observe {marker}"
            );
            assert!(
                session
                    .wait_timeout(Duration::from_secs(5))
                    .unwrap()
                    .is_some()
            );
            black_box(output.len());
        });
    });
}

pub(crate) fn real_pty_shell_input_echo_roundtrip(c: &mut Criterion) {
    if !Path::new("/bin/sh").exists() {
        skip_benchmark(
            c,
            "real_pty_shell_input_echo_roundtrip",
            "/bin/sh not found",
        );
        return;
    }

    c.bench_function("real_pty_shell_input_echo_roundtrip", |b| {
        b.iter(|| {
            let mut session = PtySession::spawn(PtyConfig {
                rows: 8,
                cols: 40,
                pixel_width: 0,
                pixel_height: 0,
                shell: ShellCommand {
                    program: "/bin/sh".into(),
                    args: Vec::new(),
                    cwd: None,
                },
            })
            .unwrap();
            session.start_output_reader().unwrap();
            session
                .write_all(b"printf 'gromaq-real-pty-input\\n'\nexit\n")
                .unwrap();

            let marker = b"gromaq-real-pty-input";
            let mut output = Vec::new();
            let deadline = Instant::now() + Duration::from_secs(5);
            while Instant::now() < deadline {
                output.extend(session.drain_available_output().unwrap());
                if contains_bytes(&output, marker) {
                    break;
                }
                std::thread::sleep(Duration::from_millis(1));
            }

            assert!(
                contains_bytes(&output, marker),
                "real PTY benchmark did not observe input echo output"
            );
            assert!(
                session
                    .wait_timeout(Duration::from_secs(5))
                    .unwrap()
                    .is_some()
            );
            black_box(output.len());
        });
    });
}

pub(crate) fn runtime_bounded_state_batches(c: &mut Criterion) {
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
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 32,
        terminal_rows: 8,
        scrollback_lines: BOUNDED_STATE_SCROLLBACK_LINES,
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
            let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
                terminal_cols: 32,
                terminal_rows: 8,
                scrollback_lines: CONTINUOUS_OUTPUT_SCROLLBACK_LINES,
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
            let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
                terminal_cols: 24,
                terminal_rows: 4,
                scrollback_lines: 16,
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
            let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
                terminal_cols: 24,
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
