# Compatibility

Compatibility proof is not complete.

| Workflow | Status | Evidence |
| --- | --- | --- |
| ANSI SGR colors and text attributes | Partial | `tests/parser.rs` covers ANSI colors, semicolon and colon-form indexed/truecolor colors, bold, dim, slow/rapid blink, hidden/conceal, overline, strikethrough, colon-form underline styles without italic side effects, SGR underline color/reset, and reset behavior |
| Cursor movement, erase-line, erase-display, wrapping, and reset | Partial | `tests/parser.rs` and `tests/terminal_state.rs` cover cursor movement, erase-line, visible-screen erase-display modes 0/1/2, scrollback-clearing mode 3, DEC autowrap/origin modes, RIS reset of visible state, scrollback, modes, margins, style, and cursor, and DECSTR soft reset of modeled modes including autowrap without clearing visible content |
| Terminal status and capability reports | Partial | `tests/terminal_state.rs` covers Primary DA `CSI c`/`CSI 0c`, DECID `ESC Z`, Secondary DA `CSI > c`/`CSI > 0c`, DSR `CSI 5n`, regular cursor-position `CSI 6n`, DEC private cursor-position `CSI ? 6n`, xterm window-state report `CSI 11t`, deterministic origin window-position report `CSI 13t`, and xterm text-area size report `CSI 18t` response bytes; `tests/osc_and_paste.rs` covers xterm window-title report `CSI 21t`; `tests/app.rs` covers native runtime write-back of terminal-generated responses to the PTY |
| VT editing/navigation subset | Partial | `tests/vt_editing.rs` and `tests/terminal_state.rs` cover default and configurable tab stops, C0 LF/VT/FF linefeed controls, C1 8-bit IND/NEL/HTS/RI/DECID equivalents, DEC special graphics G0/G1 box-drawing charsets with SI/SO shifting, DECALN screen alignment pattern, cursor forward/backward tabs, insert/delete/erase/repeat characters, insert/replace mode, insert/delete lines with scroll-margin bounds, viewport scroll up/down including the ECMA-48 scroll-down alias with scroll-margin bounds, DECSTBM linefeed, index/next-line, and reverse-index scroll margins, cursor absolute/relative row and column positioning including DEC origin mode, cursor next/previous line movement, cursor visibility/blink mode, DECSCUSR cursor shape/blink state, DEC cursor/rendition save/restore, and SCO/private-mode cursor save/restore |
| OSC title, hyperlink, and clipboard handling | Partial | `tests/osc_and_paste.rs` covers OSC 0/2 title updates, OSC 8 hyperlink metadata on printed cells, and bounded OSC 52 clipboard-text decoding; `tests/app.rs` covers host clipboard abstraction sync; live OS clipboard smoke pending |
| Bracketed paste mode | Partial | `tests/osc_and_paste.rs` |
| Unicode wide and combining characters | Partial | `tests/terminal_state.rs` covers wide-cell leading/trailing metadata and combining marks attached to wide glyphs; broader grapheme-cluster shaping and emoji sequence coverage pending |
| Bounded scrollback | Partial | `tests/terminal_state.rs` covers bounded scrollback append and erase-display mode 3 scrollback clearing |
| Visible-grid and scrollback resize reflow | Partial | `tests/reflow.rs` covers visible-grid soft-wrap/hard-newline/style/wide-cell reflow and styled scrollback row reflow |
| Dirty-region tracking | Partial | `tests/dirty_regions.rs` |
| Structured performance counters | Partial | `tests/test_api.rs` covers parser bytes, dirty cells, non-empty dirty-region batches, scrolls, and resizes through `TerminalTestApi::dump_perf_metrics`; live frame-time/input-latency/CPU/memory acceptance proof pending |
| Structured test API screenshots | Partial | `tests/test_api.rs` covers deterministic one-pixel-per-cell RGBA screenshots of visible terminal text and cursor state; live GPU/window screenshot proof pending |
| Selection/copy | Partial | `tests/selection.rs` |
| Host clipboard adapter | Partial | `tests/clipboard.rs` covers copy into deterministic memory clipboard and native OS adapter construction; live OS clipboard read/write smoke pending |
| Alternate screen | Partial | `tests/alternate_screen.rs` |
| SGR mouse press/release/drag/motion reporting | Partial | `tests/mouse.rs` |
| `/bin/sh` PTY command | Partial | `tests/pty.rs` |
| Native `winit` application lifecycle | Partial | `tests/app.rs` covers window attributes, resume, timed PTY pump deadlines, PTY output-ready user events, output-driven redraw scheduling, dirty terminal frame rendering through the renderer boundary, close, and destroy lifecycle state |
| No-argument native app launch dispatch | Partial | `tests/cli.rs` verifies CLI dispatch to the native app launcher without invoking GPU smoke paths |
| Native app shell PTY startup | Partial | `tests/app.rs` verifies runtime terminal dimensions, PTY config, single shell spawn, and retained session handle |
| Native app PTY I/O pump/input/resize | Partial | `tests/app.rs` verifies drained PTY output reaches terminal state, terminal-generated status responses are written back to the PTY, schedules redraw when output is applied, syncs terminal-owned clipboard text through the host clipboard abstraction, maps native window resize events to terminal/PTY sizes, maps native window mouse coordinates to grid cells, resizes terminal state and the retained PTY session, encodes native key input, application cursor-key mode, focus reports, committed platform text, clipboard paste shortcuts, bracketed paste text, terminal mouse reports, and PTY output-ready event proxy wakeups, and writes input bytes to the retained session; `tests/input.rs` covers `winit` printable, arrow, Home/End, Insert/Delete, PageUp/PageDown, F1-F12, modified named-key, and modified-character key encoding; `tests/mouse.rs` covers SGR mouse encoding; `tests/osc_and_paste.rs` covers bracketed paste encoding; `tests/pty.rs` verifies real background reader drain and wakeup notifications |
| `bash` | Partial | `tests/pty.rs` runs a short command through a real PTY when `bash` is available; interactive shell workflow pending |
| `zsh` | Partial | `tests/pty.rs` runs a short command through a real PTY when `zsh` is available; interactive shell workflow pending |
| `fish` | Partial | `tests/pty.rs` runs a short command through a real PTY when `fish` is available; interactive shell workflow pending |
| `nushell` | Partial | `tests/pty.rs` runs a short command through a real PTY when `nu` is available; interactive shell workflow pending |
| `vim` / `nvim` | Partial | `tests/pty.rs` runs `vim --version` and `nvim --version` through a real PTY when available; interactive editor workflows pending |
| `tmux` | Partial | `tests/pty.rs` runs `tmux -V` through a real PTY when available; live multiplexer pane and mouse-reporting workflows pending |
| `less` | Partial | `tests/pty.rs` runs `less --version` through a real PTY when available; interactive pager workflow pending |
| `top` / `htop` / `btop` | Partial | `tests/pty.rs` runs a bounded `top` snapshot and `htop`/`btop` version checks through a real PTY when available; interactive full-screen process-monitor workflows pending |
| `ssh` | Partial | `tests/pty.rs` runs `ssh -V` through a real PTY when available; remote/session workflow pending |
| `kubectl` output | Partial | `tests/pty.rs` runs `kubectl version --client=true` through a real PTY when available; large/live Kubernetes output scenarios pending |
| `cargo test` workflow | Partial | `tests/pty.rs` runs `cargo test --quiet` through a real PTY against a tiny fixture project when `cargo` is available; large repo test-output scenarios pending |
| Large continuous output | Partial | Criterion parser and scrollback benches; `tests/pty.rs` drains a 2,000-line real PTY burst through the background reader |
| Native `wgpu` adapter/device bootstrap | Partial | `tests/native_gpu.rs`, `tests/cli.rs`, `cargo run -- --gpu-info` on 2026-06-22 reported `Apple M1 Pro` / `Metal` |
| Offscreen GPU render/readback smoke | Partial | `cargo run -- --gpu-smoke` on 2026-06-22 read back `[26, 51, 77, 255]` from a 4x4 clear |
| GPU texture upload/readback smoke | Partial | `cargo run -- --gpu-upload-smoke` on 2026-06-22 matched `16/16` uploaded bytes |
| Glyph atlas packing/upload smoke | Partial | `tests/glyph_atlas_image.rs`, `cargo run -- --gpu-glyph-atlas-smoke` on 2026-06-22 matched `32/32` atlas bytes |
| Font-backed glyph rasterization | Partial | `tests/font_rasterizer.rs` renders a real macOS font glyph to RGBA8 and packs it into an atlas bitmap |
| Renderer-plan glyph bitmap population | Partial | `tests/rasterized_glyph_cache.rs` rasterizes distinct planned glyph atlas entries once, returns cached bitmaps for repeated render plans, and packs the resulting bitmaps |
| Font-backed text atlas GPU upload/readback | Partial | `tests/cli.rs`, `cargo run -- --gpu-text-atlas-smoke` on 2026-06-22 matched `1144/1144` atlas bytes for a 22x13 real-font text atlas |
| Offscreen textured GPU quad draw/readback | Partial | `tests/cli.rs`, `cargo run -- --gpu-textured-quad-smoke` on 2026-06-22 rendered a 4x4 target with first pixel `[255, 0, 0, 255]` and 16 drawn pixels |
| Offscreen terminal text GPU draw/readback | Partial | `tests/cli.rs`, `cargo run -- --gpu-terminal-text-smoke` on 2026-06-22 drew 3 terminal-planned glyph quads with 212 non-transparent output pixels |
| `wgpu` surface configuration planning | Partial | `tests/surface_config.rs` selects sRGB format, FIFO mode, opaque alpha, render-attachment usage, and rejects invalid capabilities |
| `wgpu` surface lifecycle and configuration execution | Partial | `tests/surface_config.rs` covers config planning, configure/reconfigure/zero-size lifecycle state, and applying configure/reconfigure actions to a surface backend; `tests/app.rs` covers app-owned native window surface state across initial configure, resize, zero-size suspension, clear-frame presentation delegation, and glyph-frame presentation delegation; live window surface creation pending |
| Deterministic 144Hz frame scheduler | Partial | `tests/frame_scheduler.rs` |
| Deterministic glyph atlas cache | Partial | `tests/glyph_atlas.rs` |
| Dirty-region render planning | Partial | `tests/render_plan.rs`, `tests/wgpu_renderer.rs` |
| Terminal glyph quad generation | Partial | `tests/glyph_quads.rs` builds pixel-space quads and atlas UVs, including wide glyphs, from render-plan output |
| End-to-end mouse workflows | Not proven | `tmux`, editors, and alternate-screen app scenarios pending |
| Hardware-backed frame pacing | Not proven | Native surface clear-present loop is wired, but 144Hz frame pacing has not been measured on live hardware |
| Windowed terminal glyph drawing | Partial | Native redraw events render dirty terminal snapshots through `WgpuRenderer` planning, default native monospace font discovery creates an app-owned rasterized glyph cache, prepared surface glyph frames can be built from render plans and rasterized glyph bitmaps, `NativeTerminalApp` wires dirty redraws to glyph-frame presentation, and `WgpuSurfaceBackend` can acquire, draw supplied terminal glyph atlas/quad batches, submit, and present a configured surface frame; live desktop screenshot/runtime proof pending |
| Full scrollback resize reflow | Partial | Styled scrollback rows rewrap on resize; richer logical-line metadata pending |
| Native OS clipboard integration | Partial | `NativeClipboard` provides a text backend through `arboard`; native app maps Control+V/Super+V to terminal paste routing and syncs terminal-owned clipboard text through the host clipboard abstraction; live OS clipboard smoke pending |
| OSC 52 clipboard writes | Partial | Valid OSC 52 clipboard payloads are decoded into terminal clipboard state and synchronized through the host clipboard abstraction; live OS clipboard write-through smoke pending |
