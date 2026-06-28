# Gromaq Terminal — Codex Goal Prompt

## Goal

Build `gromaq`, a native Rust terminal emulator for `gromaq.dev`.

The objective is to implement a production-quality, open-source, GPU-rendered, performance-first terminal emulator that is correct, fast, stable, maintainable, and ready for real daily usage.

The initial goal is not AI features. AI-native capabilities may be added later. The current goal is to build the best possible terminal foundation.

## Master Rules

These rules are non-negotiable:

- No Electron.
- No webview.
- No React.
- No browser-based UI.
- No JavaScript/TypeScript frontend runtime.
- Native Rust only for the application core and UI.
- GPU rendering is mandatory.
- Performance-first architecture is mandatory.
- 144Hz-capable rendering is mandatory.
- AI features must not be implemented until the terminal core is stable, benchmarked, and correct.
- Public release quality is mandatory: README, install path, branding, and
  documentation organization are part of the product, not afterthoughts.
- Do not add dependencies casually.
- Do not optimize by sacrificing correctness.
- Do not implement large untested code paths.
- Do not mark work as complete without tests, validation, and benchmarks.
- Do not hide failing tests, flaky tests, warnings, panics, or benchmark regressions.
- Do not rely on subjective “it feels fast” claims. Measure everything.

## Required Development Loop

Work in a continuous loop until the terminal is complete:

1. Implement
2. Test
3. Validate
4. Optimize / Improve
5. Repeat

Every iteration must produce concrete progress and must leave the repository in a better state than before.

## Step 1 — Implement

Implement the next smallest high-value piece of the terminal.

Prefer vertical slices over huge rewrites.

Architecture should include:

- PTY engine
- shell launcher
- VT/ANSI parser
- terminal grid/state engine
- scrollback storage
- input handling
- keyboard/mouse event mapping
- GPU renderer
- glyph atlas
- frame scheduler
- dirty-region tracking
- configuration system
- theme engine with first-class defaults and user overrides
- test/control API for debug builds
- benchmark harness

Recommended Rust ecosystem:

- `winit` for windowing and input
- `wgpu` for GPU rendering
- `portable-pty` or platform PTY abstraction
- `vte` for terminal escape parsing
- `cosmic-text`, `swash`, or equivalent for text shaping/rasterization
- `tokio`, `crossbeam`, or standard threads where appropriate
- `criterion` for benchmarks
- `proptest` for property tests where useful
- `tracing` for structured diagnostics

## Step 2 — Test

For every implemented feature, add tests.

Required test types:

- unit tests
- integration tests
- golden terminal-state tests
- parser tests
- PTY lifecycle tests
- resize/reflow tests
- scrollback tests
- keyboard input tests
- Unicode width tests
- ANSI/VT compatibility tests
- selection/copy tests
- theme and legibility tests
- snapshot tests where appropriate
- performance regression tests

Test against real workflows:

- `zsh`
- `bash`
- `fish` when available
- `vim`
- `nvim`
- `tmux`
- `less`
- `top` / `htop` / `btop` when available
- `ssh`
- `cargo test`
- `kubectl` output where available
- large log output
- fast continuous output

## Step 3 — Validate

Validation must compare actual behavior against expected terminal behavior.

Required validation methods:

- dump terminal grid state
- dump scrollback state
- compare golden fixtures
- compare screenshots where useful
- compare behavior against reference terminals where possible
- compare default visual output against curated reference-quality terminals such as Ghostty and well-configured iTerm2 setups
- run compatibility scenarios for vim, neovim, tmux, shells, and large-output programs
- verify resize behavior
- verify alternate screen behavior
- verify colors, cursor, attributes, and styles
- verify default text contrast, prompt legibility, cursor visibility, selection colors, padding, and window surface readability
- verify mouse reporting modes
- verify copy/paste and selection behavior

The debug/test build should expose a structured test API:

