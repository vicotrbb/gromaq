# Gromaq

Gromaq is a native Rust terminal emulator foundation for `gromaq.dev`.

The project goal is a GPU-rendered, performance-first terminal emulator that is correct enough for daily use. This repository is currently at the foundation stage: deterministic terminal state, ANSI parsing, scrollback, resize preservation, input encoding, clipboard boundaries, configuration validation, a structured test API, a PTY boundary, native GPU adapter/device bootstrap, a GPU renderer planning boundary, and benchmarks.

## Current Status

Implemented and tested:

- Terminal grid/state engine
- ANSI SGR colors/text attributes including semicolon and colon-form extended colors, colon truecolor with optional color-space slots, bold, dim, italic, inverse video, slow/rapid blink, hidden, framed, encircled, overline, strikethrough, colon-form underline styles, and SGR underline color/reset; cursor movement, erase-line, erase-display, DEC autowrap/origin modes, RIS full reset handling, and DECSTR soft reset handling
- VT editing/navigation subset: default and configurable tab stops, C0 LF/VT/FF column-preserving linefeed controls, ANSI linefeed/newline mode, C1 8-bit IND/NEL/HTS/RI/DECID equivalents, DEC special graphics G0/G1 box-drawing charsets with SI/SO shifting, DECPAM/DECPNM keypad mode, DECALN screen alignment pattern, cursor forward/backward tabs, insert/delete/erase/repeat characters, insert/replace mode, insert/delete lines with scroll-margin bounds, viewport scroll up/down including the ECMA-48 scroll-down alias with scroll-margin bounds, DECSTBM linefeed, index/next-line, and reverse-index scroll margins, cursor absolute/relative row and column positioning including DEC origin mode, cursor next/previous line movement, cursor visibility/blink mode and DECSCUSR shape/blink state, DEC cursor/rendition save/restore, and SCO/private-mode cursor/rendition save/restore
- Terminal-generated Primary/Secondary DA, DECID, regular and DEC private DSR cursor-position/status replies, ANSI/DEC private mode-state replies including alternate-screen state, DECRQSS SGR/scroll-margin/cursor-shape status replies, and xterm window-state/window-position/pixel-size/text-area/screen size/icon-label/title reports with native PTY write-back
- Bounded OSC icon-label/title handling, bounded OSC 8 hyperlink cell metadata, and bounded OSC 52 clipboard-text decoding
- File-backed golden terminal-state fixtures covering ANSI styling, OSC 8 hyperlink metadata, OSC title/icon-label and clipboard state, terminal-generated status/capability responses, wide-cell state, scrollback, cursor state, performance counters, and alternate-screen restoration, with update guidance in `tests/fixtures/README.md`
- Bracketed paste mode encoding
- Unicode wide-cell handling, including combining marks attached to wide glyphs, emoji presentation/keycap clusters, emoji-modifier clusters, regional-indicator pairs, emoji ZWJ clusters, and ZWJ clusters with internal emoji variation selectors
- Bounded scrollback
- Core scrollback viewport navigation for displaying retained history through the grid and screenshot snapshot APIs
- Long-output scrollback eviction capped to the configured scrollback limit
- Scrollback clearing via erase-display mode 3
- Visible-grid resize reflow for soft wraps, hard newlines, styled cells, and wide cells
- Styled scrollback row reflow during resize
- Dirty-region tracking for renderer scheduling
- Native `winit` app lifecycle boundary and window attributes
- Keyboard input encoding for common keys, navigation keys, named Space, modified named keys including Alt-modified Enter/Backspace/Escape, F1-F24 keys, Shift+Tab plus modified BackTab, control-punctuation bytes, physical numpad keys including Alt-modified numpad Enter and Alt-modified application-keypad sequences, application cursor-key mode, application keypad mode via DEC private mode and DECPAM/DECPNM, focus reports, committed platform text, paste payloads, native clipboard copy/paste shortcuts including Control+Insert, Shift+Insert, and dedicated OS clipboard keys, local Shift+PageUp/PageDown scrollback navigation, mouse reports, and native `winit` key events
- Configuration validation, including bounded visible-grid area before allocation, bounded finite font sizes, shell program/args/cwd validation, TOML config parsing from strings/files, defaulted partial config sections, validation after load, pretty TOML serialization, deterministic config-file reload checks that preserve the last valid config on invalid changes, native app/runtime/renderer config mapping, live config-file polling for reloadable native terminal/frame/render/shell settings, and config-file native app launch
- Structured `TerminalTestApi`
- Deterministic performance counters for parser bytes, dirty cells, dirty-region batches, scrolls, resizes, rendered dirty-region/cell work, measured native render durations, and app-input-to-render latency
- Structured native runtime tracing for startup, PTY pump/input, render, and clean-frame skip diagnostics
- Deterministic one-pixel-per-cell test API screenshot capture
- Alternate-screen enter/leave restoration including `1049` cursor/rendition save and restore
- Visible-grid selection and copy, including displayed scrollback viewport rows
- Host clipboard abstraction with deterministic in-memory and native OS text adapters plus a native read/write smoke command
- Xterm default and SGR mouse reporting modes with press/release/wheel/drag/motion event encoding, plus local scrollback navigation for non-reporting wheel input and Shift+PageUp/PageDown
- PTY boundary plus a real shell-command lifecycle test
- Interactive `/bin/sh` PTY input/output workflow through the native PTY writer and background reader
- Real PTY command workflow tests for available `bash` and `zsh`
- Real PTY command lifecycle checks for available `fish`, `nushell`, `vim`, `nvim`, `tmux`, `less`, `top`, `htop`, `btop`, `ssh`, `kubectl`, and `cargo test`
- Real PTY `cargo test -- --nocapture` fixture workflow with deterministic large output
- Scripted interactive PTY workflow checks for available `bash`, `zsh`, `fish`, and `nushell`
- Scripted interactive PTY workflow checks for available `vim`, `nvim`, `tmux`, and `less`
- Real PTY `less` alternate-screen enter/exit workflow when `less` is available
- Real PTY Vim SGR mouse-click split-window selection workflow when `vim` is available
- Real PTY tmux SGR mouse-click pane selection workflow when `tmux` is available
- Real PTY background-reader large-output burst drain test
- No-argument binary dispatch to the native terminal app loop
- Launched native app runtime starts and retains a shell PTY session
- Non-blocking PTY output drain into terminal state and PTY input writes
- Raw PTY byte ingestion into the terminal parser without lossy string conversion on the pump hot path
- Native runtime terminal resize with retained PTY resize notification
- Native window resize mapping to terminal dimensions with redraw request
- Native keyboard, application cursor-key, application keypad, focus-report, committed text, clipboard paste, terminal paste, and default/SGR terminal mouse-report bytes written to the retained PTY session
- OSC 52 clipboard payload write-through to the host clipboard abstraction plus a native OSC 52 clipboard smoke command
- Native window mouse coordinates mapped to terminal grid mouse press/release/drag/motion reports
- Native runtime alternate-screen SGR mouse press, release, drag, wheel, and any-motion reports written to the retained PTY session
- Timed native event-loop PTY polling with output-driven redraw requests
- PTY background-reader output-ready notifications through native event-loop user events
- Native redraw events render dirty terminal snapshots through the renderer boundary
- Swash-backed real-font glyph rasterization to RGBA8 atlas bitmaps
- Renderer-side rasterized glyph cache that populates distinct planned atlas entries once and returns cached bitmaps for repeated frames
- Native `wgpu` adapter/device bootstrap with `--gpu-info`
- Offscreen GPU render-pass clear and readback smoke test with `--gpu-smoke`
- GPU texture upload/readback smoke test with `--gpu-upload-smoke`
- Deterministic glyph atlas image packing and GPU upload/readback smoke test with `--gpu-glyph-atlas-smoke`
- Font-backed terminal text atlas packing and GPU upload/readback smoke test with `--gpu-text-atlas-smoke`
- TOML config template output with `--config-template`
- TOML config validation command with `--config-check <path>`
- TOML config native app launch with `--config <path>`
- Offscreen textured-quad GPU draw/readback smoke test with `--gpu-textured-quad-smoke`
- Offscreen terminal text GPU draw/readback smoke test with a contrast-gated default-theme glyph sample, solid background, text-decoration, and cursor output via `--gpu-terminal-text-smoke`
- Repeated offscreen terminal text GPU draw/readback timing smoke with `--gpu-terminal-text-perf-smoke`
- Offscreen terminal text GPU snapshot export with `--gpu-terminal-text-snapshot <path>` for visual inspection of the default-theme smoke frame
- Deterministic runtime clipboard paste smoke with `--runtime-clipboard-paste-smoke`
- Deterministic runtime glyph-frame preparation smoke with `--runtime-glyph-frame-smoke`
- Deterministic runtime local scrollback navigation smoke with `--runtime-scrollback-smoke`
- Deterministic runtime performance smoke with `--runtime-perf-smoke`
- Deterministic runtime performance-budget smoke with `--runtime-perf-budget-smoke`, enforcing render p95 within 6.94ms and input-to-render p95 within 10ms for the CPU-side input-echo path
- Repeated deterministic runtime p95 smoke with `--runtime-perf-p95-smoke`, collecting 16 CPU-side input-echo render/input-to-render samples against the same budgets
- Deterministic runtime large-output smoke with `--runtime-large-output-smoke`
- Deterministic runtime state-footprint snapshot and bounded-state smoke with `--runtime-bounded-state-smoke`, including capped scrollback lines, styled cell rows, retained cell count, and retained-cell cap
- Deterministic runtime process-memory smoke with `--runtime-memory-smoke`, including warmup RSS sampling, repeated long-output batches, capped scrollback state, and bounded RSS growth
- Deterministic runtime continuous-output smoke with `--runtime-continuous-output-smoke`
- Deterministic runtime alternate-screen smoke with `--runtime-alternate-screen-smoke`
- Deterministic runtime scrollback resize/reflow smoke with `--runtime-reflow-smoke`
- Deterministic runtime config reload smoke with `--runtime-config-reload-smoke`
- Deterministic runtime focus-report smoke with `--runtime-focus-smoke`
- Deterministic runtime mouse-report smoke with `--runtime-mouse-smoke`
- Deterministic runtime terminal-response smoke with `--runtime-response-smoke`
- Deterministic runtime clean-frame idle smoke with `--runtime-idle-smoke`
- Deterministic runtime idle CPU smoke with `--runtime-idle-cpu-smoke`, sampling process CPU while clean frames are suppressed
- Deterministic 144Hz frame-scheduler smoke with `--frame-scheduler-smoke`
- GPU renderer boundary with deterministic render-plan and glyph-quad generation
- Owned surface glyph-frame preparation from render plans and rasterized glyph bitmaps
- Deterministic `wgpu` surface configuration planner
- Deterministic `wgpu` surface lifecycle state for configure, resize, and zero-size deferral
- Surface configuration controller that applies configure/reconfigure actions to a surface backend
- App-owned native window surface state for initial configuration, resize, and zero-size suspension
- Safe `wgpu` window surface creation from `NativeGpuContext` for app handoff
- Presentable `wgpu` surface backend path for clear-pass frame acquisition, queue submission, and present
- Presentable `wgpu` surface backend path for supplied terminal background, glyph atlas, glyph, and cursor batches
- Native app wiring that creates, configures, resizes, and presents prepared terminal glyph frames to the window surface
- Bounded live native-window smoke paths for one-frame startup and multi-frame presentation timing
- Default native monospace font discovery for app-owned glyph rasterization cache
- Deterministic 144Hz frame scheduler foundation
- Deterministic glyph atlas cache with hit/miss/eviction metrics
- Criterion benchmark harness

