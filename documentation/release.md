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

Linux users can opt into a prebuilt release tarball install after a tagged
release publishes GitHub Release assets:

```bash
curl -fsSL https://raw.githubusercontent.com/vicotrbb/gromaq/main/scripts/install.sh | GROMAQ_INSTALL_METHOD=release GROMAQ_VERSION=v0.1.0 sh
```

The release method downloads `gromaq-<version>-linux-<arch>.tar.gz` from
`${GROMAQ_REPO}/releases/download/${GROMAQ_VERSION}` by default, or from
`GROMAQ_RELEASE_BASE` when testing against a mirror or local `file://` release
directory. It installs the binary to
`${GROMAQ_BIN_DIR:-${CARGO_HOME:-~/.cargo}/bin}` and copies the Linux desktop
identity assets from the tarball itself. By default it also downloads the
matching `SHA256SUMS-linux-<arch>` manifest and verifies the tarball before
extraction. Set `GROMAQ_VERIFY_CHECKSUMS=0` only for local mirror/debug
scenarios where another integrity check is already in place; set
`GROMAQ_CHECKSUM_ASSET` when a mirror uses a different manifest filename.

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

## Arch Package Recipe

`packaging/arch/PKGBUILD` and `packaging/arch/.SRCINFO` provide an Arch
`makepkg` source-package recipe:

```bash
bash -n packaging/arch/PKGBUILD
```

The recipe builds from the public Git repository with
`cargo build --release --locked` and installs:

- `/usr/bin/gromaq`
- README documentation
- MIT license file
- Linux desktop file
- AppStream metainfo
- hicolor app icon

CI and repository policy syntax-check the recipe and assert the expected desktop
identity payload markers. The local CI workflow now includes an
`arch-packaging` job under `archlinux:base-devel` that installs `git` and
`rust`, switches to an unprivileged builder user, and runs
`makepkg --nobuild --noconfirm` plus `makepkg --printsrcinfo` from
`packaging/arch`; it also checks that `packaging/arch/.SRCINFO` is present.
That job awaits remote GitHub Actions proof after the next push. A full live
`makepkg` build and install on Arch Linux still requires separate platform
proof.

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
- the Arch `packaging/arch/PKGBUILD` and `packaging/arch/.SRCINFO`
  source-package metadata
- a zipped macOS `.app` bundle from `scripts/package-macos-app.sh`

On tag-triggered runs, the workflow also creates or reuses the matching GitHub
Release and uploads the Linux tarball, Debian package, Arch `PKGBUILD`,
`.SRCINFO`, macOS `.app` zip, and platform-specific checksum manifests as
release assets. The checksum files are copied to `SHA256SUMS-linux-x86_64` and
`SHA256SUMS-macos-app` before release upload so the Linux and macOS manifests
do not collide as GitHub Release asset names.

`.github/workflows/ci.yml` also has a focused `linux-packaging` job that runs
repository policy checks, Linux user-local desktop asset install proof, and
Linux tarball plus Debian package assembly on `ubuntu-latest`. The job is also
configured to copy `SHA256SUMS` to `SHA256SUMS-linux-x86_64` and install from
the generated local tarball through `GROMAQ_INSTALL_METHOD=release` before
checking that `target/release-install-proof/bin/gromaq` exists. CI also runs
`bash -n packaging/arch/PKGBUILD`.
Release jobs also run `scripts/generate-checksums.sh` and upload `SHA256SUMS`
next to each artifact set. The Linux packaging and release jobs run checksum
generation with
`GROMAQ_CHECKSUM_EXTRA_FILES="packaging/arch/PKGBUILD packaging/arch/.SRCINFO"`,
so the uploaded Linux checksum manifest covers the Arch source-package recipe
and `.SRCINFO` metadata as well as the Linux tarball and Debian package.

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
- GitHub Actions CI run `28300600507` completed successfully on 2026-06-27 for
  commit `93fcbef`. The `linux-packaging` job built the Linux tarball and
  Debian package, generated checksums, copied the checksum manifest to
  `SHA256SUMS-linux-x86_64`, installed from the generated local release tarball
  with `GROMAQ_INSTALL_METHOD=release`, and verified
  `target/release-install-proof/bin/gromaq` exists. The macOS `rust` job passed
  `cargo fmt --check`, `git diff --check`,
  `cargo clippy --all-targets --all-features -- -D warnings`,
  `cargo test --all`, the runtime/theme/GPU smoke suite, and
  `cargo bench --bench parser_throughput -- --list`.
- GitHub Actions CI run `28301610408` completed successfully on 2026-06-27 for
  commit `c4feef2`. The new `arch-packaging` job ran in
  `archlinux:base-devel`, installed `git` and `rust`, checked
  `packaging/arch/.SRCINFO`, ran `bash -n packaging/arch/PKGBUILD`, and passed
  both `makepkg --nobuild --noconfirm` and `makepkg --printsrcinfo` as an
  unprivileged builder user. The `linux-packaging` job also generated
  checksums with both Arch metadata paths in `GROMAQ_CHECKSUM_EXTRA_FILES` and
  completed the local release-method install proof, while the macOS `rust` job
  passed the full formatting, clippy, test, runtime/theme/GPU smoke, and
  benchmark-list suite.
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
- Linux release-tarball installer path with
  `GROMAQ_INSTALL_METHOD=release`, `GROMAQ_RELEASE_BASE=file://...`, and
  `GROMAQ_BIN_DIR=<temp-bin>`, proven by
  `tests/install_dry_run.rs::install_script_installs_linux_release_tarball_from_local_base`
  against a locally generated tarball and matching checksum manifest without
  network or home writes
- CI Linux install-root desktop asset proof command in the `linux-packaging` job
- Linux tarball archive structure with a supplied binary
- Debian `.deb` archive structure with a supplied binary, canonical
  `debian-binary`, `control.tar.gz`, and `data.tar.gz` members, control
  metadata, `/usr/bin/gromaq`, desktop file, AppStream metainfo, icon, README,
  and copyright payloads
- Arch `PKGBUILD` syntax plus `.SRCINFO` and payload-marker policy coverage
- Arch `PKGBUILD` plus `.SRCINFO` source-package metadata is configured for CI
  syntax checks and guarded by repository policy markers for the Cargo build
  command, desktop file, AppStream metainfo, hicolor icon payload, and source
  metadata
- Arch `arch-packaging` CI job policy coverage for `makepkg --nobuild` and
  `makepkg --printsrcinfo`
- tag-triggered GitHub Release publication path is configured locally in
  `.github/workflows/release.yml` and guarded by
  `tests/project_policy/ci.rs::release_workflow_publishes_tag_assets_to_github_releases`,
  which checks the required `gh release create`, `gh release upload`, token
  permission, tag-only condition, Arch `PKGBUILD` plus `.SRCINFO` upload, and
  unique checksum manifest markers
- release checksum manifest generation for local tarball, Debian package,
  optional extra release assets such as the Arch `PKGBUILD` and `.SRCINFO`, and
  macOS zip artifacts
- shell syntax checks for install and packaging scripts
- project policy tests covering required release files and workflow markers

Not yet proven:

- live tag-triggered GitHub Release asset publication
- live release workflow upload proof for the Arch `PKGBUILD` plus `.SRCINFO`
  artifacts
- live Linux release-method install from GitHub Release assets
- live Arch `makepkg` build/install
- Developer ID signed and notarized macOS app distribution
- live Linux desktop menu refresh
- live macOS Dock behavior from a launched packaged app
