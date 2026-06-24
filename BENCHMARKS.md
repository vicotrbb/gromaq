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
- `unicode_emoji_cluster_output`: parses mixed emoji ZWJ, variation-selector, regional-indicator, tag-flag, and combining-mark output into bounded terminal state.
- `scrollback_large_output`: writes a prebuilt 2,000-line payload into a small viewport with bounded scrollback, avoiding fixture formatting work inside the measured loop.
- `scrollback_view_navigation`: repeatedly moves a populated scrollback viewport up and down through retained history, snapshots the displayed grid, and drains dirty viewport regions.
- `dirty_region_coalescing`: marks overlapping dirty spans/cells/regions, checks containment, and drains the coalesced scheduler region.
- `glyph_atlas_cache_churn`: measures hot glyph-atlas lookups plus bounded LRU churn, miss, eviction, and generation reuse behavior.
- `frame_scheduler_144hz_timeline`: exercises deterministic 144Hz frame scheduling decisions across paced waits, dirty renders, late frames, idle suppression, and dropped-frame metrics.
- `render_plan_large_dirty_region`: builds CPU-side glyph draw commands from a large dirty viewport and exercises glyph atlas lookups.
- `glyph_quad_generation_large_plan`: converts a prebuilt large terminal render plan into textured glyph quads and triangle indices for GPU buffer upload.
- `rasterized_glyph_cache_hot_plan`: replays a pre-rasterized terminal render plan through the native glyph bitmap cache to measure the cached repeated-frame path.
- `prepared_surface_glyph_frame_large_plan`: builds an owned terminal glyph frame from a pre-rasterized large render plan, including atlas image packing and glyph quad generation.
- `native_input_echo_render_cycle`: sends a native key through the runtime PTY input path, echoes it through a deterministic PTY session, pumps output into terminal state, and renders dirty terminal state into a renderer plan.
- `font_rasterizer_combining_cell`: rasterizes a shaped combining-mark terminal cell from a real system monospace font into an RGBA8 glyph bitmap.
- `pty_runtime_pump_large_output`: drains queued PTY output through `NativeTerminalRuntime::pump_pty_output` into terminal state.
- `real_pty_shell_large_output_burst`: drains a bounded real `/bin/sh` large-output burst through the native PTY background reader when `/bin/sh` is available.
- `real_pty_shell_input_echo_roundtrip`: writes a command to a real interactive `/bin/sh` PTY and measures output readback through the native background reader when `/bin/sh` is available.
- `runtime_bounded_state_batches`: pumps four deterministic long-output batches through `NativeTerminalRuntime`, renders each dirty frame, and observes capped scrollback state.
- `runtime_state_snapshot_bounded_session`: repeatedly samples `NativeTerminalRuntime::dump_runtime_state_snapshot` after a deterministic bounded long-output session has populated capped scrollback.
- `runtime_continuous_output_batches`: pumps 32 small deterministic PTY batches through `NativeTerminalRuntime`, renders each dirty frame, and observes capped scrollback state.
- `runtime_alternate_screen_stages`: pumps primary, alternate-screen, and restore output stages through `NativeTerminalRuntime`, rendering each dirty stage and observing restored primary state.
- `runtime_protocol_input_reports`: pumps deterministic focus/mouse enablement plus terminal status/capability queries, then measures focus reports, SGR mouse press/release/wheel reports, and terminal response writeback through `NativeTerminalRuntime`.