```rust
pub trait TerminalTestApi {
    fn send_keys(&mut self, keys: &[TestKey]);
    fn paste_text(&mut self, text: &str);
    fn resize(&mut self, cols: u16, rows: u16);
    fn dump_grid(&self) -> GridSnapshot;
    fn dump_scrollback(&self) -> ScrollbackSnapshot;
    fn dump_cursor(&self) -> CursorSnapshot;
    fn dump_perf_metrics(&self) -> PerfSnapshot;
    fn screenshot(&self) -> Screenshot;
}
```

Use this API for deterministic agent-driven validation.

## Step 4 — Optimize / Improve

Optimize only after correctness is protected by tests.

Optimization targets:

- p95 frame time under 6.94ms for 144Hz
- low input latency
- stable frame pacing
- minimal idle CPU
- minimal allocations in hot paths
- fast glyph cache hits
- efficient dirty-region rendering
- efficient scrollback memory use
- high-throughput PTY parsing
- no unnecessary full-screen redraws
- no blocking work on render/input hot paths

Measure:

- frame time
- dropped frames
- input-to-render latency
- PTY read throughput
- parser throughput
- render time
- glyph atlas hit rate
- CPU usage
- memory usage
- scroll throughput
- large-output throughput

## Performance Acceptance Criteria

The terminal is not complete until these are true:

- Sustains 144Hz rendering on supported hardware.
- p95 frame time is below 6.94ms during normal interaction.
- Input latency p95 is below 10ms.
- Idle CPU is near zero.
- Large output does not freeze the UI.
- Scrolling remains smooth with large scrollback.
- No unbounded memory growth during long sessions.
- Renderer uses dirty-region updates or equivalent optimization.
- Glyphs are cached efficiently.
- No avoidable allocations occur in hot paths.
- Benchmarks are documented and reproducible.

## Correctness Acceptance Criteria

The terminal is not complete until these are true:

- Works with `bash`, `zsh`, `fish`, and `nushell` where available.
- Works with `vim` and `neovim`.
- Works with `tmux`.
- Works with `ssh`.
- Handles alternate screen correctly.
- Handles resize/reflow correctly.
- Handles Unicode and wide characters correctly.
- Handles ANSI colors and text attributes correctly.
- Handles scrollback correctly.
- Handles selection and copy/paste correctly.
- Handles keyboard shortcuts predictably.
- Handles mouse interaction where supported.
- Has a documented compatibility matrix.

## Visual Experience and Theme Acceptance Criteria

The terminal is not complete until it looks and feels excellent out of the box.

The default experience must be:

- beautiful, modern, and coherent without user configuration
- strongly inspired by the polish of terminals such as Ghostty and carefully configured iTerm2 setups
- readable at normal laptop and desktop viewing distances
- high contrast without harshness
- comfortable for long daily sessions
- visually stable during shell prompts, command output, alternate screen apps, resizing, and high-throughput output
- free of muddy low-contrast foreground/background combinations
- free of cramped text, awkward padding, clipping, or visually noisy default colors
- professional enough for open-source screenshots, README demos, and daily use

The theme engine must provide configurable building blocks for:

- background, foreground, cursor, selection, ANSI, and bright ANSI colors
- surface padding and cell spacing where practical
- font family, font size, line height, and fallback font behavior
- cursor style and cursor color
- inactive/dim text treatment where supported
- named built-in themes, including one excellent default theme
- importable/exportable theme configuration in documented TOML

The visual system must be tested and validated with:

- deterministic config parsing tests for theme settings
- renderer or prepared-frame tests proving colors propagate into GPU draw data
- screenshot or pixel-level validation where useful
- manual visual smoke evidence for the default theme
- documentation that explains theme configuration clearly

## Rust Quality Standards

Follow high-quality Rust engineering standards:

