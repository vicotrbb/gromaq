# Benchmarks

Gromaq uses Criterion for deterministic CPU-side benchmark coverage. These
benchmarks are proof inputs for parser throughput, scrollback throughput, dirty
region coalescing, render planning, glyph quad generation, rasterized glyph
cache reuse, prepared surface glyph-frame construction, native input-to-render
plumbing, real-font rasterization, and PTY output pumping.

They do not prove the full performance acceptance target by themselves. Hardware
backed 144Hz frame pacing, p95 frame time, input latency, idle CPU, memory
growth, and live window runtime proof still require separate live measurements.

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
- long-session memory stability
- smooth interactive scrolling with a live window surface

Those remain acceptance criteria for later live measurement work.
