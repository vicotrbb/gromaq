# Compatibility Matrix

This matrix records what the repository currently proves through deterministic
tests, real PTY workflows, scripted interaction, and live smoke commands. It is
not a claim of full daily-driver compatibility yet; it is a proof map for the
remaining terminal-core work.

## Shells

| Workflow | Current proof | Status |
| --- | --- | --- |
| `/bin/sh` interactive input/output | Native PTY smoke, real-shell command-output smoke, and app runtime tests | Proven in CI/local tests |
| `bash` command lifecycle | Real PTY command and interactive workflows when available. On 2026-06-25, `cargo test --test pty` passed the current-host bash command and interactive checks. | Proven on current host; conditional elsewhere |
| `zsh` command lifecycle and repaint preservation | Real PTY command workflow plus native redraw preservation test and `cargo run -- --runtime-repaint-smoke` deterministic zsh-style prompt repaint proof. On 2026-06-25, `cargo test --test pty` passed the current-host zsh command and interactive checks. | Proven on current host; deterministic repaint proof is proven |
| `fish` command lifecycle | Real PTY command and interactive workflows when available. On 2026-06-25, the current host did not have `fish` on PATH, so the conditional PTY tests skipped this workflow. | Conditional on host binary; not proven on current host |
| `nushell` command lifecycle | Real PTY command and interactive workflows when available. On 2026-06-25, the current host did not have `nu` on PATH, so the conditional PTY tests skipped this workflow. | Conditional on host binary; not proven on current host |

## Editors, Pagers, and Multiplexers

| Workflow | Current proof | Status |
| --- | --- | --- |
| `vim` launch workflow | Real PTY command workflow when available. On 2026-06-25, `cargo test --test pty` passed current-host `vim --version` and scripted edit workflows. | Proven on current host; conditional elsewhere |
| `vim` alternate-screen enter/exit | Scripted real PTY workflow when available. On 2026-06-25, `cargo test --test pty` passed current-host Vim alternate-screen enter/exit proof. | Proven on current host; conditional elsewhere |
| `vim` SGR mouse split selection | Scripted real PTY workflow when available. On 2026-06-25, `cargo test --test pty` passed current-host Vim SGR mouse split-window selection proof. | Proven on current host; conditional elsewhere |
| `nvim` launch workflow | Real PTY command workflow when available. On 2026-06-25, the current host did not have `nvim` on PATH, so the conditional PTY tests skipped this workflow. | Conditional on host binary; not proven on current host |
| `nvim` alternate-screen enter/exit | Scripted real PTY workflow when available. On 2026-06-25, the current host did not have `nvim` on PATH, so the conditional PTY tests skipped this workflow. | Conditional on host binary; not proven on current host |
| `nvim` SGR mouse split selection | Scripted real PTY workflow when available. On 2026-06-25, the current host did not have `nvim` on PATH, so the conditional PTY tests skipped this workflow. | Conditional on host binary; not proven on current host |
| `tmux` launch workflow | Real PTY command and interactive pane workflows when available. On 2026-06-25, `cargo test --test pty` passed current-host `tmux -V` and interactive pane checks. | Proven on current host; conditional elsewhere |
| `tmux` SGR mouse pane selection | Scripted real PTY workflow when available. On 2026-06-25, `cargo test --test pty` passed current-host tmux SGR mouse pane-selection proof. | Proven on current host; conditional elsewhere |
| `less` launch workflow | Real PTY command and interactive search workflows when available. On 2026-06-25, `cargo test --test pty` passed current-host `less --version` and search checks. | Proven on current host; conditional elsewhere |
| `less` alternate-screen enter/exit | Scripted real PTY workflow when available. On 2026-06-25, `cargo test --test pty` passed current-host less alternate-screen enter/exit proof. | Proven on current host; conditional elsewhere |

## CLI and TUI Programs

