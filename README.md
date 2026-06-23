# Gromaq

Gromaq is a native Rust terminal emulator foundation for `gromaq.dev`.

The project goal is a GPU-rendered, performance-first terminal emulator that is correct enough for daily use. This repository is currently at the foundation stage: deterministic terminal state, ANSI parsing, scrollback, resize preservation, input encoding, clipboard boundaries, configuration validation, a structured test API, a PTY boundary, native GPU adapter/device bootstrap, a GPU renderer planning boundary, and benchmarks.

## Current Status

Implemented and tested:

- Terminal grid/state engine
- ANSI SGR colors/text attributes including bold, dim, italic, inverse video, slow/rapid blink, hidden, overline, strikethrough, colon-form underline styles, and SGR underline color/reset; cursor movement, erase-line, erase-display, DEC autowrap/origin modes, RIS full reset handling, and DECSTR soft reset handling
- VT editing/navigation subset: default and configurable tab stops, C0 LF/VT/FF linefeed controls, C1 8-bit IND/NEL/HTS/RI/DECID equivalents, DEC special graphics G0/G1 box-drawing charsets with SI/SO shifting, DECALN screen alignment pattern, cursor forward/backward tabs, insert/delete/erase/repeat characters, insert/replace mode, insert/delete lines with scroll-margin bounds, viewport scroll up/down including the ECMA-48 scroll-down alias with scroll-margin bounds, DECSTBM linefeed, index/next-line, and reverse-index scroll margins, cursor absolute/relative row and column positioning including DEC origin mode, cursor next/previous line movement, cursor visibility/blink mode and DECSCUSR shape/blink state, DEC cursor/rendition save/restore, and SCO/private-mode cursor save/restore
- Terminal-generated Primary/Secondary DA, DECID, regular and DEC private DSR cursor-position/status replies, ANSI/DEC private mode-state replies, DECRQSS SGR/scroll-margin/cursor-shape status replies, and xterm window-state/window-position/pixel-size/text-area/screen size/icon-label/title reports with native PTY write-back
- OSC icon-label/title handling, OSC 8 hyperlink cell metadata, and bounded OSC 52 clipboard-text decoding
- Bracketed paste mode encoding
- Unicode wide-cell handling, including combining marks attached to wide glyphs and simple emoji ZWJ clusters
- Bounded scrollback
- Scrollback clearing via erase-display mode 3
- Visible-grid resize reflow for soft wraps, hard newlines, styled cells, and wide cells
- Styled scrollback row reflow during resize
- Dirty-region tracking for renderer scheduling
- Native `winit` app lifecycle boundary and window attributes
- Keyboard input encoding for common keys, navigation keys, modified named keys, F1-F12 keys, Shift+Tab, control-punctuation bytes, application cursor-key mode, focus reports, committed platform text, paste payloads, native clipboard paste shortcuts, mouse reports, and native `winit` key events
- Configuration validation
- Structured `TerminalTestApi`
- Deterministic performance counters for parser bytes, dirty cells, dirty-region batches, scrolls, and resizes
- Deterministic one-pixel-per-cell test API screenshot capture
- Alternate-screen enter/leave restoration
- Visible-grid selection and copy
- Host clipboard abstraction with deterministic in-memory and native OS text adapters plus a native read/write smoke command
- SGR mouse reporting mode and press/release/drag/motion event encoding
- PTY boundary plus a real shell-command lifecycle test
- Real PTY command workflow tests for available `bash` and `zsh`
- Real PTY command lifecycle checks for available `fish`, `nushell`, `vim`, `nvim`, `tmux`, `less`, `top`, `htop`, `btop`, `ssh`, `kubectl`, and `cargo test`
- Real PTY background-reader large-output burst drain test
- No-argument binary dispatch to the native terminal app loop
- Launched native app runtime starts and retains a shell PTY session
- Non-blocking PTY output drain into terminal state and PTY input writes
- Raw PTY byte ingestion into the terminal parser without lossy string conversion on the pump hot path
- Native runtime terminal resize with retained PTY resize notification
- Native window resize mapping to terminal dimensions with redraw request
- Native keyboard, application cursor-key, focus-report, committed text, clipboard paste, terminal paste, and terminal mouse-report bytes written to the retained PTY session
- OSC 52 clipboard payload write-through to the host clipboard abstraction plus a native OSC 52 clipboard smoke command
- Native window mouse coordinates mapped to terminal grid mouse reports
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
- Offscreen textured-quad GPU draw/readback smoke test with `--gpu-textured-quad-smoke`
- Offscreen terminal text GPU draw/readback smoke test with `--gpu-terminal-text-smoke`
- GPU renderer boundary with deterministic render-plan and glyph-quad generation
- Owned surface glyph-frame preparation from render plans and rasterized glyph bitmaps
- Deterministic `wgpu` surface configuration planner
- Deterministic `wgpu` surface lifecycle state for configure, resize, and zero-size deferral
- Surface configuration controller that applies configure/reconfigure actions to a surface backend
- App-owned native window surface state for initial configuration, resize, and zero-size suspension
- Safe `wgpu` window surface creation from `NativeGpuContext` for app handoff
- Presentable `wgpu` surface backend path for clear-pass frame acquisition, queue submission, and present
- Presentable `wgpu` surface backend path for supplied terminal glyph atlas and quad batches
- Native app wiring that creates, configures, resizes, and presents prepared terminal glyph frames to the window surface
- Default native monospace font discovery for app-owned glyph rasterization cache
- Deterministic 144Hz frame scheduler foundation
- Deterministic glyph atlas cache with hit/miss/eviction metrics
- Criterion benchmark harness

Not yet complete:

- OS paste event integration
- Hardware-backed 144Hz frame pacing proof
- Live desktop screenshot/runtime proof of windowed terminal glyph drawing
- Live desktop OS clipboard paste smoke
- End-to-end mouse workflows in `tmux`, editors, and alternate screen apps
- Full VT compatibility coverage for editors, multiplexers, and pagers
- Compatibility matrix proof against shells, editors, `tmux`, `ssh`, and large-output workflows
- Performance acceptance proof for 144Hz, frame time, input latency, idle CPU, and memory growth

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
cargo run -- --clipboard-smoke
cargo run -- --osc52-clipboard-smoke
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
cargo bench
```

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
