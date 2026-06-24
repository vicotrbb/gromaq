use winit::keyboard::{Key, ModifiersState};

use crate::app::{NativeTerminalRuntime, NativeTerminalRuntimeConfig};
use crate::cli::CliExit;
use crate::pty::ShellCommand;
use crate::renderer::{RendererConfig, WgpuRenderer};

use super::pty_smoke::RuntimePerfSmokePtySpawner;

const RUNTIME_IDLE_SMOKE_RENDER_ATTEMPTS: u64 = 16;

pub(in crate::cli) fn runtime_perf_smoke_exit() -> CliExit {
    let mut runtime = match NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
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
    }) {
        Ok(runtime) => runtime,
        Err(error) => return runtime_perf_smoke_error(error),
    };
    if let Err(error) = runtime.start_shell(&RuntimePerfSmokePtySpawner) {
        return runtime_perf_smoke_error(error);
    }

    let key = Key::Character("x".into());
    let sent = match runtime.send_winit_key_input(&key, ModifiersState::empty()) {
        Ok(sent) => sent,
        Err(error) => return runtime_perf_smoke_error(error),
    };
    let pumped_bytes = match runtime.pump_pty_output() {
        Ok(bytes) => bytes,
        Err(error) => return runtime_perf_smoke_error(error),
    };
    let mut renderer = match WgpuRenderer::new(RendererConfig::default()) {
        Ok(renderer) => renderer,
        Err(error) => return runtime_perf_smoke_error(error),
    };
    let rendered = match runtime.render_terminal_frame(&mut renderer) {
        Ok(rendered) => rendered,
        Err(error) => return runtime_perf_smoke_error(error),
    };
    let metrics = runtime.dump_runtime_perf_metrics();

    if !sent
        || pumped_bytes == 0
        || !rendered
        || metrics.rendered_dirty_regions == 0
        || metrics.rendered_dirty_cells == 0
        || metrics.render_time_samples == 0
        || metrics.input_to_render_samples == 0
    {
        return CliExit {
            code: 1,
            stdout: String::new(),
            stderr: "runtime perf smoke failed: input echo did not reach a rendered frame\n"
                .to_owned(),
        };
    }

    CliExit {
        code: 0,
        stdout: format!(
            "runtime perf smoke: ok\npumped bytes: {}\nrendered frames: {}\nrendered dirty regions: {}\nrendered dirty cells: {}\nrendered dirty cells max: {}\nrender samples: {}\nrender avg ns: {}\nrender max ns: {}\nrender p95 ns: {}\ninput-to-render samples: {}\ninput-to-render avg ns: {}\ninput-to-render max ns: {}\ninput-to-render p95 ns: {}\n",
            pumped_bytes,
            metrics.rendered_frames,
            metrics.rendered_dirty_regions,
            metrics.rendered_dirty_cells,
            metrics.rendered_dirty_cells_max,
            metrics.render_time_samples,
            metrics.render_time_avg_ns,
            metrics.render_time_max_ns,
            metrics.render_time_p95_ns,
            metrics.input_to_render_samples,
            metrics.input_to_render_avg_ns,
            metrics.input_to_render_max_ns,
            metrics.input_to_render_p95_ns
        ),
        stderr: String::new(),
    }
}

fn runtime_perf_smoke_error(error: impl std::fmt::Display) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime perf smoke failed: {error}\n"),
    }
}

pub(in crate::cli) fn runtime_idle_smoke_exit() -> CliExit {
    let mut runtime = match NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
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
    }) {
        Ok(runtime) => runtime,
        Err(error) => return runtime_idle_smoke_error(error),
    };
    if let Err(error) = runtime.start_shell(&RuntimePerfSmokePtySpawner) {
        return runtime_idle_smoke_error(error);
    }
    let pumped_bytes = match runtime.pump_pty_output() {
        Ok(bytes) => bytes,
        Err(error) => return runtime_idle_smoke_error(error),
    };
    let mut renderer = match WgpuRenderer::new(RendererConfig::default()) {
        Ok(renderer) => renderer,
        Err(error) => return runtime_idle_smoke_error(error),
    };
    for _ in 0..RUNTIME_IDLE_SMOKE_RENDER_ATTEMPTS {
        let rendered = match runtime.render_terminal_frame(&mut renderer) {
            Ok(rendered) => rendered,
            Err(error) => return runtime_idle_smoke_error(error),
        };
        if rendered {
            return runtime_idle_smoke_failure("clean runtime produced a rendered frame");
        }
    }
    let metrics = runtime.dump_runtime_perf_metrics();
    if pumped_bytes != 0
        || metrics.pty_output_batches != 0
        || metrics.pty_output_bytes != 0
        || metrics.render_attempts != RUNTIME_IDLE_SMOKE_RENDER_ATTEMPTS
        || metrics.clean_frame_skips != RUNTIME_IDLE_SMOKE_RENDER_ATTEMPTS
        || metrics.rendered_frames != 0
        || metrics.render_time_samples != 0
        || metrics.input_to_render_samples != 0
    {
        return runtime_idle_smoke_failure(
            "idle runtime counters did not prove clean-frame suppression",
        );
    }

    CliExit {
        code: 0,
        stdout: format!(
            "runtime idle smoke: ok\npumped bytes: {}\nrender attempts: {}\nclean frame skips: {}\nrendered frames: {}\n",
            pumped_bytes,
            metrics.render_attempts,
            metrics.clean_frame_skips,
            metrics.rendered_frames
        ),
        stderr: String::new(),
    }
}

fn runtime_idle_smoke_error(error: impl std::fmt::Display) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime idle smoke failed: {error}\n"),
    }
}

fn runtime_idle_smoke_failure(reason: &str) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime idle smoke failed: {reason}\n"),
    }
}
