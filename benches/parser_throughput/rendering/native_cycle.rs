use std::hint::black_box;

use criterion::Criterion;
use gromaq::app::{NativeTerminalRuntime, NativeTerminalRuntimeConfig};
use gromaq::pty::ShellCommand;
use gromaq::renderer::{RendererConfig, WgpuRenderer};
use winit::keyboard::{Key, ModifiersState};

use crate::support::BenchPtySpawner;

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
