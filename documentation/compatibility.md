# Compatibility Matrix

This matrix records what the repository currently proves through deterministic
tests, real PTY workflows, scripted interaction, and live smoke commands. It is
not a claim of full daily-driver compatibility yet; it is a proof map for the
remaining terminal-core work.

## Shells

| Workflow | Current proof | Status |
| --- | --- | --- |
| `/bin/sh` interactive input/output | Native PTY smoke and app runtime tests | Proven in CI/local tests |
| `bash` command lifecycle | Real PTY command workflow when available | Conditional on host binary |
| `zsh` command lifecycle and repaint preservation | Real PTY command workflow plus native redraw preservation test | Conditional on host binary |
| `fish` command lifecycle | Real PTY command workflow when available | Conditional on host binary |
| `nushell` command lifecycle | Real PTY command workflow when available | Conditional on host binary |

## Editors, Pagers, and Multiplexers

| Workflow | Current proof | Status |
| --- | --- | --- |
| `vim` launch workflow | Real PTY command workflow when available | Conditional on host binary |
| `vim` SGR mouse split selection | Scripted real PTY workflow when available | Conditional on host binary |
| `nvim` launch workflow | Real PTY command workflow when available | Conditional on host binary |
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
| Mouse reporting modes | Runtime mouse smoke and alternate-screen mouse tests | Proven for default and SGR covered paths |
| Theme color propagation | Renderer config mapping plus prepared-frame tests for background, ANSI foreground, selection, and cursor colors | Proven for covered paths |
| Default terminal font stack | Native font resolver tests for polished user fonts, including JetBrains Mono Nerd Font and MesloLGS Nerd Font, before system fallbacks | Proven for covered paths |

## Live Native Window Proof

| Workflow | Current proof | Status |
| --- | --- | --- |
| Native window startup | `--window-smoke` | Proven on local live window |
| Native glyph-frame presentation | `--window-perf-smoke` reports glyph frame dimensions, quads, atlas bytes, and occupied slots | Proven on local live window |
| Prepared native glyph-frame preview artifact | `--runtime-glyph-frame-snapshot <path>` writes a deterministic PPM from the same owned glyph-frame preparation path used before surface presentation | Proven as CPU-side preview, not a live desktop screenshot |
| Active-monitor frame pacing | `--window-perf-smoke` warmup-excluded 120Hz run with zero dropped frames and accepted p95 budget | Proven on the current 120Hz display |
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
