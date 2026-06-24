//! Runtime input and performance CLI smoke commands.

use winit::keyboard::{Key, ModifiersState};

use super::CliExit;
use crate::app::{NativeTerminalRuntime, NativeTerminalRuntimeConfig};
use crate::mouse::{MouseButton, MouseEvent, MouseEventKind};
use crate::pty::ShellCommand;
use crate::renderer::{RendererConfig, WgpuRenderer};
use pty_smoke::{RuntimeInputCaptureSmokePtySpawner, RuntimePerfSmokePtySpawner};

mod pty_smoke;

const RUNTIME_FOCUS_SMOKE_ENABLE_REPORTING: &str = "\x1b[?1004h";
const RUNTIME_MOUSE_SMOKE_ENABLE_REPORTING: &str = "\x1b[?1000h\x1b[?1006h";
const RUNTIME_RESPONSE_SMOKE_QUERIES: &str = "\x1b[3;5H\x1b[6n\x1b[5n\x1b[c\x1b[>c";
const RUNTIME_IDLE_SMOKE_RENDER_ATTEMPTS: u64 = 16;

pub(super) fn runtime_perf_smoke_exit() -> CliExit {
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

pub(super) fn runtime_focus_smoke_exit() -> CliExit {
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
        Err(error) => return runtime_focus_smoke_error(error),
    };
    let spawner =
        RuntimeInputCaptureSmokePtySpawner::new(RUNTIME_FOCUS_SMOKE_ENABLE_REPORTING.as_bytes());
    if let Err(error) = runtime.start_shell(&spawner) {
        return runtime_focus_smoke_error(error);
    }

    let pumped_bytes = match runtime.pump_pty_output() {
        Ok(bytes) => bytes,
        Err(error) => return runtime_focus_smoke_error(error),
    };
    let focused = match runtime.send_focus_event(true) {
        Ok(focused) => focused,
        Err(error) => return runtime_focus_smoke_error(error),
    };
    let blurred = match runtime.send_focus_event(false) {
        Ok(blurred) => blurred,
        Err(error) => return runtime_focus_smoke_error(error),
    };
    let metrics = runtime.dump_runtime_perf_metrics();
    let input = runtime
        .shell_session()
        .map(|session| session.input.concat())
        .unwrap_or_default();

    if pumped_bytes != RUNTIME_FOCUS_SMOKE_ENABLE_REPORTING.len()
        || !focused
        || !blurred
        || input != b"\x1b[I\x1b[O"
        || metrics.focus_inputs != 2
        || metrics.pty_input_writes != 2
        || metrics.pty_input_bytes != 6
    {
        return CliExit {
            code: 1,
            stdout: String::new(),
            stderr: "runtime focus smoke failed: focus reports did not reach PTY writes\n"
                .to_owned(),
        };
    }

    CliExit {
        code: 0,
        stdout: format!(
            "runtime focus smoke: ok\npumped bytes: {}\nfocus in reported: {}\nfocus out reported: {}\nfocus inputs: {}\npty input writes: {}\npty input bytes: {}\n",
            pumped_bytes,
            focused,
            blurred,
            metrics.focus_inputs,
            metrics.pty_input_writes,
            metrics.pty_input_bytes
        ),
        stderr: String::new(),
    }
}

fn runtime_focus_smoke_error(error: impl std::fmt::Display) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime focus smoke failed: {error}\n"),
    }
}

