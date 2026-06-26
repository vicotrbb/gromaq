# Release and Distribution

This document tracks how Gromaq is installed and packaged today, and what is
still missing before release artifacts can be called complete.

## User Install

The public one-command install path builds from source with Cargo:

```bash
curl -fsSL https://raw.githubusercontent.com/vicotrbb/gromaq/main/scripts/install.sh | sh
```

Requirements:

- Rust stable with Cargo
- macOS or Linux
- GPU/windowing support suitable for `winit` and `wgpu`

The installer intentionally does not install Rust or system packages. If Cargo
is absent, it exits with a clear error.

## Linux Desktop Assets

On Linux, `scripts/install.sh` installs user-local desktop identity assets by
default:

- `share/applications/dev.gromaq.Gromaq.desktop`
- `share/icons/hicolor/256x256/apps/dev.gromaq.Gromaq.png`
- `share/metainfo/dev.gromaq.Gromaq.metainfo.xml`

Disable desktop asset installation with:

```bash
GROMAQ_INSTALL_DESKTOP_ASSETS=0 sh scripts/install.sh
```

Maintainer non-network placement proof:

```bash
GROMAQ_SKIP_CARGO_INSTALL=1 GROMAQ_PLATFORM=Linux \
  GROMAQ_ASSET_ROOT="$PWD" GROMAQ_INSTALL_ROOT=target/install-proof \
  sh scripts/install.sh
```

## Linux Tarball

Build a tarball from the current checkout:

```bash
scripts/package-linux-tarball.sh
```

The archive includes:

- `bin/gromaq`
- `README.md`
- `LICENSE`
- Linux desktop file
- AppStream metainfo
- hicolor app icon

Use `GROMAQ_BINARY_PATH=<path>` to package an already-built binary.

## macOS App Bundle

Build a local `.app` bundle:

```bash
scripts/package-macos-app.sh
open target/dist/Gromaq.app
```

The script builds a release binary when needed, creates `AppIcon.icns` from the
generated logo assets, writes `CFBundleIconFile`, and derives bundle version
metadata from `Cargo.toml`.

Use `GROMAQ_BINARY_PATH=<path>` to package an already-built binary.

## GitHub Artifacts

`.github/workflows/release.yml` runs on `v*` tags and manual dispatch. It
uploads:

- a Linux tarball from `scripts/package-linux-tarball.sh`
- a zipped macOS `.app` bundle from `scripts/package-macos-app.sh`

`.github/workflows/ci.yml` also has a focused `linux-packaging` job that runs
repository policy checks, Linux user-local desktop asset install proof, and
Linux tarball assembly on `ubuntu-latest`.
Release jobs also run `scripts/generate-checksums.sh` and upload `SHA256SUMS`
next to each artifact set.

## Current Proof Boundary

Proven locally:

- macOS `.app` generation with a supplied debug binary
- `Info.plist` syntax and icon metadata
- Linux install-root desktop asset placement without network or home writes
- CI Linux install-root desktop asset proof command in the `linux-packaging` job
- Linux tarball archive structure with a supplied binary
- release checksum manifest generation for local tarball artifacts
- shell syntax checks for install and packaging scripts
- project policy tests covering required release files and workflow markers

Not yet proven:

- remote GitHub Actions release workflow success
- signed and notarized macOS app distribution
- package-manager-specific Linux packages
- live Linux desktop menu refresh
- live macOS Dock behavior from a launched packaged app
