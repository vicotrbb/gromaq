# Benchmarks

Gromaq uses Criterion for deterministic CPU-side benchmark coverage. These
benchmarks are proof inputs for parser throughput, scrollback throughput, dirty
region coalescing, render planning, glyph quad generation, rasterized glyph
cache reuse, prepared surface glyph-frame construction, native input-to-render
plumbing, real-font rasterization, PTY output pumping, bounded runtime state
under repeated output batches, and runtime alternate-screen transitions.

They do not prove the full performance acceptance target by themselves. Hardware
backed 144Hz frame pacing, p95 frame time, input latency, idle CPU, memory
growth, and live window runtime proof still require separate live measurements.
The native runtime exposes bounded render-time and app-input-to-render latency
counters, including sample count, total, average, max, and bucketed p95
estimates, so live-window measurements can be reported from structured counters
instead of subjective observation.
On 2026-06-23, `cargo run -- --runtime-perf-smoke` pumped 1 deterministic PTY
echo byte, rendered 1 CPU-side frame, and reported render average/p95 plus
input-to-render average/p95 counters. On the same date,
`cargo run -- --runtime-large-output-smoke` pumped 12288 bytes from 512 lines,
reported 128 retained scrollback lines, rendered 1 CPU-side dirty frame,
verified `gromaq-runtime-line-511` in the render plan, and reported render p95
500000 ns. On the same date, `cargo run -- --runtime-bounded-state-smoke`
pumped 51200 bytes from 2048 lines across 4 batches, retained 128 scrollback
lines and 128 styled scrollback cell rows, kept retained cell data within the
4096-cell deterministic cap for a 32-column bounded runtime, rendered 4
CPU-side dirty frames, and verified
`gromaq-bounded-line-2047` in the render plan. On the same date,
`cargo run -- --runtime-continuous-output-smoke` pumped 6912 bytes from 256
lines across 32 small PTY batches, rendered each dirty batch, verified the
configured 64-line scrollback cap, reported render p95 500000 ns, and checked
that `gromaq-continuous-line-255` reached the latest visible render plan. On
the same date,
`cargo run -- --runtime-alternate-screen-smoke` pumped primary, alternate, and
restore output stages through the runtime, rendered 3 CPU-side dirty frames,
restored primary visible content, and suppressed alternate-screen scrollback.
On the same date, `cargo run -- --runtime-reflow-smoke` pumped 80
bytes, reported 1 resize event, preserved 2 reflowed scrollback lines with
styled metadata, rendered visible lines `klmno|pqrst`, and rendered 1 CPU-side
dirty frame. On the same date, `cargo run -- --runtime-idle-smoke` pumped 0
bytes, reported 16 render attempts, 16 clean-frame skips, and 0 rendered frames
through the deterministic CPU-side path. On the same date,
`cargo run -- --frame-scheduler-smoke` reported a 6944444 ns 144Hz target
interval, 4944444 ns frame-paced wait, 3 presented frames, and 2 dropped
frames. These are deterministic smoke results, not live hardware acceptance
measurements.

## Reproducible Local Run

Run from the repository root:

```bash
cargo fmt --check
git diff --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
cargo test --all -- --list | rg "^[a-zA-Z0-9_].*: test$" | wc -l
cargo bench --bench parser_throughput -- --list
cargo bench
```

The benchmark list should include:

- `parser_large_output`
- `scrollback_large_output`
- `dirty_region_coalescing`
- `render_plan_large_dirty_region`
- `glyph_quad_generation_large_plan`
- `rasterized_glyph_cache_hot_plan`
- `prepared_surface_glyph_frame_large_plan`
- `native_input_echo_render_cycle`
- `font_rasterizer_combining_cell`
- `pty_runtime_pump_large_output`
- `runtime_bounded_state_batches`
- `runtime_continuous_output_batches`
- `runtime_alternate_screen_stages`

Some benchmarks load a local monospace font for real glyph rasterization. The
benchmark harness checks common macOS and Linux font paths. If no candidate is
available, the font-dependent benchmark name is still registered and emits a
clear skip message instead of panicking; that skip does not prove rasterization
throughput on the current machine.

## Regression Handling

Criterion compares the current run against the local baseline under
`target/criterion`. Treat any `Performance has regressed` line as a finding, not
as noise to hide.

Required local handling:

1. Record the benchmark name and percentage range from the first run.
2. Rerun `cargo bench` once.
3. If the same benchmark still reports a regression, investigate before
   treating the slice as performance-clean.
4. If the rerun clears the original regression but reports a different
   benchmark, report both runs and do not run an unbounded loop to chase
   benchmark variance.

## Current Proof Boundary

These benchmarks prove reproducible local CPU-side measurement coverage for the
foundation components named above. They do not prove:

- sustained 144Hz live rendering
- p95 frame time below 6.94ms
- p95 input latency below 10ms
- near-zero idle CPU
- live process memory stability during long sessions
- smooth interactive scrolling with a live window surface

Those remain acceptance criteria for later live measurement work.