- Use clear module boundaries.
- Prefer simple, explicit ownership models.
- Avoid unnecessary cloning.
- Avoid global mutable state.
- Avoid `unsafe` unless justified, documented, isolated, and tested.
- Treat Clippy warnings as failures.
- Treat Rust warnings as failures.
- Use `Result` and structured errors instead of panics.
- Use `tracing` for diagnostics.
- Keep hot paths allocation-aware.
- Add comments for non-obvious performance or terminal-emulation logic.
- Keep public APIs documented.
- Use meaningful names.
- Keep functions small enough to review.
- Prefer deterministic behavior.
- Preserve cross-platform design where practical.

Required commands must pass before completion:

```bash
scripts/prove-local-ci-parity.sh
cargo fmt --check
git diff --check
git diff --cached --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
cargo bench
```

When `scripts/prove-local-ci-parity.sh` passes, it writes
`target/local-ci-parity-proof/summary.txt` with proof artifact handles.

## Repository Quality

The repository must include:

- `README.md`
- `ARCHITECTURE.md`
- `CONTRIBUTING.md`
- `BENCHMARKS.md`
- `COMPATIBILITY.md`
- `ROADMAP.md`
- license file
- CI workflow
- benchmark instructions
- development setup instructions
- debugging instructions
- test fixture documentation

## Open Source Standards

The project should be easy for contributors to understand.

Maintain:

- an accurate, polished, open-source-grade `README.md` that is suitable for a
  public GitHub project and reflects the actual current feature set, proof
  boundaries, installation path, validation commands, screenshots/assets, and
  contribution workflow
- clear issue labels
- clear coding standards
- reproducible tests
- reproducible benchmarks
- minimal setup friction
- one-command installation for normal end users on macOS and Linux where
  practical, with documented manual fallback commands for contributors and
  unsupported environments
- documented architecture
- clear contribution path
- good first issue candidates
- no hidden proprietary assumptions
- one canonical documentation tree under `documentation/`; remove any stray
  `docs/` tree instead of maintaining parallel documentation locations

## Distribution and Branding Standards

The public project must be easy to install and visibly branded:

- provide a simple one-command install path for normal macOS and Linux users
- keep that one-command path easy to audit and safe to run from a public
  open-source repository
- document manual build/install fallback commands for contributors and
  unsupported environments
- ensure the project logo is wired into native application identity where the
  platform supports it, including window, app bundle, taskbar, app switcher, and
  dock metadata
- keep source logo/avatar assets and generated app/icon outputs organized in
  repository assets
- test or document the platform boundary for app-icon behavior instead of
  assuming it works everywhere
- keep installer scripts small, auditable, and free of hidden network or
  proprietary assumptions

## Optional Future Direction

After the terminal core is stable, AI-native features may be added later.

Future AI features must be isolated from the terminal hot path and must never compromise latency, correctness, or stability.

Potential future features:

- command explanation
- error diagnosis
- repo-aware suggestions
- shell history reasoning
- log summarization
- safe command generation
- native accelerated commands
- semantic command layer

Do not implement these until the core terminal is excellent.

## Definition of Done

The goal is complete only when:

- the terminal launches successfully
- the terminal can run real shells
- the terminal is GPU-rendered
- the default theme is beautiful, legible, and polished enough to use without configuration
- theme configuration exposes the expected user-facing building blocks
- the terminal sustains 144Hz on supported hardware
- benchmarks prove performance targets
- tests prove correctness
- compatibility matrix is documented
- CI passes
- clippy passes with warnings denied
- formatting passes
- architecture is documented
- README quality is suitable for public open-source release
- macOS and Linux install paths are documented and easy to run, including one
  simple command for normal users
- native app branding uses the project logo where supported by the platform
- repository documentation lives under the intended documentation locations
  without a stray parallel `docs/` tree
- no Electron/webview/React/browser UI exists anywhere in the project
- no major known correctness gaps remain undocumented
- no major known performance gaps remain undocumented
- the project is suitable for public open-source release

Continue implementing, testing, validating, optimizing, and repeating until this definition of done is satisfied.