Not yet complete:

- Live desktop OS paste menu smoke
- Hardware-backed 144Hz frame pacing proof
- Live desktop screenshot proof of windowed terminal glyph drawing
- Broader alternate-screen mouse workflows beyond scripted Vim and tmux proof paths
- Full VT compatibility coverage for editors, multiplexers, and pagers beyond scripted PTY workflows
- Compatibility matrix proof against shells, editors, `tmux`, `ssh`, and large-output workflows
- Live performance acceptance proof for 144Hz, frame time, input latency, idle CPU, and real-session memory growth

## Development

```bash
cargo run
cargo run -- --gpu-info
cargo run -- --gpu-smoke
cargo run -- --gpu-upload-smoke
cargo run -- --gpu-glyph-atlas-smoke
cargo run -- --gpu-text-atlas-smoke
cargo run -- --gpu-textured-quad-smoke
cargo run -- --gpu-terminal-text-smoke
cargo run -- --gpu-terminal-text-perf-smoke
cargo run -- --gpu-terminal-text-snapshot target/gromaq-terminal-text.ppm
cargo run -- --clipboard-smoke
cargo run -- --config path/to/gromaq.toml
cargo run -- --config-check path/to/gromaq.toml
cargo run -- --config-template
cargo run -- --osc52-clipboard-smoke
cargo run -- --runtime-clipboard-paste-smoke
cargo run -- --runtime-glyph-frame-smoke
cargo run -- --runtime-scrollback-smoke
cargo run -- --runtime-perf-smoke
cargo run -- --runtime-perf-budget-smoke
cargo run -- --runtime-perf-p95-smoke
cargo run -- --runtime-large-output-smoke
cargo run -- --runtime-bounded-state-smoke
cargo run -- --runtime-memory-smoke
cargo run -- --runtime-continuous-output-smoke
cargo run -- --runtime-alternate-screen-smoke
cargo run -- --runtime-reflow-smoke
cargo run -- --runtime-config-reload-smoke
cargo run -- --runtime-focus-smoke
cargo run -- --runtime-mouse-smoke
cargo run -- --runtime-response-smoke
cargo run -- --runtime-idle-smoke
cargo run -- --runtime-idle-cpu-smoke
cargo run -- --frame-scheduler-smoke
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
cargo bench --bench parser_throughput -- --list
cargo bench
```