| Workflow | Current proof | Status |
| --- | --- | --- |
| `top` launch workflow | Real PTY command workflow when available. On 2026-06-25, `cargo test --test pty` passed the current-host `top` snapshot check. | Proven on current host; conditional elsewhere |
| `htop`, `btop` launch workflows | Real PTY command workflows when available. On 2026-06-25, the current host did not have `htop` or `btop` on PATH, so the conditional PTY tests skipped these workflows. | Conditional on host binaries; not proven on current host |
| `ssh` launch workflow | Real PTY command workflow when available plus `cargo run -- --runtime-tool-workflow-smoke`, which runs `ssh -V` in a native PTY and requires `OpenSSH` output when the binary is present. On 2026-06-25 this smoke passed on the current host with 31 output bytes. | Proven for current host client/version workflow; smoke reports pass or skip elsewhere |
| `kubectl` output workflow | Real PTY command workflow when available plus `cargo run -- --runtime-tool-workflow-smoke`, which runs `kubectl version --client=true` in a native PTY and requires `Client` output when the binary is present. On 2026-06-25 this smoke passed on the current host with 52 output bytes. | Proven for current host client/version workflow; smoke reports pass or skip elsewhere |
| `cargo test -- --nocapture` output | Real PTY fixture workflow with deterministic large output. On 2026-06-25, `cargo test --test pty` passed current-host quiet and large-output cargo fixture checks. | Proven on current host |

## Terminal Features

| Feature | Current proof | Status |
| --- | --- | --- |
| Alternate-screen enter/leave restoration | Golden state tests and runtime alternate-screen smoke | Proven |
| Resize/reflow | Golden reflow tests and runtime reflow smoke | Proven for covered cases |
| ANSI/SGR styling | Parser, state, and fixture tests | Proven for covered sequences |
| Unicode wide/combining/emoji clusters | Terminal-state tests and glyph rasterization tests | Proven for covered clusters |
| Scrollback retention and viewport navigation | Unit/integration tests and runtime scrollback smoke | Proven |
| Selection/copy and OSC 52 clipboard | Unit tests and native clipboard smoke paths | Proven for covered paths |
| Structured test/control API | `TerminalTestApi` covers key encoding, paste, resize, visible grid, scrollback, cursor, performance counters, OSC title, OSC 52 clipboard text, terminal-generated response-byte draining, and deterministic one-pixel-per-cell screenshots | Proven for covered terminal snapshots and response bytes |
| Keyboard input modes | Unit/integration tests for common, application cursor, keypad, focus, paste, and native shortcuts, including dedicated Paste, Shift+Insert, Control+V, and Super+V paste routing | Proven for covered keys |
| Browser-style terminal text zoom | Native shortcut mapping tests plus native app renderer/grid reconfiguration tests for increase, decrease, reset, shifted plus, modifier-wheel, and dedicated OS/browser zoom-key policy; `cargo run -- --runtime-text-zoom-smoke` verifies default 32/18/44 px metrics zoom to 37/21/51 px, reduces the visible grid, and resets without a live GPU window | Proven for covered controls |
| Mouse reporting modes | Runtime mouse smoke now covers SGR press, release, drag, any-motion, and wheel report writeback; alternate-screen mouse tests cover runtime app paths | Proven for default and SGR covered paths |
| Shell prompt repaint output retention | App redraw tests and `cargo run -- --runtime-repaint-smoke` prove zsh-style prompt repaint sequences preserve the command line, two output rows, and repainted prompt in a full-viewport render plan after the swapchain-clear boundary | Proven for covered repaint sequence |
| Theme color propagation | Renderer config mapping plus prepared-frame tests for background, ANSI foreground, selection, and cursor colors | Proven for covered paths |
| Built-in theme legibility | Config contrast tests for foreground, cursor, selection, and readable ANSI slots across shipped presets, `cargo run -- --theme-legibility-smoke` CLI proof for the shipped default `gromaq-ghostty` preset and default text metrics, `cargo run -- --theme-preview-snapshot <path>` PPM artifact proof for default text, ANSI colors, selection, cursor, and padding with high-contrast text-pixel plus exact selection/cursor pixel gates, `cargo run -- --theme-preview-config <config> <path>` proof for TOML theme overrides plus background, cursor, and selection opacity through the same prepared-frame path, plus prepared-frame preview pixel tests for default padding, foreground glyph coverage, cursor color, straight-alpha opacity, and unclipped cell edges | Proven for shipped presets, default prepared-frame path, and covered TOML theme override path |
| Default and configured terminal font stack | Native font resolver tests for polished user fonts, including JetBrains Mono Nerd Font, MesloLGS Nerd Font, Geist Mono, Monaspace Neon, Fira Code, and Hack, before system fallbacks; `[font].fallback_families` parsing, config-check reporting, launch wiring, and reload tests cover ordered user fallback names or explicit font paths before automatic symbol and emoji fallbacks | Proven for covered primary and fallback paths |
| Default welcome preview artifact | `cargo run -- --welcome-preview-snapshot <path>` and CLI tests render the sectioned welcome screen to a deterministic PPM through the prepared glyph-frame path without GPU bootstrap, while checking glyph/cursor/atlas metrics and argument handling | Proven for deterministic prepared-frame artifact path |
| Frame FPS status overlay | Runtime rendering tests prove the FPS/status text is added only to a cloned render snapshot, appears in the rendered frame when its target cells are blank, appends an overlay dirty span, skips occupied right-prompt cells, and does not mutate terminal-owned grid text or shell scrollback | Proven for covered runtime render path |

