# Benchmarks

Gromaq uses Criterion for deterministic CPU-side benchmark coverage. These
benchmarks are proof inputs for parser throughput, Unicode cluster ingestion,
scrollback throughput, dirty region coalescing, glyph atlas cache churn,
scrollback viewport navigation, render planning, glyph quad generation,
144Hz frame-scheduler decisions, rasterized glyph cache reuse, prepared surface glyph-frame
construction, native input-to-render plumbing, real-font rasterization,
deterministic PTY output pumping, real PTY shell-output bursts, real PTY
input/output roundtrips, bounded runtime state under repeated output batches,
and runtime alternate-screen transitions, plus runtime
focus/mouse/terminal-response protocol input paths, texture and glyph-atlas GPU
upload/readback, font-backed text-atlas GPU upload/readback, offscreen
textured-quad GPU draw/readback, offscreen terminal text GPU draw/readback, and
repeated offscreen terminal text GPU draw/readback timing.

They do not prove the full performance acceptance target by themselves. Hardware
backed 144Hz frame pacing on a 144Hz-capable display, p95 frame time, input
latency, idle CPU, memory growth, and broader live window runtime proof still
require separate live measurements.
The native runtime exposes bounded render-time and app-input-to-render latency
counters, including sample count, total, average, max, and bucketed p95
estimates, plus rendered dirty-region/cell counters, so live-window measurements
can be reported from structured counters instead of subjective observation.
On 2026-06-24, the `--window-smoke` and `--window-perf-smoke` commands were
tightened to fail unless the bounded native-window run records an actual surface
presentation. The latest local `--window-smoke` run
exited with `window smoke failed: no surface frame was presented; redraw
attempts: 16; surface timeouts: 0; surface occluded: 16`, and the latest local
`--window-perf-smoke` run exited with `window perf smoke failed: no glyph frame
was presented; redraw attempts: 768; frames presented: 0; surface timeouts: 0;
surface occluded: 768; glyph quads: 0; background quads: 0; cursor quads: 0`.
Hardware-backed live-window glyph presentation and active-monitor frame pacing
acceptance therefore remain unproven in the current state; the commands now
prevent stale empty-frame pacing output from being misreported as terminal
rendering proof.
On 2026-06-23, `cargo run -- --runtime-perf-smoke` pumped 1 deterministic PTY
echo byte, rendered 1 CPU-side frame, and reported rendered dirty-region work,
render sample/average/max/p95, and input-to-render sample/average/max/p95
counters. On 2026-06-24, `cargo run -- --runtime-perf-budget-smoke` ran the
same deterministic input-echo render path as an executable budget gate, failing
if render p95 exceeds 6940000 ns or input-to-render p95 exceeds 10000000 ns; it
pumped 1 byte, rendered 1 CPU-side frame, reported render p95 500000 ns, and
reported input-to-render p95 1000000 ns. On 2026-06-24,
`cargo run -- --runtime-perf-p95-smoke` repeated that deterministic input-echo
render path for 16 samples after the shaped-glyph placement fix, pumped 16
bytes, rendered 16 CPU-side frames, reported render p95 2000000 ns against the
6940000 ns budget, and reported input-to-render p95 4000000 ns against the
10000000 ns budget. This is deterministic runtime counter evidence, not live
windowed GPU frame pacing acceptance proof. `cargo run --
--runtime-real-shell-perf-budget-smoke` applies the same render and
input-to-render p95 budgets to the real `/bin/sh` PTY transcript path, making
the executable gate closer to daily shell usage while still remaining distinct
from live windowed GPU pacing proof. On 2026-06-25, `cargo run --
--runtime-real-shell-command-output-smoke` pumped 205 bytes from a real
`/bin/sh`, wrote 97 PTY input bytes, observed two command-output rows plus a
post-command prompt marker, rendered 3 CPU-side frames, and proved a full redraw
preserved the command output in the render plan. On 2026-06-24,
`cargo run -- --runtime-glyph-frame-smoke` pumped 19 bytes, planned 16 glyphs,
rasterized 12 glyphs, reused 4 glyphs, built 16 prepared quads, produced one
selection background, one cursor quad, a 604x204 frame, a 40128-byte atlas, 44 px
line height, and 14 px surface padding through the native glyph-frame path. On
2026-06-24,
`cargo run -- --runtime-glyph-frame-snapshot target/gromaq-runtime-glyph-frame.ppm`
wrote a 604x204 binary PPM CPU preview from the same prepared glyph-frame path,
reported 369663 bytes written, 123216 preview pixels, 16 prepared quads, one
background quad, one cursor quad, and 40128 atlas bytes. This is an inspectable
prepared-frame artifact, not a live desktop screenshot capture. On
2026-06-24,
`cargo run -- --window-glyph-frame-snapshot target/gromaq-window-glyph-frame.ppm`
launched the bounded native-window path and wrote a 2556x1586 binary PPM with
12161465 bytes, 60 glyph quads, 0 background quads, and 1 cursor quad from the
prepared glyph-frame snapshot path; it reported `glyph frame presented: false`
because this command captures the prepared artifact and is not an OS compositor
screenshot. On
2026-06-23,
`cargo run -- --runtime-large-output-smoke` pumped 12288 bytes from 512 lines,
reported 128 retained scrollback lines, rendered 1 CPU-side dirty frame,
reported viewport-capped rendered dirty-region work, verified
`gromaq-runtime-line-511` in the render plan, and reported render p95 500000 ns.
`cargo run -- --runtime-real-shell-large-output-smoke` enforces the 6940000 ns
render p95 budget for a real `/bin/sh` large-output transcript while proving
bounded scrollback eviction. On this machine it pumped 7168 bytes, rendered one
dirty frame, retained the 64-line scrollback cap, evicted the first line,
observed the last line, and reported render p95 1000000 ns.
On the same date, `cargo run -- --runtime-bounded-state-smoke`
pumped 51200 bytes from 2048 lines across 4 batches, retained 128 scrollback
lines and 128 styled scrollback cell rows, used the runtime state snapshot to
keep retained cell data within the 4096-cell deterministic cap for a 32-column
bounded runtime, rendered 4 CPU-side dirty frames, reported viewport-capped
rendered dirty-region work, and verified
`gromaq-bounded-line-2047` in the render plan. On 2026-06-24,
`cargo run -- --runtime-memory-smoke` extended that deterministic long-session
path with one warmup batch, 8 measured batches, process RSS sampling through
`ps`, a 65536 KiB RSS growth cap after warmup, and the same capped
scrollback-state assertions; it pumped 110592 bytes from 4608 lines, rendered 9
CPU-side frames, retained 128 scrollback lines, reported RSS growth of 1280 KiB,
and verified `gromaq-memory-line-4607` in the render plan. On 2026-06-23,
`cargo run -- --runtime-continuous-output-smoke` pumped 6912 bytes from 256
lines across 32 small PTY batches, rendered each dirty batch, verified the
configured 64-line scrollback cap, reported viewport-capped rendered dirty-region
work and render p95 500000 ns, and checked that `gromaq-continuous-line-255`
reached the latest visible render plan. On the same date,
`cargo run -- --runtime-scrollback-smoke` pumped 32 bytes, used Shift+PageUp and
Shift+PageDown through `NativeTerminalRuntime`, locally scrolled 4 retained
history rows without PTY writes, rendered 3 CPU-side dirty frames, and restored
the live visible lines `four|five|six`. On the same date,
`cargo run -- --runtime-alternate-screen-smoke` pumped primary, alternate, and
restore output stages through the runtime, rendered 3 CPU-side dirty frames,
restored primary visible content, and suppressed alternate-screen scrollback.
On the same date, `cargo run -- --runtime-reflow-smoke` pumped 80
bytes, reported 1 resize event, preserved 2 reflowed scrollback lines with
styled metadata, rendered visible lines `klmno|pqrst`, and rendered 1 CPU-side
dirty frame. On the same date, `cargo run -- --runtime-idle-smoke` pumped 0
bytes, reported 16 render attempts, 16 clean-frame skips, and 0 rendered frames
through the deterministic CPU-side path. On 2026-06-24,
`cargo run -- --runtime-idle-cpu-smoke` sampled the same clean-frame idle path 5
times at 50 ms intervals, reported max process CPU of 1.8% against the 5.0%
budget, and preserved the 16 clean-frame skips with 0 rendered frames. On
2026-06-23, `cargo run -- --frame-scheduler-smoke` reported a 6944444 ns 144Hz
target interval, 4944444 ns frame-paced wait, 3 presented frames, and 2 dropped
frames. On 2026-06-24, `cargo run -- --gpu-terminal-text-smoke` drew a 144x36
offscreen terminal frame with 4 glyphs, 4 glyph quads, 1 background quad, 1
decoration quad, 1 cursor quad, 3 rasterized glyphs, 1 reused glyph, 1523 drawn
pixels, a sampled background pixel of `[1, 2, 2, 255]`, a sampled glyph pixel of
`[254, 253, 214, 254]`, and a contrast ratio of 2002 x100 against the 700 x100
minimum smoke gate. On the same date,
`cargo run -- --gpu-terminal-text-perf-smoke` measured 16 repeated offscreen
terminal text GPU draw/readback frames at 144x36 pixels, reported 1523 drawn
pixels on the final frame, and reported min/avg/max/p95 draw/readback timings of
5867417/6755039/11309667/11309667 ns. On the same date,
`cargo run -- --gpu-terminal-text-snapshot target/gromaq-gpu-terminal-text.ppm`
wrote a 144x36 binary PPM artifact of the same contrast-gated terminal-text
smoke frame, reported 15566 bytes written, 4 glyphs, 1523 drawn pixels, sampled
background `[1, 2, 2, 255]`, sampled glyph `[254, 253, 214, 254]`, cursor
`[200, 200, 200, 255]`, and the same 2002 x100 contrast ratio. These are
deterministic smoke results, an inspectable offscreen snapshot artifact, and
offscreen GPU draw/readback timing results, not live hardware acceptance
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
- `unicode_emoji_cluster_output`
- `scrollback_large_output`
- `scrollback_view_navigation`
- `dirty_region_coalescing`
- `glyph_atlas_cache_churn`
- `frame_scheduler_144hz_timeline`
- `render_plan_large_dirty_region`
- `glyph_quad_generation_large_plan`
- `rasterized_glyph_cache_hot_plan`
- `prepared_surface_glyph_frame_large_plan`
- `native_input_echo_render_cycle`
- `font_rasterizer_combining_cell`
- `pty_runtime_pump_large_output`
- `real_pty_shell_large_output_burst`
- `real_pty_shell_input_echo_roundtrip`
- `runtime_bounded_state_batches`
- `runtime_state_snapshot_bounded_session`
- `runtime_continuous_output_batches`
- `runtime_alternate_screen_stages`
- `runtime_protocol_input_reports`
- `gpu_textured_quad_draw_readback`
- `gpu_terminal_text_draw_readback`
- `gpu_text_atlas_upload_readback`
- `gpu_texture_upload_readback`
- `gpu_glyph_atlas_upload_readback`

