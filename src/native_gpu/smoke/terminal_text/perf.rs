use std::time::Instant;

use super::super::super::reports::{GpuTerminalTextPerfReport, GpuTerminalTextPerfRunner};
use super::super::{GpuBootstrapError, NativeGpuContext};
use super::build_terminal_text_smoke_draw;

const TERMINAL_TEXT_PERF_SMOKE_FRAMES: usize = 16;

impl GpuTerminalTextPerfRunner for NativeGpuContext {
    fn run_terminal_text_perf_smoke(
        &self,
    ) -> std::result::Result<GpuTerminalTextPerfReport, GpuBootstrapError> {
        let draw = build_terminal_text_smoke_draw()?;
        let mut durations = Vec::new();
        durations
            .try_reserve_exact(TERMINAL_TEXT_PERF_SMOKE_FRAMES)
            .map_err(|_| {
                GpuBootstrapError::SmokeReadback("perf sample allocation failed".to_owned())
            })?;
        let mut final_drawn_pixels = 0;

        for _ in 0..TERMINAL_TEXT_PERF_SMOKE_FRAMES {
            let started = Instant::now();
            let pixels = self.draw_terminal_text_smoke_frame(&draw)?;
            durations.push(started.elapsed().as_nanos());
            final_drawn_pixels = pixels.chunks_exact(4).filter(|pixel| pixel[3] != 0).count();
        }

        Ok(GpuTerminalTextPerfReport {
            frames: TERMINAL_TEXT_PERF_SMOKE_FRAMES,
            width: draw.target_width,
            height: draw.target_height,
            drawn_pixels: final_drawn_pixels,
            min_ns: *durations.iter().min().unwrap_or(&0),
            avg_ns: average_duration_ns(&durations),
            max_ns: *durations.iter().max().unwrap_or(&0),
            p95_ns: p95_duration_ns(&durations),
        })
    }
}

pub(super) fn average_duration_ns(durations: &[u128]) -> u128 {
    if durations.is_empty() {
        return 0;
    }
    durations.iter().sum::<u128>() / durations.len() as u128
}

pub(super) fn p95_duration_ns(durations: &[u128]) -> u128 {
    if durations.is_empty() {
        return 0;
    }
    let mut sorted = durations.to_vec();
    sorted.sort_unstable();
    let index = ((sorted.len() * 95).div_ceil(100)).saturating_sub(1);
    sorted[index]
}
