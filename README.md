# Gromaq

![Gromaq logo](images/logos/logo-on-graphite.png)

Gromaq is a native Rust GPU terminal emulator for `gromaq.dev`.

The project is intentionally native: Rust, `winit`, `wgpu`, real PTYs, and no
Electron, webview, React, or browser UI runtime. It is currently in an alpha
foundation stage. The core terminal state, PTY boundary, theme system, font
rasterization, GPU presentation path, performance smokes, and compatibility
tests are under active development, but broader daily-driver proof across
machines and workflows is still in progress.

## Install

The current installer builds from source with Cargo. It works on normal macOS
and Linux development machines that already have Rust stable installed.

```bash
curl -fsSL https://raw.githubusercontent.com/vicotrbb/gromaq/main/scripts/install.sh | sh
```

Manual install:

```bash
git clone https://github.com/vicotrbb/gromaq.git
cd gromaq
cargo install --path . --locked
```

Run:

```bash
gromaq
```

Requirements:

- Rust stable with Cargo
- macOS or a Linux desktop session with GPU drivers available to `wgpu`
- A login shell such as `zsh`, `bash`, or another configured shell

The one-command installer is deliberately small and auditable. It does not
install Rust or system packages for you; if Cargo is missing, install Rust from
your package manager or `https://rustup.rs`, then run the command again.

On Linux, the installer also installs user-local desktop assets by default:
`dev.gromaq.Gromaq.desktop`, the project icon under the hicolor icon theme, and
AppStream metainfo under `${XDG_DATA_HOME:-~/.local/share}`. Set
`GROMAQ_INSTALL_DESKTOP_ASSETS=0` to install only the binary.

On macOS, source install gives you the `gromaq` binary. To build a `.app` bundle
with the project logo as the Dock/app icon from a checked-out repository, run:

```bash
scripts/package-macos-app.sh
open target/dist/Gromaq.app
```

Release artifact helpers:

```bash
scripts/package-linux-tarball.sh
scripts/package-macos-app.sh
```

Tagged releases and manual workflow runs use `.github/workflows/release.yml` to
upload a Linux tarball and a zipped macOS `.app` bundle.

## Status

Implemented and covered by automated tests or deterministic smoke commands:

- terminal grid/state, scrollback, resize reflow, alternate screen, selection,
  clipboard boundaries, OSC title/label/8/52 handling, and terminal-generated
  responses
- broad ANSI/VT parsing coverage including SGR colors and attributes, DEC modes,
  cursor movement, tab stops, editing commands, mouse reporting, focus reports,
  bracketed paste, and Unicode wide/emoji cluster handling
- native PTY runtime with shell startup, input/output pump, resize propagation,
  large-output handling, command-output redraw proof, and external-tool workflow
  smoke coverage for available `ssh` and `kubectl`
- native `winit` app lifecycle, keyboard/mouse mapping, clipboard paste/copy,
  scrollback navigation, live config reload, text zoom, frame scheduling, FPS
  overlay, startup welcome screen, and generated logo window icon
- Swash-backed font rasterization, glyph atlas packing/cache, `wgpu` adapter and
  device bootstrap, offscreen GPU smokes, and presentable window-surface glyph
  frame path
- theme presets, opacity controls, deterministic theme snapshots, and default
  theme legibility gates
- Criterion benchmark harness and repository policy tests for native-only Rust,
  public metadata, docs, CI commands, and module-size discipline

Not yet proven enough to call complete:

- live desktop OS paste-menu workflow
- hardware-backed 144 Hz frame pacing proof on a 144 Hz-capable display
- live desktop screenshot proof across supported platforms
- wider compatibility matrix coverage across shells, editors, multiplexers,
  pagers, remote workflows, and multiple hosts
- release packaging beyond source install
- signed/notarized macOS release artifacts and package-manager-specific Linux
  packages

Current proof details live in
[`documentation/compatibility.md`](documentation/compatibility.md).

## Quick Start

Developer run:

```bash
cargo run
```

Useful smoke commands:

```bash
cargo run -- --gpu-info
cargo run -- --gpu-smoke
cargo run -- --gpu-terminal-text-smoke
cargo run -- --runtime-real-shell-smoke
cargo run -- --runtime-real-shell-command-output-smoke
cargo run -- --runtime-tool-workflow-smoke
cargo run -- --runtime-text-zoom-smoke
cargo run -- --theme-legibility-smoke
cargo run -- --theme-preview-snapshot target/gromaq-theme-preview.ppm
cargo run -- --theme-preview-config path/to/gromaq.toml target/gromaq-theme-preview.ppm
cargo run -- --welcome-preview-snapshot target/gromaq-welcome-preview.ppm
```

Full local verification:

```bash
cargo fmt --check
git diff --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
cargo bench --bench parser_throughput -- --list
```

Run `cargo bench` when changing parser, PTY pump, render planning, glyph cache,
rasterization, frame preparation, or other measured hot paths.

## Configuration

Generate a full starter config:

```bash
gromaq --config-template > gromaq.toml
gromaq --config-check gromaq.toml
gromaq --config gromaq.toml
```

