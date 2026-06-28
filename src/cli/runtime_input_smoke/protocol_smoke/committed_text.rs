use crate::cli::CliExit;

use super::super::pty_smoke::RuntimeInputCaptureSmokePtySpawner;
use super::{captured_shell_input, runtime_protocol_smoke_runtime};

const RUNTIME_COMMITTED_TEXT_SMOKE_TEXT: &str = "a\u{0301}界🙂";

pub(in crate::cli) fn runtime_committed_text_smoke_exit() -> CliExit {
    let mut runtime = match runtime_protocol_smoke_runtime() {
        Ok(runtime) => runtime,
        Err(error) => return runtime_committed_text_smoke_error(error),
    };
    let spawner = RuntimeInputCaptureSmokePtySpawner::new(b"");
    if let Err(error) = runtime.start_shell(&spawner) {
        return runtime_committed_text_smoke_error(error);
    }
    if let Err(error) = runtime.send_committed_text(RUNTIME_COMMITTED_TEXT_SMOKE_TEXT) {
        return runtime_committed_text_smoke_error(error);
    }

    let metrics = runtime.dump_runtime_perf_metrics();
    let input = captured_shell_input(&runtime);
    let committed_bytes = RUNTIME_COMMITTED_TEXT_SMOKE_TEXT.len();

    if input != RUNTIME_COMMITTED_TEXT_SMOKE_TEXT.as_bytes()
        || metrics.committed_text_bytes != committed_bytes as u64
        || metrics.pty_input_writes != 1
        || metrics.pty_input_bytes != committed_bytes as u64
        || metrics.native_key_inputs != 0
        || metrics.paste_bytes != 0
    {
        return CliExit {
            code: 1,
            stdout: String::new(),
            stderr:
                "runtime committed text smoke failed: committed text did not reach PTY writes\n"
                    .to_owned(),
        };
    }

    CliExit {
        code: 0,
        stdout: format!(
            "runtime committed text smoke: ok\ncommitted bytes: {}\npty input writes: {}\npty input bytes: {}\nnative key inputs: {}\npaste bytes: {}\n",
            metrics.committed_text_bytes,
            metrics.pty_input_writes,
            metrics.pty_input_bytes,
            metrics.native_key_inputs,
            metrics.paste_bytes
        ),
        stderr: String::new(),
    }
}

fn runtime_committed_text_smoke_error(error: impl std::fmt::Display) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime committed text smoke failed: {error}\n"),
    }
}