Dirty-region tracking is unit-tested and benchmarked for coalescing/containment/drain behavior. Renderer benchmarks cover CPU-side render planning, glyph-quad generation, prepared surface glyph-frame construction, and offscreen GPU upload/draw/readback paths.
Frame-scheduler decisions are unit-tested with injected timestamps. `cargo run -- --frame-scheduler-smoke` on 2026-06-23 reported a 6,944,444 ns 144Hz target interval, a 4,944,444 ns frame-paced wait, 3 presented frames, and 2 dropped frames through the deterministic scheduler path; it does not yet prove hardware-backed 144Hz rendering.
PTY background reader, runtime pump behavior, and timed event-loop pump scheduling are integration-tested. PTY runtime pump throughput is benchmarked with deterministic queued output and feeds raw PTY bytes directly into the terminal parser. `real_pty_shell_large_output_burst` benchmarks a real `/bin/sh` PTY session producing a bounded large-output burst through the native background reader when `/bin/sh` is available. `real_pty_shell_input_echo_roundtrip` benchmarks writing a command to a real interactive `/bin/sh` PTY and reading the resulting output through the background reader. Native input echo-to-render latency is benchmarked with a deterministic PTY echo session and CPU-side renderer planning; live input-to-present latency is not yet benchmarked.
Native runtime render counters include rendered dirty regions/cells, sample count, total, average, max, and a bounded bucketed p95 estimate for rendered dirty frames. The runtime also tracks app-input-to-render latency with bounded sample, total, average, max, and p95 counters. `cargo run -- --runtime-perf-smoke` on 2026-06-23 pumped 1 byte through a deterministic PTY echo, rendered 1 CPU-side frame, and reported rendered dirty-region work plus render and input-to-render sample, average, max, and p95 counters. Large, bounded-state, continuous-output, and local scrollback runtime smokes validate nonzero rendered dirty-region/cell work and cap each rendered dirty-cell batch to the visible viewport. `cargo run -- --runtime-idle-smoke` on 2026-06-23 pumped 0 bytes, reported 16 render attempts, 16 clean-frame skips, and 0 rendered frames through the deterministic CPU-side path. Those counters are structured inputs for future live-window proof, but they do not by themselves prove hardware-backed p95 frame time, input latency, or idle CPU.
Glyph-atlas cache behavior is unit-tested for identity, LRU eviction, and metrics, and `glyph_atlas_cache_churn` measures hot lookup plus bounded LRU churn behavior. This does not by itself prove rasterization speed, and live GPU upload/readback throughput is measured separately by the GPU upload benchmarks.
Font rasterization, renderer-plan glyph bitmap population, texture upload/readback, glyph-atlas upload/readback, and text-atlas GPU upload/readback are integration-tested with deterministic fixtures or a real local font. Font-dependent benchmarks register their names and emit a clear skip message when no supported local monospace font is available, so a skipped run does not prove rasterization throughput on that machine. Direct shaped-cell font rasterization, cached render-plan glyph bitmap population, and prepared terminal glyph-frame construction are benchmarked for CPU-side paths, while `gpu_texture_upload_readback`, `gpu_glyph_atlas_upload_readback`, and `gpu_text_atlas_upload_readback` measure live GPU upload/readback paths when a compatible adapter is available; those still do not prove full windowed terminal frame time.
Render-plan generation is unit-tested against dirty-region and full-viewport modes and benchmarked for CPU-side command generation. Glyph-quad generation is integration-tested for pixel positions, wide-cell geometry, atlas UVs, and triangle indices, and benchmarked both directly and through prepared terminal glyph-frame construction. The offscreen textured-quad and terminal-text smoke tests prove sampled draw pipelines and readback, and the Criterion harness includes `gpu_textured_quad_draw_readback` plus `gpu_terminal_text_draw_readback` for live offscreen GPU draw/readback measurement when a compatible adapter is available; they still do not prove windowed terminal frame time.

## Current Full Local Run

On 2026-06-24, `cargo bench` completed on macOS Darwin 23.5.0 arm64 with an
Apple M1 Pro and reported no Criterion regression lines. The measured ranges are
recorded in [`documentation/benchmarks.md`](documentation/benchmarks.md).

Key evidence from that run:

- `native_input_echo_render_cycle`: 111.12-113.73 us
- `runtime_bounded_state_batches`: 6.0650-6.1365 ms
- `runtime_continuous_output_batches`: 1.6107-1.7109 ms
- `gpu_textured_quad_draw_readback`: 1.5650-1.5952 ms
- `gpu_terminal_text_draw_readback`: 32.000-32.464 ms
- `gpu_text_atlas_upload_readback`: 31.170-31.839 ms
- `scrollback_view_navigation`: 4.6589-4.7105 s before hot-path fix; 19.365-20.240 ms after

`scrollback_view_navigation` was materially slower than the other CPU-side
foundation benchmarks before the scrollback-view hot-path fix, and Criterion
estimated 461.1 seconds for its 100-sample collection. After removing full
scrollback snapshot cloning from visible history-grid dumps, targeted reruns
measured 19.365-20.240 ms; Criterion reported a 99.578%-99.587% improvement
for the first post-fix rerun and then reported the second post-fix rerun as
within the noise threshold. This is not a substitute for live-window
smooth-scrolling acceptance proof.

## Acceptance Targets

The full terminal goal is not complete until benchmarks and runtime validation prove:

- p95 frame time below 6.94 ms during normal interaction
- input latency p95 below 10 ms
- near-zero idle CPU
- smooth scrolling with large scrollback
- no unbounded memory growth
- efficient glyph cache hit rate
- no avoidable hot-path allocations

This benchmark harness does not yet prove those acceptance targets. It establishes reproducible parser, Unicode cluster ingestion, scrollback ingestion, scrollback viewport navigation, dirty-region, frame-scheduler, render-plan, glyph-quad, prepared-frame, input echo-to-render, font-rasterization, cached glyph-bitmap, deterministic runtime PTY pump, real PTY shell burst, real PTY input/output roundtrip, bounded runtime state, continuous runtime output, runtime alternate-screen, and runtime protocol input/report measurements for future regression tracking. `NativeTerminalRuntime::dump_runtime_state_snapshot` exposes deterministic visible-cell and retained scrollback row/cell/cap counters, and `cargo run -- --runtime-bounded-state-smoke` uses that state snapshot for capped runtime scrollback proof, but it is not a live process-memory growth measurement.
