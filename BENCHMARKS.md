# Benchmarks

Benchmarks use Criterion.

Run:

```bash
cargo bench
```

Current benchmarks:

- `parser_large_output`: parses ANSI-styled output with Unicode content.
- `scrollback_large_output`: writes a prebuilt 2,000-line payload into a small viewport with bounded scrollback, avoiding fixture formatting work inside the measured loop.
- `dirty_region_coalescing`: marks overlapping dirty spans/cells/regions, checks containment, and drains the coalesced scheduler region.
- `render_plan_large_dirty_region`: builds CPU-side glyph draw commands from a large dirty viewport and exercises glyph atlas lookups.
- `glyph_quad_generation_large_plan`: converts a prebuilt large terminal render plan into textured glyph quads and triangle indices for GPU buffer upload.
- `rasterized_glyph_cache_hot_plan`: replays a pre-rasterized terminal render plan through the native glyph bitmap cache to measure the cached repeated-frame path.
- `pty_runtime_pump_large_output`: drains queued PTY output through `NativeTerminalRuntime::pump_pty_output` into terminal state.

Dirty-region tracking is unit-tested and benchmarked for coalescing/containment/drain behavior. Renderer benchmarks will be added with the concrete `wgpu` pipeline.
Frame-scheduler decisions are unit-tested with injected timestamps; they do not yet prove hardware-backed 144Hz rendering.
PTY background reader, runtime pump behavior, and timed event-loop pump scheduling are integration-tested. PTY runtime pump throughput is benchmarked with deterministic queued output and feeds raw PTY bytes directly into the terminal parser; real PTY throughput and input-to-render latency are not yet benchmarked.
Glyph-atlas cache behavior is unit-tested for identity, LRU eviction, and metrics; it does not yet prove rasterization speed or GPU upload performance.
Font rasterization, renderer-plan glyph bitmap population, and text-atlas GPU upload/readback are integration-tested with a real local font; cached render-plan glyph bitmap population is benchmarked for the repeated-frame path, but cold font rasterization and GPU upload performance are not yet benchmarked and are not yet integrated into a terminal draw pipeline.
Render-plan generation is unit-tested against dirty-region and full-viewport modes and benchmarked for CPU-side command generation. Glyph-quad generation is integration-tested for pixel positions, wide-cell geometry, atlas UVs, and triangle indices, and benchmarked for CPU-side conversion from planned glyphs to vertices/indices; it does not yet prove GPU draw performance. The offscreen textured-quad and terminal-text smoke tests prove sampled draw pipelines and readback, but they are not benchmarked and do not yet prove windowed terminal frame time.

## Acceptance Targets

The full terminal goal is not complete until benchmarks and runtime validation prove:

- p95 frame time below 6.94 ms during normal interaction
- input latency p95 below 10 ms
- near-zero idle CPU
- smooth scrolling with large scrollback
- no unbounded memory growth
- efficient glyph cache hit rate
- no avoidable hot-path allocations

This benchmark harness does not yet prove those acceptance targets. It establishes reproducible parser, scrollback, dirty-region, render-plan, glyph-quad, cached glyph-bitmap, and runtime PTY pump measurements for future regression tracking.
