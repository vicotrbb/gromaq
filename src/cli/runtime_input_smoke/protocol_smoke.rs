use crate::app::{NativeTerminalRuntime, NativeTerminalRuntimeConfig};
use crate::cli::CliExit;
use crate::mouse::{MouseButton, MouseEvent, MouseEventKind};
use crate::pty::ShellCommand;

use super::pty_smoke::{RuntimeInputCaptureSmokePtySession, RuntimeInputCaptureSmokePtySpawner};

const RUNTIME_FOCUS_SMOKE_ENABLE_REPORTING: &str = "\x1b[?1004h";
const RUNTIME_MOUSE_SMOKE_ENABLE_REPORTING: &str = "\x1b[?1000h\x1b[?1006h";
const RUNTIME_RESPONSE_SMOKE_QUERIES: &str = "\x1b[3;5H\x1b[6n\x1b[5n\x1b[c\x1b[>c";

pub(in crate::cli) fn runtime_focus_smoke_exit() -> CliExit {
    let mut runtime = match runtime_protocol_smoke_runtime() {
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
    let input = captured_shell_input(&runtime);

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

pub(in crate::cli) fn runtime_mouse_smoke_exit() -> CliExit {
    let mut runtime = match runtime_protocol_smoke_runtime() {
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
    let input = captured_shell_input(&runtime);
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

pub(in crate::cli) fn runtime_response_smoke_exit() -> CliExit {
    let mut runtime = match runtime_protocol_smoke_runtime() {
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
    let input = captured_shell_input(&runtime);
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

fn runtime_protocol_smoke_runtime()
-> Result<NativeTerminalRuntime<RuntimeInputCaptureSmokePtySession>, String> {
    NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
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
    .map_err(|error| error.to_string())
}

fn captured_shell_input(
    runtime: &NativeTerminalRuntime<RuntimeInputCaptureSmokePtySession>,
) -> Vec<u8> {
    runtime
        .shell_session()
        .map(|session| session.input.concat())
        .unwrap_or_default()
}
