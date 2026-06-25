use std::process::Command;
use std::thread::sleep;
use std::time::Duration;

use crate::cli::CliExit;

use super::run_runtime_idle_probe;

const RUNTIME_IDLE_CPU_SMOKE_SAMPLES: usize = 5;
const RUNTIME_IDLE_CPU_SMOKE_SAMPLE_INTERVAL_MS: u64 = 50;
const RUNTIME_IDLE_CPU_BUDGET_PERCENT: f32 = 5.0;

pub(in crate::cli) fn runtime_idle_cpu_smoke_exit() -> CliExit {
    let probe = match run_runtime_idle_probe() {
        Ok(probe) => probe,
        Err(error) => return runtime_idle_cpu_smoke_error(error),
    };
    let mut max_cpu_percent = 0.0_f32;
    for _ in 0..RUNTIME_IDLE_CPU_SMOKE_SAMPLES {
        sleep(Duration::from_millis(
            RUNTIME_IDLE_CPU_SMOKE_SAMPLE_INTERVAL_MS,
        ));
        let cpu_percent = match current_process_cpu_percent() {
            Ok(cpu_percent) => cpu_percent,
            Err(error) => return runtime_idle_cpu_smoke_error(error),
        };
        max_cpu_percent = max_cpu_percent.max(cpu_percent);
    }

    if let Some(failure) = runtime_idle_cpu_budget_failure(max_cpu_percent) {
        return CliExit {
            code: 1,
            stdout: String::new(),
            stderr: format!("runtime idle cpu smoke failed: {failure}\n"),
        };
    }

    CliExit {
        code: 0,
        stdout: format!(
            "runtime idle cpu smoke: ok\nsamples: {}\nsample interval ms: {}\nmax cpu percent: {:.1}\ncpu budget percent: {:.1}\nrender attempts: {}\nclean frame skips: {}\nrendered frames: {}\n",
            RUNTIME_IDLE_CPU_SMOKE_SAMPLES,
            RUNTIME_IDLE_CPU_SMOKE_SAMPLE_INTERVAL_MS,
            max_cpu_percent,
            RUNTIME_IDLE_CPU_BUDGET_PERCENT,
            probe.metrics.render_attempts,
            probe.metrics.clean_frame_skips,
            probe.metrics.rendered_frames
        ),
        stderr: String::new(),
    }
}

fn current_process_cpu_percent() -> Result<f32, String> {
    let pid = std::process::id().to_string();
    let output = Command::new("ps")
        .args(["-o", "%cpu=", "-p", &pid])
        .output()
        .map_err(|error| format!("process cpu sampling failed to start: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "process cpu sampling failed with status {}",
            output.status
        ));
    }
    let stdout = String::from_utf8(output.stdout)
        .map_err(|error| format!("process cpu output was not utf-8: {error}"))?;
    stdout
        .split_whitespace()
        .next()
        .ok_or_else(|| "process cpu output was empty".to_owned())?
        .parse::<f32>()
        .map_err(|error| format!("process cpu output was not numeric: {error}"))
}

fn runtime_idle_cpu_budget_failure(max_cpu_percent: f32) -> Option<&'static str> {
    if max_cpu_percent > RUNTIME_IDLE_CPU_BUDGET_PERCENT {
        return Some("idle cpu percent exceeded budget");
    }
    None
}

fn runtime_idle_cpu_smoke_error(error: impl std::fmt::Display) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime idle cpu smoke failed: {error}\n"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_idle_cpu_budget_accepts_cpu_at_limit() {
        assert_eq!(
            runtime_idle_cpu_budget_failure(RUNTIME_IDLE_CPU_BUDGET_PERCENT),
            None
        );
    }

    #[test]
    fn runtime_idle_cpu_budget_rejects_cpu_over_limit() {
        assert_eq!(
            runtime_idle_cpu_budget_failure(RUNTIME_IDLE_CPU_BUDGET_PERCENT + 0.1),
            Some("idle cpu percent exceeded budget")
        );
    }
}
