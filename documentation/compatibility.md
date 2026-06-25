# Compatibility Matrix

This matrix records what the repository currently proves through deterministic
tests, real PTY workflows, scripted interaction, and live smoke commands. It is
not a claim of full daily-driver compatibility yet; it is a proof map for the
remaining terminal-core work.

## Shells

| Workflow | Current proof | Status |
| --- | --- | --- |
| `/bin/sh` interactive input/output | Native PTY smoke, real-shell command-output smoke, and app runtime tests | Proven in CI/local tests |
| `bash` command lifecycle | Real PTY command workflow when available | Conditional on host binary |
| `zsh` command lifecycle and repaint preservation | Real PTY command workflow plus native redraw preservation test and `cargo run -- --runtime-repaint-smoke` deterministic zsh-style prompt repaint proof | Conditional on host binary for real PTY lifecycle; deterministic repaint proof is proven |
| `fish` command lifecycle | Real PTY command workflow when available | Conditional on host binary |
| `nushell` command lifecycle | Real PTY command workflow when available | Conditional on host binary |

## Editors, Pagers, and Multiplexers

| Workflow | Current proof | Status |
| --- | --- | --- |
| `vim` launch workflow | Real PTY command workflow when available | Conditional on host binary |
| `vim` alternate-screen enter/exit | Scripted real PTY workflow when available | Conditional on host binary |
| `vim` SGR mouse split selection | Scripted real PTY workflow when available | Conditional on host binary |
| `nvim` launch workflow | Real PTY command workflow when available | Conditional on host binary |
| `nvim` alternate-screen enter/exit | Scripted real PTY workflow when available | Conditional on host binary |
| `nvim` SGR mouse split selection | Scripted real PTY workflow when available | Conditional on host binary |
| `tmux` launch workflow | Real PTY command workflow when available | Conditional on host binary |
| `tmux` SGR mouse pane selection | Scripted real PTY workflow when available | Conditional on host binary |
| `less` launch workflow | Real PTY command workflow when available | Conditional on host binary |
| `less` alternate-screen enter/exit | Scripted real PTY workflow when available | Conditional on host binary |

## CLI and TUI Programs

| Workflow | Current proof | Status |
| --- | --- | --- |
| `top`, `htop`, `btop` launch workflows | Real PTY command workflows when available | Conditional on host binaries |
| `ssh` launch workflow | Real PTY command workflow when available | Conditional on host binary |
| `kubectl` output workflow | Real PTY command workflow when available | Conditional on host binary |
| `cargo test -- --nocapture` output | Real PTY fixture workflow with deterministic large output | Proven when Cargo is available |

## Terminal Features

| Feature | Current proof | Status |
| --- | --- | --- |
| Alternate-screen enter/leave restoration | Golden state tests and runtime alternate-screen smoke | Proven |
| Resize/reflow | Golden reflow tests and runtime reflow smoke | Proven for covered cases |
| ANSI/SGR styling | Parser, state, and fixture tests | Proven for covered sequences |
| Unicode wide/combining/emoji clusters | Terminal-state tests and glyph rasterization tests | Proven for covered clusters |
| Scrollback retention and viewport navigation | Unit/integration tests and runtime scrollback smoke | Proven |
| Selection/copy and OSC 52 clipboard | Unit tests and native clipboard smoke paths | Proven for covered paths |
| Keyboard input modes | Unit/integration tests for common, application cursor, keypad, focus, paste, and native shortcuts, including dedicated Paste, Shift+Insert, Control+V, and Super+V paste routing | Proven for covered keys |
| Browser-style terminal text zoom | Native shortcut mapping tests plus native app renderer/grid reconfiguration tests for increase, decrease, reset, shifted plus, modifier-wheel, and dedicated OS/browser zoom-key policy; `cargo run -- --runtime-text-zoom-smoke` verifies default 37/21/51 px metrics zoom to 43/24/59 px, reduces the visible grid from 59x15 to 52x13, and resets without a live GPU window | Proven for covered controls |
| Mouse reporting modes | Runtime mouse smoke now covers SGR press, release, drag, any-motion, and wheel report writeback; alternate-screen mouse tests cover runtime app paths | Proven for default and SGR covered paths |
| Shell prompt repaint output retention | App redraw tests and `cargo run -- --runtime-repaint-smoke` prove zsh-style prompt repaint sequences preserve the command line, two output rows, and repainted prompt in a full-viewport render plan after the swapchain-clear boundary | Proven for covered repaint sequence |
| Theme color propagation | Renderer config mapping plus prepared-frame tests for background, ANSI foreground, selection, and cursor colors | Proven for covered paths |
| Built-in theme legibility | Config contrast tests for foreground, cursor, selection, and readable ANSI slots across shipped presets, `cargo run -- --theme-legibility-smoke` CLI proof for the shipped default `gromaq-ghostty` preset and default text metrics, `cargo run -- --theme-preview-snapshot <path>` PPM artifact proof for default text, ANSI colors, selection, cursor, and padding with high-contrast text-pixel plus exact selection/cursor pixel gates, plus prepared-frame preview pixel tests for default padding, foreground glyph coverage, cursor color, and unclipped cell edges | Proven for shipped presets and default prepared-frame path |
| Default terminal font stack | Native font resolver tests for polished user fonts, including JetBrains Mono Nerd Font, MesloLGS Nerd Font, Geist Mono, Monaspace Neon, Fira Code, and Hack, before system fallbacks | Proven for covered paths |

## Live Native Window Proof

| Workflow | Current proof | Status |
| --- | --- | --- |
| Native window startup | `cargo run -- --window-smoke` on 2026-06-25 presented one native surface frame after 4 redraw attempts with 0 timeouts and 3 occluded acquisitions | Proven in current live run |
| Native glyph-frame presentation | `cargo run -- --window-perf-smoke` on 2026-06-25 presented 192 live glyph frames with a 2548x1568 glyph frame, 74 glyph quads, 73920 atlas bytes, and 17 occupied atlas slots | Proven in current live run |
| Prepared native glyph-frame preview artifact | `--runtime-glyph-frame-snapshot <path>` writes a deterministic PPM from the same owned glyph-frame preparation path used before surface presentation | Proven as CPU-side preview, not a live desktop screenshot |
| Native-window glyph-frame snapshot artifact | `cargo run -- --window-glyph-frame-snapshot target/gromaq-window-glyph-frame-current.ppm` on 2026-06-25 wrote a 2548x1568 PPM with 11985809 bytes, 60 glyph quads, and 1 cursor quad from the prepared native-window frame path | Proven as native presentation-path artifact, not an OS compositor screenshot |
| Active-monitor frame pacing | `cargo run -- --window-perf-smoke` on 2026-06-25 collected 180 post-warmup samples on a 120000 mHz monitor, reported `frame interval target limited by monitor: true`, measured exact p95 8845125 ns against the 10000000 ns active-monitor budget, recorded 0 dropped frames, and reported `frame pacing accepted: true` | Proven for current 120Hz active monitor |
| 144Hz display pacing | Requires live proof on 144Hz-capable hardware | Not yet proven |
| Desktop screenshot of windowed glyph output | Requires live screenshot proof | Not yet proven |
| Desktop OS paste menu | Requires native desktop workflow proof | Not yet proven |

## Remaining Matrix Work

- Run the same scripted workflows on a broader host matrix.
- Add live screenshot artifacts for the native window path.
- Add 144Hz hardware proof on a 144Hz-capable monitor.
- Expand editor/multiplexer interaction beyond launch and current scripted mouse
  workflows.
- Record pass/fail evidence for `ssh` and `kubectl` scenarios against real but
  safe targets.