Example config:

```toml
[terminal]
cols = 132
rows = 40
scrollback_lines = 4096

[shell]
program = "/bin/zsh"
args = ["-l"]
cwd = "/tmp"

[font]
family = "monospace"
size_px = 21.0
# cell_width_px = 12
line_height_px = 29.0

[theme]
preset = "gromaq-dark"
background = "#090d12"
foreground = "#f4f7fb"
cursor = "#ffd27a"
selection = "#2b4162"
cursor_style = "block"
cursor_blinking = true
ansi = [
  "#151922", "#ff6b7a", "#7ee787", "#ffd27a",
  "#82aaff", "#c792ea", "#7dcfff", "#d7dde8",
  "#6b7280", "#ff8fa3", "#a6e3a1", "#f9e2af",
  "#89b4fa", "#f5c2e7", "#94e2d5", "#ffffff",
]
surface_padding_px = 14
dim_opacity = 0.66

[performance]
target_fps = 144
dirty_region_rendering = true
```

`gromaq-dark` is the built-in default theme preset. It provides the baseline
background, foreground, cursor, selection, ANSI palette, cursor style, cursor
blinking, surface padding, and dim text opacity; every field in `[theme]` can
still be overridden directly in TOML. The theme fields are documented in
[`documentation/theme.md`](documentation/theme.md).

