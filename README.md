# Gromaq

![Gromaq logo](images/logos/logo-on-graphite.png)

Gromaq is a native Rust GPU-rendered terminal emulator built for performance,
correctness, and a polished developer experience.

Launch site: `https://vicotrbb.github.io/gromaq/`

The project is intentionally native: Rust, `winit`, `wgpu`, real PTYs, and no
Electron, webview, React, or browser UI runtime.

![Gromaq welcome screen preview](images/screenshots/gromaq-welcome-preview.png)

## Status

Gromaq `0.2.1` is a public alpha/beta terminal foundation release. It is
installable and usable by early adopters on supported macOS and Linux systems,
but it is not a v1.0 daily-driver stability claim.

Use it if you want to try the native terminal foundation, verify the packaging
work, or contribute focused compatibility and performance proof. Do not read
this release as proof of broad host compatibility, Developer ID notarized macOS
distribution, accepted live desktop screenshot evidence, or 144 Hz hardware
acceptance.

Detailed proof ledgers live in
[`documentation/release.md`](documentation/release.md),
[`documentation/compatibility.md`](documentation/compatibility.md), and
[`documentation/benchmarks.md`](documentation/benchmarks.md).

## Install

macOS release installer, using the `v0.2.1` GitHub Release app bundle and
checksum manifest. This path does not require Rust:

```bash
curl -fsSL https://raw.githubusercontent.com/vicotrbb/gromaq/main/scripts/install.sh | GROMAQ_INSTALL_METHOD=release GROMAQ_VERSION=v0.2.1 sh
```

Linux release installer, using the `v0.2.1` GitHub Release tarball and checksum
manifest:

```bash
curl -fsSL https://raw.githubusercontent.com/vicotrbb/gromaq/main/scripts/install.sh | GROMAQ_INSTALL_METHOD=release GROMAQ_VERSION=v0.2.1 sh
```

Source installer, for macOS and Linux contributors with Rust stable installed:

```bash
curl -fsSL https://raw.githubusercontent.com/vicotrbb/gromaq/main/scripts/install.sh | sh
```

Manual source install:

```bash
git clone https://github.com/vicotrbb/gromaq.git
cd gromaq
cargo install --path . --locked
```

Run:

```bash
gromaq
gromaq --help
gromaq --version
```

Preview installer actions without writing files:

```bash
curl -fsSL https://raw.githubusercontent.com/vicotrbb/gromaq/main/scripts/install.sh | GROMAQ_DRY_RUN=1 sh
```

### Packages

- Linux tarball: `gromaq-0.2.1-linux-x86_64.tar.gz`, verified through
  `SHA256SUMS-linux-x86_64`.
- Debian: `gromaq_0.2.1_amd64.deb`.
- Arch: release assets include `PKGBUILD`, `default.SRCINFO`, and
  `gromaq.install` for source-package workflows.
- macOS universal app bundle: `Gromaq-macos-app.zip`, verified through
  `SHA256SUMS-macos-app`.

macOS release install copies `Gromaq.app` to `~/Applications` by default.
Set `GROMAQ_MACOS_APP_DIR=/path/to/apps` to choose another destination. The
current public macOS app artifact is ad-hoc signed and not Developer ID
notarized unless a future release note says otherwise.

macOS source install gives contributors the `gromaq` binary. To build and copy
a user-local `.app` bundle from source on macOS:

```bash
curl -fsSL https://raw.githubusercontent.com/vicotrbb/gromaq/main/scripts/install.sh | GROMAQ_INSTALL_APP_BUNDLE=1 sh
```

This source bundle path requires Rust and Cargo.

## Requirements

- Rust stable with Cargo for source installs only.
- macOS or Linux with GPU/windowing support available to `winit` and `wgpu`.
- A configured shell such as `zsh`, `bash`, or another login shell.

The installer does not install Rust or system packages. If Cargo is missing,
use the release installer or install Rust from your package manager or
`https://rustup.rs` before running a source install.

## What Works Today

- Terminal grid/state, scrollback, resize reflow, alternate screen,
  selection/copy, clipboard boundaries, OSC title/label/8/52 handling, and
  terminal-generated responses.
- ANSI/VT parsing coverage for SGR colors and attributes, DEC modes, cursor
  movement, tab stops, editing commands, mouse reporting, focus reports,
  bracketed paste, and Unicode wide/emoji cluster handling.
- Native PTY runtime with shell startup, input/output pump, resize propagation,
  command-output redraw proof, large-output handling, and scripted external
  tool workflows where host binaries are present.
- Native `winit` app lifecycle, keyboard/mouse mapping, clipboard paste/copy,
  scrollback navigation, live config reload, text zoom, frame scheduling, FPS
  overlay, startup welcome screen, and generated logo window icon.
- Swash-backed font rasterization, glyph atlas packing/cache, `wgpu` adapter
  bootstrap, offscreen GPU smokes, and presentable window-surface glyph-frame
  path.
- Theme presets, opacity controls, deterministic theme snapshots, default
  theme legibility gates, and welcome-preview freshness proof.
- Release automation for Linux tarballs, Debian packages, Arch metadata, macOS
  app zips, and SHA256 checksum manifests.
