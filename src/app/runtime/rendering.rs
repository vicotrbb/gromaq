use crate::error::Result as GromaqResult;
use crate::renderer::GpuRenderer;

use super::NativeTerminalRuntime;
use crate::app::perf::{
    NativeRuntimePerfSnapshot, NativeRuntimeStateSnapshot, add_usize_counter,
    average_duration_nanos, dirty_region_cell_count, saturating_duration_nanos,
    scrollback_cell_count,
};

impl<S> NativeTerminalRuntime<S> {
    /// Return deterministic native runtime counters.
    pub fn dump_runtime_perf_metrics(&self) -> NativeRuntimePerfSnapshot {
        self.perf
    }

    /// Return deterministic runtime state-footprint counters.
    pub fn dump_runtime_state_snapshot(&self) -> NativeRuntimeStateSnapshot {
        let scrollback = self.terminal.dump_scrollback();
        NativeRuntimeStateSnapshot {
            terminal_cols: self.config.terminal_cols,
            terminal_rows: self.config.terminal_rows,
            visible_cells: usize::from(self.config.terminal_cols)
                .saturating_mul(usize::from(self.config.terminal_rows)),
            scrollback_limit: self.config.scrollback_lines,
            scrollback_lines: scrollback.lines.len(),
            scrollback_cell_rows: scrollback.cells.len(),
            scrollback_cells: scrollback_cell_count(&scrollback),
            scrollback_cell_limit: self
                .config
                .scrollback_lines
                .saturating_mul(usize::from(self.config.terminal_cols)),
        }
    }

    /// Render the current terminal frame when dirty regions are pending.
    pub fn render_terminal_frame<R>(&mut self, renderer: &mut R) -> GromaqResult<bool>
    where
        R: GpuRenderer,
    {
        self.perf.render_attempts += 1;
        let dirty_regions = self.terminal.take_dirty_regions();
        if dirty_regions.is_empty() {
            self.perf.clean_frame_skips += 1;
            tracing::trace!(
                render_attempts = self.perf.render_attempts,
                clean_frame_skips = self.perf.clean_frame_skips,
                "skipped clean native terminal frame"
            );
            return Ok(false);
        }
        let render_started = std::time::Instant::now();
        if let Err(error) = renderer.render_frame(
            &self.terminal.dump_grid(),
            self.terminal.dump_cursor(),
            &dirty_regions,
        ) {
            self.terminal.invalidate_viewport();
            return Err(error);
        }
        let elapsed_ns = saturating_duration_nanos(render_started.elapsed());
        let dirty_cells = dirty_region_cell_count(&dirty_regions);
        self.perf.rendered_frames += 1;
        add_usize_counter(&mut self.perf.rendered_dirty_regions, dirty_regions.len());
        self.perf.rendered_dirty_cells = self.perf.rendered_dirty_cells.saturating_add(dirty_cells);
        self.perf.rendered_dirty_cells_max = self.perf.rendered_dirty_cells_max.max(dirty_cells);
        self.perf.render_time_samples += 1;
        self.perf.render_time_total_ns = self.perf.render_time_total_ns.saturating_add(elapsed_ns);
        self.perf.render_time_avg_ns = average_duration_nanos(
            self.perf.render_time_total_ns,
            self.perf.render_time_samples,
        );
        self.perf.render_time_max_ns = self.perf.render_time_max_ns.max(elapsed_ns);
        self.render_time_histogram.record(elapsed_ns);
        self.perf.render_time_p95_ns = self
            .render_time_histogram
            .p95_upper_bound_ns(self.perf.render_time_samples);
        tracing::trace!(
            dirty_regions = dirty_regions.len(),
            dirty_cells,
            render_time_ns = elapsed_ns,
            rendered_frames = self.perf.rendered_frames,
            render_time_p95_ns = self.perf.render_time_p95_ns,
            "rendered native terminal frame"
        );
        if let Some(input_started) = self.pending_input_to_render_started.take() {
            self.record_input_to_render_latency(saturating_duration_nanos(input_started.elapsed()));
        }
        Ok(true)
    }

    /// Force the next renderer pass to cover the visible terminal viewport.
    pub fn invalidate_terminal_frame(&mut self) {
        self.terminal.invalidate_viewport();
    }

    fn record_input_to_render_latency(&mut self, elapsed_ns: u64) {
        self.perf.input_to_render_samples += 1;
        self.perf.input_to_render_total_ns = self
            .perf
            .input_to_render_total_ns
            .saturating_add(elapsed_ns);
        self.perf.input_to_render_avg_ns = average_duration_nanos(
            self.perf.input_to_render_total_ns,
            self.perf.input_to_render_samples,
        );
        self.perf.input_to_render_max_ns = self.perf.input_to_render_max_ns.max(elapsed_ns);
        self.input_to_render_histogram.record(elapsed_ns);
        self.perf.input_to_render_p95_ns = self
            .input_to_render_histogram
            .p95_upper_bound_ns(self.perf.input_to_render_samples);
    }
}
