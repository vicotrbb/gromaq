use std::hint::black_box;

use criterion::Criterion;
use gromaq::app::{NativeTerminalRuntime, NativeTerminalRuntimeConfig};
use gromaq::pty::ShellCommand;

use crate::support::BenchPtySpawner;

#[path = "pty_runtime/real_shell.rs"]
mod real_shell;
#[path = "pty_runtime/runtime_batches.rs"]
mod runtime_batches;

pub(crate) use real_shell::{
    real_pty_shell_input_echo_roundtrip, real_pty_shell_large_output_burst,
};
pub(crate) use runtime_batches::{
    runtime_alternate_screen_stages, runtime_bounded_state_batches,
    runtime_continuous_output_batches, runtime_protocol_input_reports,
    runtime_state_snapshot_bounded_session,
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