`font.family = "monospace"` uses Gromaq's automatic mono stack: polished
user-installed terminal fonts such as JetBrains Mono Nerd Font, Cascadia Mono,
and Iosevka Term are preferred when present, then the app falls back to SF Mono,
Menlo, and common Linux mono fonts. Explicit `.ttf`, `.ttc`, and `.otf` file
paths are also supported. Supported named aliases currently include
`JetBrains Mono`, `JetBrains Mono Nerd Font`, `Cascadia Mono`,
`CaskaydiaCove Nerd Font`, `Iosevka Term`, `SF Mono`, and `Menlo`.

Benchmark coverage, expected benchmark names, reproducible local run steps, and
Criterion regression handling are documented in
[`documentation/benchmarks.md`](documentation/benchmarks.md).
Test fixture conventions and live-proof boundaries are documented in
[`TESTING.md`](TESTING.md).

Clippy warnings are treated as failures. The codebase forbids `unsafe` in the crate.

## Debugging

Use the structured test API for deterministic validation:

- `send_keys`
- `paste_text`
- `resize`
- `dump_grid`
- `dump_scrollback`
- `dump_cursor`
- `dump_perf_metrics`
- `screenshot`

`screenshot` returns a deterministic one-pixel-per-cell RGBA capture of terminal grid state for test assertions. Live GPU/window screenshot proof is still tracked separately.