pub(super) fn runtime_mouse_smoke_exit() -> CliExit {
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
        Err(error) => return runtime_mouse_smoke_error(error),
    };
    let spawner =
        RuntimeInputCaptureSmokePtySpawner::new(RUNTIME_MOUSE_SMOKE_ENABLE_REPORTING.as_bytes());
    if let Err(error) = runtime.start_shell(&spawner) {
        return runtime_mouse_smoke_error(error);
    }

    let pumped_bytes = match runtime.pump_pty_output() {
        Ok(bytes) => bytes,
        Err(error) => return runtime_mouse_smoke_error(error),
    };
    let pressed = match runtime.send_mouse_input(MouseEvent::new(
        MouseEventKind::Press,
        MouseButton::Left,
        2,
        1,
    )) {
        Ok(pressed) => pressed,
        Err(error) => return runtime_mouse_smoke_error(error),
    };
    let released = match runtime.send_mouse_input(MouseEvent::new(
        MouseEventKind::Release,
        MouseButton::Left,
        2,
        1,
    )) {
        Ok(released) => released,
        Err(error) => return runtime_mouse_smoke_error(error),
    };
    let wheel = match runtime.send_mouse_input(MouseEvent::new(
        MouseEventKind::Press,
        MouseButton::WheelUp,
        0,
        0,
    )) {
        Ok(wheel) => wheel,
        Err(error) => return runtime_mouse_smoke_error(error),
    };
    let metrics = runtime.dump_runtime_perf_metrics();
    let input = runtime
        .shell_session()
        .map(|session| session.input.concat())
        .unwrap_or_default();
    let expected_input = [
        b"\x1b[<0;3;2M".as_slice(),
        b"\x1b[<0;3;2m".as_slice(),
        b"\x1b[<64;1;1M".as_slice(),
    ]
    .concat();

    if pumped_bytes != RUNTIME_MOUSE_SMOKE_ENABLE_REPORTING.len()
        || !pressed
        || !released
        || !wheel
        || input != expected_input
        || metrics.mouse_inputs != 3
        || metrics.pty_input_writes != 3
        || metrics.pty_input_bytes != expected_input.len() as u64
    {
        return CliExit {
            code: 1,
            stdout: String::new(),
            stderr: "runtime mouse smoke failed: mouse reports did not reach PTY writes\n"
                .to_owned(),
        };
    }

    CliExit {
        code: 0,
        stdout: format!(
            "runtime mouse smoke: ok\npumped bytes: {}\npress reported: {}\nrelease reported: {}\nwheel reported: {}\nmouse inputs: {}\npty input writes: {}\npty input bytes: {}\n",
            pumped_bytes,
            pressed,
            released,
            wheel,
            metrics.mouse_inputs,
            metrics.pty_input_writes,
            metrics.pty_input_bytes
        ),
        stderr: String::new(),
    }
}

fn runtime_mouse_smoke_error(error: impl std::fmt::Display) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime mouse smoke failed: {error}\n"),
    }
}

pub(super) fn runtime_response_smoke_exit() -> CliExit {
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
        Err(error) => return runtime_response_smoke_error(error),
    };
    let spawner =
        RuntimeInputCaptureSmokePtySpawner::new(RUNTIME_RESPONSE_SMOKE_QUERIES.as_bytes());
    if let Err(error) = runtime.start_shell(&spawner) {
        return runtime_response_smoke_error(error);
    }

    let pumped_bytes = match runtime.pump_pty_output() {
        Ok(bytes) => bytes,
        Err(error) => return runtime_response_smoke_error(error),
    };
    let metrics = runtime.dump_runtime_perf_metrics();
    let input = runtime
        .shell_session()
        .map(|session| session.input.concat())
        .unwrap_or_default();
    let expected_response = b"\x1b[3;5R\x1b[0n\x1b[?1;2c\x1b[>0;1;0c";

    if pumped_bytes != RUNTIME_RESPONSE_SMOKE_QUERIES.len()
        || input != expected_response
        || metrics.pty_response_writes != 1
        || metrics.pty_response_bytes != expected_response.len() as u64
        || metrics.pty_input_writes != 0
    {
        return CliExit {
            code: 1,
            stdout: String::new(),
            stderr: "runtime response smoke failed: terminal responses did not reach PTY writes\n"
                .to_owned(),
        };
    }

    CliExit {
        code: 0,
        stdout: format!(
            "runtime response smoke: ok\npumped bytes: {}\nresponse writes: {}\nresponse bytes: {}\npty input writes: {}\n",
            pumped_bytes,
            metrics.pty_response_writes,
            metrics.pty_response_bytes,
            metrics.pty_input_writes
        ),
        stderr: String::new(),
    }
}

fn runtime_response_smoke_error(error: impl std::fmt::Display) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime response smoke failed: {error}\n"),
    }
}

pub(super) fn runtime_idle_smoke_exit() -> CliExit {
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
