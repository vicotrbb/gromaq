use winit::keyboard::{Key, ModifiersState};

use crate::app::{NativeRuntimePerfSnapshot, NativeTerminalRuntime, NativeTerminalRuntimeConfig};
use crate::pty::ShellCommand;
use crate::renderer::{RendererConfig, WgpuRenderer};

use super::super::pty_smoke::RuntimePerfSmokePtySpawner;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct RuntimePerfProbe {
    pub(super) pumped_bytes: usize,
    pub(super) expected_samples: usize,
    pub(super) metrics: NativeRuntimePerfSnapshot,
}

pub(super) fn run_runtime_perf_probe(samples: usize) -> Result<RuntimePerfProbe, String> {
    if samples == 0 {
        return Err("runtime perf probe requires at least one sample".to_owned());
    }
    let mut runtime = match NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
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
    }) {
        Ok(runtime) => runtime,
        Err(error) => return Err(error.to_string()),
    };
    if let Err(error) = runtime.start_shell(&RuntimePerfSmokePtySpawner) {
        return Err(error.to_string());
    }

    let mut renderer = match WgpuRenderer::new(RendererConfig::default()) {
        Ok(renderer) => renderer,
        Err(error) => return Err(error.to_string()),
    };
    let mut pumped_bytes = 0;
    for sample in 0..samples {
        let key = Key::Character(sample_key(sample).to_string().into());
        let sent = match runtime.send_winit_key_input(&key, ModifiersState::empty()) {
            Ok(sent) => sent,
            Err(error) => return Err(error.to_string()),
        };
        let pumped = match runtime.pump_pty_output() {
            Ok(bytes) => bytes,
            Err(error) => return Err(error.to_string()),
        };
        let rendered = match runtime.render_terminal_frame(&mut renderer) {
            Ok(rendered) => rendered,
            Err(error) => return Err(error.to_string()),
        };
        if !sent || pumped == 0 || !rendered {
            return Err(format!(
                "input echo sample {} did not reach a rendered frame",
                sample + 1
            ));
        }
        pumped_bytes += pumped;
    }
    let metrics = runtime.dump_runtime_perf_metrics();

    if metrics.rendered_dirty_regions == 0
        || metrics.rendered_dirty_cells == 0
        || metrics.render_time_samples != samples as u64
        || metrics.input_to_render_samples != samples as u64
    {
        return Err("input echo did not produce the expected performance samples".to_owned());
    }

    Ok(RuntimePerfProbe {
        pumped_bytes,
        expected_samples: samples,
        metrics,
    })
}

fn sample_key(sample: usize) -> char {
    char::from(b'a' + u8::try_from(sample % 26).unwrap_or(0))
}
