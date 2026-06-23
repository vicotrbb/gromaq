# Benchmarks

Benchmarks use Criterion.

Run:

```bash
cargo bench --bench parser_throughput -- --list
cargo bench
```

CI runs the benchmark list command so the harness keeps compiling without
running full Criterion measurements on every push.

Current benchmarks:

- `parser_large_output`: parses ANSI-styled output with Unicode content.
- `scrollback_large_output`: writes a prebuilt 2,000-line payload into a small viewport with bounded scrollback, avoiding fixture formatting work inside the measured loop.
- `dirty_region_coalescing`: marks overlapping dirty spans/cells/regions, checks containment, and drains the coalesced scheduler region.
- `render_plan_large_dirty_region`: builds CPU-side glyph draw commands from a large dirty viewport and exercises glyph atlas lookups.
- `glyph_quad_generation_large_plan`: converts a prebuilt large terminal render plan into textured glyph quads and triangle indices for GPU buffer upload.
- `rasterized_glyph_cache_hot_plan`: replays a pre-rasterized terminal render plan through the native glyph bitmap cache to measure the cached repeated-frame path.
- `prepared_surface_glyph_frame_large_plan`: builds an owned terminal glyph frame from a pre-rasterized large render plan, including atlas image packing and glyph quad generation.
- `native_input_echo_render_cycle`: sends a native key through the runtime PTY input path, echoes it through a deterministic PTY session, pumps output into terminal state, and renders dirty terminal state into a renderer plan.
- `font_rasterizer_combining_cell`: rasterizes a shaped combining-mark terminal cell from a real system monospace font into an RGBA8 glyph bitmap.
- `pty_runtime_pump_large_output`: drains queued PTY output through `NativeTerminalRuntime::pump_pty_output` into terminal state.
- `runtime_bounded_state_batches`: pumps four deterministic long-output batches through `NativeTerminalRuntime`, renders each dirty frame, and observes capped scrollback state.
- `runtime_alternate_screen_stages`: pumps primary, alternate-screen, and restore output stages through `NativeTerminalRuntime`, rendering each dirty stage and observing restored primary state.

Dirty-region tracking is unit-tested and benchmarked for coalescing/containment/drain behavior. Renderer benchmarks will be added with the concrete `wgpu` pipeline.
Frame-scheduler decisions are unit-tested with injected timestamps. `cargo run -- --frame-scheduler-smoke` on 2026-06-23 reported a 6,944,444 ns 144Hz target interval, a 4,944,444 ns frame-paced wait, 3 presented frames, and 2 dropped frames through the deterministic scheduler path; it does not yet prove hardware-backed 144Hz rendering.
PTY background reader, runtime pump behavior, and timed event-loop pump scheduling are integration-tested. PTY runtime pump throughput is benchmarked with deterministic queued output and feeds raw PTY bytes directly into the terminal parser. Native input echo-to-render latency is benchmarked with a deterministic PTY echo session and CPU-side renderer planning; real PTY throughput and live input-to-present latency are not yet benchmarked.
Native runtime render counters include sample count, total, max, and a bounded bucketed p95 estimate for rendered dirty frames. The runtime also tracks app-input-to-render latency with bounded sample, total, max, and p95 counters. `cargo run -- --runtime-perf-smoke` on 2026-06-23 pumped 1 byte through a deterministic PTY echo, rendered 1 CPU-side frame, and reported render p95 4,000,000 ns plus input-to-render p95 6,940,000 ns. `cargo run -- --runtime-idle-smoke` on 2026-06-23 pumped 0 bytes, reported 16 render attempts, 16 clean-frame skips, and 0 rendered frames through the deterministic CPU-side path. Those counters are structured inputs for future live-window proof, but they do not by themselves prove hardware-backed p95 frame time, input latency, or idle CPU.
Glyph-atlas cache behavior is unit-tested for identity, LRU eviction, and metrics; it does not yet prove rasterization speed or GPU upload performance.
Font rasterization, renderer-plan glyph bitmap population, and text-atlas GPU upload/readback are integration-tested with a real local font. Font-dependent benchmarks register their names and emit a clear skip message when no supported local monospace font is available, so a skipped run does not prove rasterization throughput on that machine. Direct shaped-cell font rasterization, cached render-plan glyph bitmap population, and prepared terminal glyph-frame construction are benchmarked for CPU-side paths, but GPU upload performance is not yet benchmarked and is not yet integrated into a terminal draw pipeline.
Render-plan generation is unit-tested against dirty-region and full-viewport modes and benchmarked for CPU-side command generation. Glyph-quad generation is integration-tested for pixel positions, wide-cell geometry, atlas UVs, and triangle indices, and benchmarked both directly and through prepared terminal glyph-frame construction; it does not yet prove GPU draw performance. The offscreen textured-quad and terminal-text smoke tests prove sampled draw pipelines and readback, but they are not benchmarked and do not yet prove windowed terminal frame time.

## Acceptance Targets

The full terminal goal is not complete until benchmarks and runtime validation prove:

- p95 frame time below 6.94 ms during normal interaction
- input latency p95 below 10 ms
- near-zero idle CPU
- smooth scrolling with large scrollback
- no unbounded memory growth
- efficient glyph cache hit rate
- no avoidable hot-path allocations

This benchmark harness does not yet prove those acceptance targets. It establishes reproducible parser, scrollback, dirty-region, render-plan, glyph-quad, prepared-frame, input echo-to-render, font-rasterization, cached glyph-bitmap, runtime PTY pump, bounded runtime state, and runtime alternate-screen measurements for future regression tracking. `cargo run -- --runtime-bounded-state-smoke` is a deterministic long-session state smoke for capped runtime scrollback, but it is not a live process-memory growth measurement.
