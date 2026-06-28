use crate::cli::CliExit;
use crate::mouse::{MouseButton, MouseEvent, MouseEventKind};

use super::{
    RuntimeInputCaptureSmokePtySpawner, captured_shell_input, runtime_protocol_smoke_runtime,
};

const RUNTIME_MOUSE_SMOKE_ENABLE_REPORTING: &str = "\x1b[?1000h\x1b[?1002h\x1b[?1003h\x1b[?1006h";
const RUNTIME_MOUSE_SMOKE_ENABLE_DEFAULT: &str = "\x1b[?1000h";
const RUNTIME_MOUSE_SMOKE_ENABLE_X10: &str = "\x1b[?9h";

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
    let dragged = match runtime.send_mouse_input(MouseEvent::new(
        MouseEventKind::Drag,
        MouseButton::Left,
        4,
        2,
    )) {
        Ok(dragged) => dragged,
        Err(error) => return runtime_mouse_smoke_error(error),
    };
    let motion = match runtime.send_mouse_input(MouseEvent::new(
        MouseEventKind::Motion,
        MouseButton::None,
        6,
        1,
    )) {
        Ok(motion) => motion,
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
    let mut default_runtime = match runtime_protocol_smoke_runtime() {
        Ok(runtime) => runtime,
        Err(error) => return runtime_mouse_smoke_error(error),
    };
    let default_spawner =
        RuntimeInputCaptureSmokePtySpawner::new(RUNTIME_MOUSE_SMOKE_ENABLE_DEFAULT.as_bytes());
    if let Err(error) = default_runtime.start_shell(&default_spawner) {
        return runtime_mouse_smoke_error(error);
    }
    if let Err(error) = default_runtime.pump_pty_output() {
        return runtime_mouse_smoke_error(error);
    }
    let default_pressed = match default_runtime.send_mouse_input(MouseEvent::new(
        MouseEventKind::Press,
        MouseButton::Left,
        2,
        1,
    )) {
        Ok(pressed) => pressed,
        Err(error) => return runtime_mouse_smoke_error(error),
    };
    let default_released = match default_runtime.send_mouse_input(MouseEvent::new(
        MouseEventKind::Release,
        MouseButton::Left,
        2,
        1,
    )) {
        Ok(released) => released,
        Err(error) => return runtime_mouse_smoke_error(error),
    };
    let mut x10_runtime = match runtime_protocol_smoke_runtime() {
        Ok(runtime) => runtime,
        Err(error) => return runtime_mouse_smoke_error(error),
    };
    let x10_spawner =
        RuntimeInputCaptureSmokePtySpawner::new(RUNTIME_MOUSE_SMOKE_ENABLE_X10.as_bytes());
    if let Err(error) = x10_runtime.start_shell(&x10_spawner) {
        return runtime_mouse_smoke_error(error);
    }
    if let Err(error) = x10_runtime.pump_pty_output() {
        return runtime_mouse_smoke_error(error);
    }
    let x10_pressed = match x10_runtime.send_mouse_input(MouseEvent::new(
        MouseEventKind::Press,
        MouseButton::Left,
        2,
        1,
    )) {
        Ok(pressed) => pressed,
        Err(error) => return runtime_mouse_smoke_error(error),
    };
    let x10_released = match x10_runtime.send_mouse_input(MouseEvent::new(
        MouseEventKind::Release,
        MouseButton::Left,
        2,
        1,
    )) {
        Ok(released) => released,
        Err(error) => return runtime_mouse_smoke_error(error),
    };
    let metrics = runtime.dump_runtime_perf_metrics();
    let default_metrics = default_runtime.dump_runtime_perf_metrics();
    let x10_metrics = x10_runtime.dump_runtime_perf_metrics();
    let input = [
        captured_shell_input(&runtime),
        captured_shell_input(&default_runtime),
        captured_shell_input(&x10_runtime),
    ]
    .concat();
    let expected_input = [
        b"\x1b[<0;3;2M".as_slice(),
        b"\x1b[<0;3;2m".as_slice(),
        b"\x1b[<32;5;3M".as_slice(),
        b"\x1b[<35;7;2M".as_slice(),
        b"\x1b[<64;1;1M".as_slice(),
        b"\x1b[M #\"".as_slice(),
        b"\x1b[M##\"".as_slice(),
        b"\x1b[M #\"".as_slice(),
    ]
    .concat();

    if pumped_bytes != RUNTIME_MOUSE_SMOKE_ENABLE_REPORTING.len()
        || !pressed
        || !released
        || !dragged
        || !motion
        || !wheel
        || !default_pressed
        || !default_released
        || !x10_pressed
        || x10_released
        || input != expected_input
        || metrics.mouse_inputs + default_metrics.mouse_inputs + x10_metrics.mouse_inputs != 8
        || metrics.pty_input_writes
            + default_metrics.pty_input_writes
            + x10_metrics.pty_input_writes
            != 8
        || metrics.pty_input_bytes + default_metrics.pty_input_bytes + x10_metrics.pty_input_bytes
            != expected_input.len() as u64
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
            "runtime mouse smoke: ok\npumped bytes: {}\npress reported: {}\nrelease reported: {}\ndrag reported: {}\nmotion reported: {}\nwheel reported: {}\ndefault press reported: {}\ndefault release reported: {}\nx10 press reported: {}\nx10 release suppressed: {}\nmouse inputs: {}\npty input writes: {}\npty input bytes: {}\n",
            pumped_bytes,
            pressed,
            released,
            dragged,
            motion,
            wheel,
            default_pressed,
            default_released,
            x10_pressed,
            !x10_released,
            metrics.mouse_inputs + default_metrics.mouse_inputs + x10_metrics.mouse_inputs,
            metrics.pty_input_writes
                + default_metrics.pty_input_writes
                + x10_metrics.pty_input_writes,
            metrics.pty_input_bytes + default_metrics.pty_input_bytes + x10_metrics.pty_input_bytes
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