## Current Full Local Run

On 2026-06-24, a full `cargo bench` run completed on macOS Darwin 23.5.0
arm64 with an Apple M1 Pro. Criterion reported no `Performance has regressed`
lines in the captured output. Gnuplot was not available, so Criterion used the
plotters backend.

| Benchmark | Measured time range |
| --- | ---: |
| `parser_large_output` | 8.8378-8.8944 ms |
| `unicode_emoji_cluster_output` | 6.0104-6.0441 ms |
| `scrollback_large_output` | 6.3080-6.3475 ms |
| `scrollback_view_navigation` | 4.6589-4.7105 s before hot-path fix; 19.365-20.240 ms after |
| `dirty_region_coalescing` | 623.27-628.25 ns |
| `glyph_atlas_cache_churn` | 2.1612-2.1782 ms |
| `frame_scheduler_144hz_timeline` | 9.0973-9.1711 us |
| `render_plan_large_dirty_region` | 155.03-157.72 us |
| `glyph_quad_generation_large_plan` | 22.870-23.136 us |
| `rasterized_glyph_cache_hot_plan` | 18.869-19.117 us |
| `prepared_surface_glyph_frame_large_plan` | 40.286-40.701 us |
| `native_input_echo_render_cycle` | 111.12-113.73 us |
| `font_rasterizer_combining_cell` | 3.6378-3.6865 us |
| `pty_runtime_pump_large_output` | 8.9816-9.2103 ms |
| `real_pty_shell_large_output_burst` | 82.405-84.192 ms |
| `real_pty_shell_input_echo_roundtrip` | 15.422-15.873 ms |
| `runtime_bounded_state_batches` | 6.0650-6.1365 ms |
| `runtime_state_snapshot_bounded_session` | 65.269-66.154 us |
| `runtime_continuous_output_batches` | 1.6107-1.7109 ms |
| `runtime_alternate_screen_stages` | 23.159-24.282 us |
| `runtime_protocol_input_reports` | 1.2273-1.2448 us |
| `gpu_textured_quad_draw_readback` | 1.5650-1.5952 ms |
| `gpu_terminal_text_draw_readback` | 32.000-32.464 ms |
| `gpu_text_atlas_upload_readback` | 31.170-31.839 ms |
| `gpu_texture_upload_readback` | 1.5644-1.6099 ms |
| `gpu_glyph_atlas_upload_readback` | 1.5498-1.5586 ms |