Example:

```toml
[terminal]
cols = 132
rows = 40
scrollback_lines = 4096

[shell]
program = "/bin/zsh"
args = ["-l"]
cwd = "/tmp"

[welcome]
enabled = true

[font]
family = "JetBrains Mono Nerd Font"
# fallback_families = ["Apple Color Emoji"]
size_px = 32.0
# cell_width_px = 18
line_height_px = 44.0

[theme]
# presets: gromaq-dark, gromaq-graphite, gromaq-ghostty
preset = "gromaq-ghostty"
background = "#101216"
foreground = "#eef4fb"
cursor = "#f6c177"
selection = "#2f3b52"
background_opacity = 1.0
cursor_opacity = 1.0
selection_opacity = 1.0
cursor_style = "block"
cursor_blinking = true
ansi = [
  "#242933", "#ff6b7a", "#9ece6a", "#e0af68",
  "#7aa2f7", "#bb9af7", "#7dcfff", "#c8d3e5",
  "#5f667a", "#ff8fa3", "#b9f27c", "#ffd98a",
  "#9dbdff", "#d7afff", "#9ee7ff", "#f7fbff",
]
surface_padding_px = 14
cell_spacing_px = 0
dim_opacity = 0.68

[performance]
target_fps = 144
dirty_region_rendering = true
```

`gromaq-ghostty` is the built-in default theme preset, with calm contrast and
expressive ANSI colors inspired by polished Ghostty setups. `gromaq-dark` keeps
the original polished dark palette, and `gromaq-graphite` is an alternate
high-contrast graphite preset.

Use these commands to inspect and export themes:

```bash
cargo run -- --theme-list
cargo run -- --theme-export gromaq-ghostty target/gromaq-theme.toml
```

A preset provides the baseline background, foreground, cursor, selection, ANSI
palette, cursor style, cursor blinking, background/cursor/selection opacity,
surface padding, optional cell spacing, and dim text opacity; every field in
`[theme]` can still be overridden directly in TOML. Use
`gromaq --theme-preview-config <config> <path>` to render a deterministic
preview snapshot from any TOML config, including background, cursor, and
selection opacity, before adopting it.

`[welcome].enabled = true` shows the built-in startup screen with sectioned
system, terminal, renderer, and theme stats before the shell prompt. Set it to
`false` for a blank shell-first startup. The native frame status text, such as
FPS, is rendered as a presentation overlay and is not written into shell output
or scrollback. The overlay only draws into blank cells so right-aligned shell
prompts are not overwritten.

`font.family = "JetBrains Mono Nerd Font"` is the default preference. The
special value `"monospace"` remains an automatic mono-stack alias: polished
user-installed terminal fonts such as JetBrains Mono Nerd Font, MesloLGS Nerd
Font, Cascadia Mono, Iosevka Term, Geist Mono, Monaspace Neon, Fira Code, and
Hack are preferred when present, then the app falls back to SF Mono, Menlo, and
common Linux mono fonts. Explicit `.ttf`, `.ttc`, and `.otf` file paths are also
supported. `font.fallback_families = [...]` can add ordered fallback font names
or explicit font file paths before the automatic symbol and emoji fallback
stack.

Terminal text can be zoomed at runtime with browser-style shortcuts:
Control/Super `+`, Control/Super `-`, Control/Super `0`, and Control/Super
mouse wheel. Dedicated OS/browser `ZoomIn` and `ZoomOut` keys are also handled
when the platform exposes them.

More theme details are in [`documentation/theme.md`](documentation/theme.md).

## Documentation

- [`documentation/architecture.md`](documentation/architecture.md): module
  boundaries, organization rules, and native-app architecture
- [`documentation/benchmarks.md`](documentation/benchmarks.md): benchmark names,
  reproducible runs, and regression handling
- [`documentation/compatibility.md`](documentation/compatibility.md): current
  compatibility proof and gaps
- [`documentation/theme.md`](documentation/theme.md): theme, font, opacity, and
  welcome-screen contract
- [`TESTING.md`](TESTING.md): fixture conventions and live-proof boundaries
- [`DEBUGGING.md`](DEBUGGING.md): failure investigation workflow
- [`ROADMAP.md`](ROADMAP.md): open work toward daily-driver quality

The repository keeps one documentation tree under `documentation/` for project
docs that do not belong at the root.

Source logo/avatar images and generated terminal, preview, and window-icon
assets live under [`images/`](images/). The native app currently embeds
`images/logos/logo-icon-128.rgba` as the `winit` window icon.

## Contributing

Read [`CONTRIBUTING.md`](CONTRIBUTING.md) before opening a pull request.

Important project rules:

- native Rust only
- no `unsafe` in the crate
- no Electron, webview, React, or JavaScript frontend runtime
- Clippy warnings are failures
- behavior changes need tests and, where relevant, benchmark or smoke evidence
- unproven compatibility or performance claims must be documented as unproven

## License

Gromaq is licensed under the MIT License. See [`LICENSE`](LICENSE).
