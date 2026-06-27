# Release and Distribution

This document tracks how Gromaq is installed and packaged today, which release
artifacts are proven, and which distribution surfaces still need live platform
proof.

## User Install

The public one-command install path builds from source with Cargo:

```bash
curl -fsSL https://raw.githubusercontent.com/vicotrbb/gromaq/main/scripts/install.sh | sh
```

On macOS, the same installer can also package and copy a user-local `.app`
bundle with the project icon:

```bash
curl -fsSL https://raw.githubusercontent.com/vicotrbb/gromaq/main/scripts/install.sh | GROMAQ_INSTALL_APP_BUNDLE=1 sh
```

The default destination is `~/Applications/Gromaq.app`. Override it with
`GROMAQ_MACOS_APP_DIR=/path/to/apps`.

Requirements:

- Rust stable with Cargo
- macOS or Linux
- GPU/windowing support suitable for `winit` and `wgpu`

The installer intentionally does not install Rust or system packages. If Cargo
is absent, it exits with a clear error.
Preview installer actions without installing or writing files:

```bash
curl -fsSL https://raw.githubusercontent.com/vicotrbb/gromaq/main/scripts/install.sh | GROMAQ_DRY_RUN=1 sh
```

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

## Debian Package

Build a Debian package from the current checkout:

```bash
scripts/package-debian-deb.sh
```

The `.deb` installs:

- `/usr/bin/gromaq`
- `/usr/share/doc/gromaq/README.md`
- `/usr/share/doc/gromaq/copyright`
- Linux desktop file
- AppStream metainfo
- hicolor app icon

The script does not require `dpkg-deb`; it writes the Debian ar/tar members
directly so package assembly is testable on normal Unix CI hosts. Use
`GROMAQ_BINARY_PATH=<path>` to package an already-built binary and
`GROMAQ_DEB_ARCH=<arch>` to override the detected Debian architecture.

## macOS App Bundle

Build a local `.app` bundle:

```bash
scripts/package-macos-app.sh
open target/dist/Gromaq.app
```

The script builds a release binary when needed, creates `AppIcon.icns` from the
generated logo assets, writes `CFBundleIconFile`, and derives bundle version
metadata from `Cargo.toml`.

Use `GROMAQ_BINARY_PATH=<path>` to package an already-built binary. Set
`GROMAQ_CODESIGN_IDENTITY=-` for local ad-hoc signing, or set
`GROMAQ_CODESIGN_IDENTITY` to a Developer ID Application identity for release
signing. Developer ID signing uses hardened runtime and timestamp options.
After signing, `scripts/notarize-macos-app.sh <path-to-app>` creates a notary
zip, submits it with `xcrun notarytool submit --wait`, staples the accepted
ticket, validates stapling, and runs `spctl` assessment when available. Set
`GROMAQ_NOTARY_KEYCHAIN_PROFILE`, or provide
`GROMAQ_NOTARY_APPLE_ID`, `GROMAQ_NOTARY_PASSWORD`, and
`GROMAQ_NOTARY_TEAM_ID`.

When `GROMAQ_INSTALL_APP_BUNDLE=1` is set, `scripts/install.sh` prepares the
minimal packaging assets needed by `scripts/package-macos-app.sh`, runs it
against the installed binary, and copies the generated bundle to
`${GROMAQ_MACOS_APP_DIR:-~/Applications}`.

## GitHub Artifacts

`.github/workflows/release.yml` runs on `v*` tags and manual dispatch. It
uploads workflow artifacts for both trigger types:

- a Linux tarball from `scripts/package-linux-tarball.sh`
- a Debian package from `scripts/package-debian-deb.sh`
- a zipped macOS `.app` bundle from `scripts/package-macos-app.sh`

On tag-triggered runs, the workflow also creates or reuses the matching GitHub
Release and uploads the Linux tarball, Debian package, macOS `.app` zip, and
platform-specific checksum manifests as release assets. The checksum files are
copied to `SHA256SUMS-linux-x86_64` and `SHA256SUMS-macos-app` before release
upload so the Linux and macOS manifests do not collide as GitHub Release asset
names.

`.github/workflows/ci.yml` also has a focused `linux-packaging` job that runs
repository policy checks, Linux user-local desktop asset install proof, and
Linux tarball plus Debian package assembly on `ubuntu-latest`.
Release jobs also run `scripts/generate-checksums.sh` and upload `SHA256SUMS`
next to each artifact set.

## Current Proof Boundary

Proven remotely:

- GitHub Actions release workflow success: manual `workflow_dispatch` run
  `28298839954` for `Release Artifacts` completed successfully on 2026-06-27.
  The `linux-tarball` job ran project policy, packaged the Linux tarball and
  Debian package, generated checksums, and uploaded artifacts. The downloaded
  `gromaq-linux-tarball` artifact contains `gromaq-0.1.0-linux-x86_64.tar.gz`,
  `gromaq_0.1.0_amd64.deb`, and `SHA256SUMS`; `ar -t` on the downloaded `.deb`
  listed `debian-binary`, `control.tar.gz`, and `data.tar.gz`. The `macos-app`
  job ran project policy, packaged and zipped `Gromaq.app`, generated
  checksums, and uploaded artifacts.
- The tag-triggered GitHub Release publication path is configured in
  `.github/workflows/release.yml` and guarded by
  `tests/project_policy/ci.rs::release_workflow_publishes_tag_assets_to_github_releases`,
  which checks the required `gh release create`, `gh release upload`, token
  permission, tag-only condition, and unique checksum manifest markers.
- GitHub Actions CI run `28299568944` completed successfully on 2026-06-27 for
  commit `12f7dfe`. The `linux-packaging` job built the Linux tarball and
  Debian package, generated checksums, and proved Linux install-root desktop
  asset placement. The macOS `rust` job passed `cargo test --all`, including
  the packaging test that inspects the Debian package member structure.

Proven locally:

- macOS `.app` generation with a supplied debug binary
- `Info.plist` syntax and icon metadata
- optional macOS app-bundle install path with file-backed raw assets and a
  supplied installed binary
- local ad-hoc codesigning of the macOS app bundle with
  `GROMAQ_CODESIGN_IDENTITY=-` and strict `codesign --verify`
- notarization helper dry-run with `GROMAQ_NOTARY_DRY_RUN=1`, which creates the
  notary zip and prints the planned notarytool, stapler, and validation steps
- packaged macOS app executable launch via
  `target/dist/Gromaq.app/Contents/MacOS/gromaq --window-smoke`
- Linux install-root desktop asset placement without network or home writes
- Linux and macOS installer dry-run planning without Cargo, network, home, or
  install-root/app-directory writes
- CI Linux install-root desktop asset proof command in the `linux-packaging` job
- Linux tarball archive structure with a supplied binary
- Debian `.deb` archive structure with a supplied binary, canonical
  `debian-binary`, `control.tar.gz`, and `data.tar.gz` members, control
  metadata, `/usr/bin/gromaq`, desktop file, AppStream metainfo, icon, README,
  and copyright payloads
- release checksum manifest generation for local tarball, Debian package, and
  macOS zip artifacts
- shell syntax checks for install and packaging scripts
- project policy tests covering required release files and workflow markers

Not yet proven:

- live tag-triggered GitHub Release asset publication
- Developer ID signed and notarized macOS app distribution
- live Linux desktop menu refresh
- live macOS Dock behavior from a launched packaged app