Criterion warned that these benchmarks could not complete 100 samples inside
the default 5-second target: `scrollback_view_navigation`,
`real_pty_shell_large_output_burst`, `runtime_continuous_output_batches`,
`gpu_textured_quad_draw_readback`, `gpu_texture_upload_readback`, and
`gpu_glyph_atlas_upload_readback`. The slowest path was
`scrollback_view_navigation`, which required an estimated 461.1 seconds for 100
samples and measured multi-second viewport navigation iterations. A follow-up
hot-path fix removed full scrollback snapshot cloning from visible history-grid
dumps; targeted `scrollback_view_navigation` reruns on 2026-06-24 measured
19.365-20.240 ms. Criterion reported a 99.578%-99.587% improvement for the
first post-fix rerun, then reported the second post-fix rerun as within the
noise threshold. This is deterministic CPU-side scrollback-view evidence, not
live-window smooth-scrolling acceptance proof.

Some benchmarks load a local monospace font for real glyph rasterization. The
benchmark harness checks common macOS and Linux font paths. If no candidate is
available, the font-dependent benchmark name is still registered and emits a
clear skip message instead of panicking; that skip does not prove rasterization
throughput on the current machine. The real PTY benchmark registers a skip
placeholder when `/bin/sh` is unavailable; that skip does not prove real PTY
throughput on the current machine. The GPU draw/readback benchmarks similarly
register skip placeholders when no compatible native GPU adapter can be created;
that skip does not prove GPU draw/readback throughput on the current machine.

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
- near-zero live-window idle CPU
- live window process memory stability during real interactive long sessions
- smooth interactive scrolling with a live window surface

Those remain acceptance criteria for later live measurement work.