## Live Native Window Proof

| Workflow | Current proof | Status |
| --- | --- | --- |
| Native window startup | `cargo run -- --window-smoke` on 2026-06-25 presented one native surface frame after 4 redraw attempts with 0 timeouts and 3 occluded acquisitions | Proven in current live run |
| Native glyph-frame presentation | `cargo run -- --window-perf-smoke` on 2026-06-25 presented 192 live glyph frames with a 2548x1568 glyph frame, 74 glyph quads, 73920 atlas bytes, and 17 occupied atlas slots | Proven in current live run |
| Prepared native glyph-frame preview artifact | `--runtime-glyph-frame-snapshot <path>` writes a deterministic PPM from the same owned glyph-frame preparation path used before surface presentation | Proven as CPU-side preview, not a live desktop screenshot |
| Native-window glyph-frame snapshot artifact | `cargo run -- --window-glyph-frame-snapshot target/gromaq-window-glyph-frame-current.ppm` on 2026-06-25 wrote a 2548x1568 PPM with 11985809 bytes, 60 glyph quads, and 1 cursor quad from the prepared native-window frame path | Proven as native presentation-path artifact, not an OS compositor screenshot |
| Generated logo window icon | Native app config embeds `images/logos/logo-icon-128.rgba` and sets it on `winit` `WindowAttributes`; app config tests assert the icon is present before native window creation | Proven for the cross-platform `winit` window-icon boundary |
| Linux desktop identity assets | `packaging/linux/dev.gromaq.Gromaq.desktop` uses `Icon=dev.gromaq.Gromaq`, AppStream metainfo declares the same desktop id, and `scripts/install.sh` installs both plus the generated hicolor icon under user-local XDG data paths by default | Proven by repository policy and shell syntax checks; live desktop-menu refresh not yet proven |
| macOS app bundle identity assets | `GROMAQ_BINARY_PATH=target/debug/gromaq scripts/package-macos-app.sh` on 2026-06-25 created `target/dist/Gromaq.app`; `plutil -lint` accepted `Info.plist`, `CFBundleIconFile` resolved to `AppIcon`, `CFBundleIdentifier` resolved to `dev.gromaq.Gromaq`, and `file` identified `Contents/Resources/AppIcon.icns` as a Mac OS X icon | Proven for bundle generation and icon metadata on current host; live packaged app launch/Dock proof not yet proven |
| Active-monitor frame pacing | `cargo run -- --window-perf-smoke` on 2026-06-25 collected 180 post-warmup samples on a 120000 mHz monitor, reported `frame interval target limited by monitor: true`, measured exact p95 8845125 ns against the 10000000 ns active-monitor budget, recorded 0 dropped frames, and reported `frame pacing accepted: true` | Proven for current 120Hz active monitor |
| 144Hz display pacing | Requires live proof on 144Hz-capable hardware | Not yet proven |
| Desktop screenshot of windowed glyph output | Requires live screenshot proof | Not yet proven |
| Desktop OS paste menu | Requires native desktop workflow proof | Not yet proven |
| macOS signed/notarized app distribution | Requires release signing and notarization proof | Not yet proven |

## Remaining Matrix Work

- Run the current-host scripted workflows on a broader host matrix.
- Add live screenshot artifacts for the native window path.
- Add 144Hz hardware proof on a 144Hz-capable monitor.
- Expand editor/multiplexer interaction beyond the current scripted edit,
  alternate-screen, search, and mouse workflows.
- Expand `ssh` and `kubectl` beyond current safe local client/version commands
  into real but safe target scenarios.