- Native tmux assist foundations: `gromaq --tmux-assist`, `--tmux-action`,
  and `--tmux-manager` expose registry-backed tmux command/keybinding metadata.
  The no-argument native app opens the manager on startup for status,
  navigation, start/attach, split/window/rename, confirmed kill actions,
  snapshot refresh, workspace preset launch, and mouse clicks select visible manager rows.
  Control/Super Shift `T` toggles the same manager after launch;
  `--runtime-tmux-smoke` and
  `--runtime-tmux-ui-smoke` prove isolated tmux paths when tmux is installed.

## Known Proof Gaps

- Accepted live desktop screenshot proof for the default native window.
- Hardware-backed 144 Hz frame pacing proof on a 144 Hz-capable display.
- Wider compatibility matrix coverage across shells, editors, multiplexers,
  pagers, remote workflows, and multiple hosts.
- Developer ID signed and notarized macOS distribution.
- Live desktop menu, Dock/Finder, Linux menu UI, and OS paste-menu workflows.
- Manual app-window proof for the full native tmux manager workflow.

These gaps are tracked as proof boundaries, not hidden failures. The current
compatibility matrix is in
[`documentation/compatibility.md`](documentation/compatibility.md).

## Verification And Development

Run the local CI parity bundle from the repository root:

```bash
scripts/prove-local-ci-parity.sh
```

For focused checks:

```bash
cargo fmt --check
git diff --check
git diff --cached --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
cargo test --test project_policy
cargo bench --bench parser_throughput -- --list
```

`scripts/prove-local-ci-parity.sh` runs shell syntax checks, formatting,
staged and unstaged whitespace checks, Clippy, the full test suite,
Avatar asset freshness, README screenshot freshness, current-host compatibility proof,
theme/welcome proof helpers, runtime smoke commands, GPU smoke commands, and
the parser benchmark inventory. Run full `cargo bench` when changing measured
hot paths such as parser, PTY pump, render planning, glyph cache, rasterization,
or frame preparation.

## Configuration

Generate and validate a starter config:

```bash
gromaq --config-template > gromaq.toml
gromaq --config-check gromaq.toml
gromaq --config gromaq.toml
```

Useful theme commands:

```bash
gromaq --theme-list
gromaq --theme-export gromaq-ghostty target/gromaq-theme.toml
gromaq --theme-preview-config gromaq.toml target/gromaq-theme-preview.ppm
```

More theme, font, opacity, and welcome-screen details are in [`documentation/theme.md`](documentation/theme.md).

tmux assist and workspace-preset foundations are native Rust commands:

```bash
gromaq --tmux-assist
gromaq --tmux-manager
gromaq --tmux-action split-pane-right
gromaq --tmux-action kill-session my-session --confirm
gromaq --runtime-tmux-smoke
gromaq --runtime-tmux-ui-smoke
gromaq --window-tmux-manager-snapshot target/native-tmux-manager.ppm
scripts/prove-native-tmux-default-snapshot.sh
```

Inside the native window, the tmux manager opens on startup when tmux UI is enabled and `open_manager_on_start` is true; `show_status_strip = false` hides only the strip. Control/Super Shift `T` toggles the manager. Use arrows or `h`/`j`/`k`/`l` to move; Enter runs actions or starts name entry; `r` refreshes the tmux snapshot without shell input; destructive actions require `y`; Esc closes or cancels prompts. `scripts/prove-native-tmux-default-snapshot.sh` creates an isolated tmux session, records the rendered status-strip pane command plus manager session/window/pane counts, and writes PPM/PNG artifacts under `target/native-tmux-default-snapshot-proof`; it proves the native rendered frame path, not live keyboard or mouse interaction.

## Documentation

- [`documentation/architecture.md`](documentation/architecture.md): module
  boundaries and native-app architecture.
- [`documentation/release.md`](documentation/release.md): install, packaging,
  release procedure, and proof-boundary workflow.
- [`documentation/compatibility.md`](documentation/compatibility.md): current
  compatibility proof and gaps.
- [`documentation/benchmarks.md`](documentation/benchmarks.md): benchmark names,
  reproducible runs, and performance boundaries.
- [`documentation/theme.md`](documentation/theme.md): themes, fonts, opacity,
  and welcome-screen contract.
- [`TESTING.md`](TESTING.md): fixture conventions and local proof commands.
- [`DEBUGGING.md`](DEBUGGING.md): failure investigation workflow.
- [`ROADMAP.md`](ROADMAP.md): open work toward daily-driver quality.
- [`SECURITY.md`](SECURITY.md): vulnerability reporting scope and private
  disclosure path.
- [`CONTRIBUTING.md`](CONTRIBUTING.md): contribution standards and pull request
  expectations.

The repository keeps one canonical project documentation tree under
`documentation/`.

## Contributing

Read [`CONTRIBUTING.md`](CONTRIBUTING.md) before opening a pull request.

Important project rules:

- Native Rust only.
- No `unsafe` in crate roots.
- No Electron, webview, React, or JavaScript frontend runtime.
- Clippy warnings are failures.
- Behavior changes need tests and, where relevant, benchmark or smoke evidence.
- Unproven compatibility or performance claims must stay documented as
  unproven.

## License

Gromaq is licensed under the MIT License. See [`LICENSE`](LICENSE).
