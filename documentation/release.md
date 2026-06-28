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

When `update-desktop-database` is available, the installer refreshes the Linux
desktop database for `share/applications` and reports the refreshed directory.
The deterministic installer test uses a fake command on `PATH` to prove the
hook is invoked without requiring a live Linux desktop session.

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
scripts/prove-linux-release-install.sh
scripts/prove-github-release-install.sh
scripts/prove-linux-desktop-discovery.sh
```

The archive includes:

- `bin/gromaq`
- `README.md`
- `LICENSE`
- Linux desktop file
- AppStream metainfo
- hicolor app icon

Use `GROMAQ_BINARY_PATH=<path>` to package an already-built binary.
The packager writes `target/dist/gromaq-linux-tarball-summary.txt` after
archive assembly succeeds.
On Linux hosts, `scripts/prove-linux-release-install.sh` packages the tarball,
generates checksums, installs through `GROMAQ_INSTALL_METHOD=release` from a
local `file://` release base into `target/release-install-proof`, and verifies
the installed binary plus desktop identity payloads without writing to the
user's home directory. The helper writes `summary.txt` in the proof root after
the install and payload checks pass.
After a tagged GitHub Release publishes the Linux tarball and
`SHA256SUMS-linux-<arch>` assets, Linux maintainers can run
`scripts/prove-github-release-install.sh` to install from the real GitHub
Release URL into `target/github-release-install-proof` with checksum
verification enabled and a `summary.txt` success handle. That helper is present
for the live download proof, but the proof is still unrun until release assets
exist.
On Linux desktop hosts, `scripts/prove-linux-desktop-discovery.sh` installs the
desktop identity payloads into `target/linux-desktop-discovery-proof` by
default, requires `desktop-file-validate`, `appstreamcli`,
`update-desktop-database`, and `gtk-update-icon-cache`, validates the desktop
file and AppStream metadata, refreshes the proof-root desktop database and
hicolor icon cache, writes `summary.txt`, and records that this metadata/cache
proof does not prove live menu UI rendering.

## Debian Package

Build a Debian package from the current checkout:

```bash
scripts/package-debian-deb.sh
scripts/prove-debian-package.sh
```

The `.deb` installs:

- `/usr/bin/gromaq`
- `/usr/share/doc/gromaq/README.md`
- `/usr/share/doc/gromaq/copyright`
- Linux desktop file
- AppStream metainfo
- hicolor app icon
- Debian `postinst` and `postrm` maintainer scripts that refresh the desktop
  database and hicolor icon cache when the relevant desktop utilities are
  available

The script does not require `dpkg-deb`; it writes the Debian ar/tar members
directly so package assembly is testable on normal Unix CI hosts. Use
`GROMAQ_BINARY_PATH=<path>` to package an already-built binary and
`GROMAQ_DEB_ARCH=<arch>` to override the detected Debian architecture.
On Debian/Ubuntu hosts, `scripts/prove-debian-package.sh` builds the package,
installs it with `dpkg -i`, runs `/usr/bin/gromaq --version`, and checks the
installed binary, README, copyright, desktop file, AppStream metainfo, and
hicolor icon payloads. The helper writes
`target/dist/gromaq-debian-proof-summary.txt` with the installed payload entry
count after those checks pass.

## Arch Package Recipe

`packaging/arch/PKGBUILD`, `packaging/arch/.SRCINFO`, and
`packaging/arch/gromaq.install` provide an Arch `makepkg` source-package recipe:

```bash
bash -n packaging/arch/PKGBUILD
sh -n packaging/arch/gromaq.install
scripts/prove-arch-package.sh
```

The recipe builds from the public Git repository with
`cargo build --release --locked` and installs:

- `/usr/bin/gromaq`
- README documentation
- MIT license file
- Linux desktop file
- AppStream metainfo
- hicolor app icon
- Arch package install/upgrade/remove hooks that refresh the desktop database
  and hicolor icon cache when the relevant desktop utilities are available

CI and repository policy syntax-check the recipe and assert the expected desktop
identity payload markers. The local CI workflow now includes an
`arch-packaging` job under `archlinux:base-devel` that installs `git` and
`rust`, switches to an unprivileged builder user, and runs
`makepkg --nobuild --noconfirm` plus `makepkg --printsrcinfo` from
`packaging/arch`; it also checks that `packaging/arch/.SRCINFO` is present.
On 2026-06-28 UTC, GitHub Actions CI run `28308158338` completed green for
commit `5d204f2`; its `arch-packaging` job continued through full
`makepkg --noconfirm`, `pacman -U` package installation,
`/usr/bin/gromaq --version`, and `pacman -Ql gromaq-git` payload checks for the
binary, README, license, desktop file, AppStream metainfo, and hicolor icon.
GitHub Actions CI runs `28301610408` and `28302521484` proved the earlier
metadata-only job remotely.
Maintainers with a working Docker daemon can run
`scripts/prove-arch-package.sh` to perform the same full package build,
`pacman -U` install, `/usr/bin/gromaq --version`, and installed-payload checks
inside an `archlinux:base-devel` container. The helper writes
`target/arch-package-proof-summary.txt` with the installed payload entry count
after success.

## macOS App Bundle

Build a local `.app` bundle:

```bash
scripts/package-macos-app.sh
open target/dist/Gromaq.app
```

The script builds a release binary when needed, creates `AppIcon.icns` from the
generated logo assets, writes `CFBundleIconFile` and
`LSApplicationCategoryType=public.app-category.utilities`, and derives bundle
version metadata from `Cargo.toml`. It writes
`target/dist/Gromaq-macos-app-summary.txt` after successful packaging with the
bundle path, identifier, executable, plist, icon, and signing identity when one
was supplied.

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
- the Arch `packaging/arch/PKGBUILD`, `packaging/arch/.SRCINFO`, and
  `packaging/arch/gromaq.install` source-package metadata and install hook
- a zipped macOS `.app` bundle from `scripts/package-macos-app.sh`

On tag-triggered runs, the workflow also creates or reuses the matching GitHub
Release and uploads the Linux tarball, Debian package, Arch `PKGBUILD`,
`.SRCINFO`, `gromaq.install`, macOS `.app` zip, and platform-specific checksum
manifests as release assets. The checksum files are copied to
`SHA256SUMS-linux-x86_64` and
`SHA256SUMS-macos-app` before release upload so the Linux and macOS manifests
do not collide as GitHub Release asset names.

`.github/workflows/ci.yml` also has a focused `linux-packaging` job that runs
repository policy checks, installs `desktop-file-utils`, `appstream`, and
`gtk-update-icon-cache`, runs Linux user-local desktop asset install proof, runs
the Linux desktop metadata/cache discovery helper, and runs Linux tarball plus
Debian package assembly on `ubuntu-latest`. The job is also configured for
Debian package install, `gromaq --version`, and installed-payload checks.
It then runs `scripts/prove-linux-release-install.sh` with Arch metadata
checksum extras, which packages the local tarball, writes
`SHA256SUMS-linux-x86_64` with the generated checksum entry count, installs
through `GROMAQ_INSTALL_METHOD=release`, and checks the installed binary plus
desktop identity payloads under
`target/release-install-proof`. The macOS `rust` job is configured to run
`scripts/prove-current-host-compatibility.sh` and upload
`target/compatibility-proof/*`, including `summary.txt` with present/missing
tool counts and runtime tool workflow checked/passed/skipped/failed counts, as
the `gromaq-current-host-compatibility-proof` artifact. CI also has a Linux
compatibility job that installs common Ubuntu shell/editor/TUI tools, runs the
same helper, sets `GROMAQ_REQUIRED_COMPAT_TOOLS` so expected installed tools
fail closed, and uploads `gromaq-linux-compatibility-proof`. CI run
`28314822034` passed and uploaded both compatibility artifacts before the
overall run failed later in the macOS welcome-preview threshold step.
CI also runs `bash -n packaging/arch/PKGBUILD`.
The macOS `rust` job is configured to run `scripts/prove-theme-preview.sh`,
`scripts/prove-welcome-preview.sh`, and
`scripts/prove-readme-welcome-preview.sh`. It uploads
`target/theme-preview-proof/*` as `gromaq-theme-preview-proof`, then uploads
both `target/welcome-preview-proof/*` and
`target/readme-welcome-preview-proof/*` as `gromaq-welcome-preview-proof` so
theme, prepared welcome preview, and README screenshot freshness proof artifacts
are retained together after the visual proof steps run. The theme helper writes
a compact `summary.txt` artifact. CI run `28315944025` passed the default
welcome proof on macOS 26 with 126062 avatar color pixels and uploaded compact
welcome diagnostics, but the README freshness proof failed because exact
decoded pixels differed from the committed local PNG. CI run `28316513803`
then proved the bounded-delta README freshness helper and README freshness
`summary.txt` artifact remotely. CI run `28321082609` completed green for
commit `84740e3` after rerunning avatar freshness, default welcome proof,
README screenshot freshness proof, and the full packaging/compatibility CI job
set for the refreshed 33x17 block avatar. CI run `28326188288` completed green
for commit `0dfed64` after rerunning the same helper set for the current 33x17
Braille avatar; the macOS welcome proof accepted 20509 high-contrast text
pixels, 25966 avatar color pixels, 654 glyph quads, 0 cursor quads, 576576
atlas bytes, and `Metric: avatar rows=17`. The welcome and theme preview
artifact uploads use `if: always()` so diagnostic visual artifacts survive
proof-command or later macOS job failures when files were written before the
failure.
Release jobs also run `scripts/generate-checksums.sh`, report the checksum
entry count, and upload `SHA256SUMS` next to each artifact set. The Linux
packaging and release jobs run checksum generation with
`GROMAQ_CHECKSUM_EXTRA_FILES="packaging/arch/PKGBUILD packaging/arch/.SRCINFO packaging/arch/gromaq.install"`,
so the uploaded Linux checksum manifest covers the Arch source-package recipe
metadata, `.SRCINFO`, and install hook as well as the Linux tarball and Debian
package.

## Current Proof Boundary

Proven remotely:

- GitHub Actions CI run `28321082609` completed green for commit `84740e3` on
  2026-06-28 UTC. The macOS `rust` job passed formatting, whitespace, clippy,
  `cargo test --all`, avatar asset freshness, theme proof, current-host
  compatibility proof, welcome preview proof, README screenshot freshness
  proof, the runtime/theme/GPU smoke suite, and
  `cargo bench --bench parser_throughput -- --list`. The `linux-packaging`
  job passed Debian package install and payload checks plus the helper-backed
  Linux release-install proof with Arch metadata checksum extras. The
  `linux-compatibility` job passed the required-tool compatibility helper, and
  `arch-packaging` passed full `makepkg --noconfirm`, package install,
  `/usr/bin/gromaq --version`, payload listing, and `.SRCINFO` generation.
- GitHub Actions CI run `28326188288` completed green for commit `0dfed64` on
  2026-06-28 UTC. The macOS `rust` job passed formatting, whitespace, clippy,
  `cargo test --all`, avatar asset freshness, theme proof, current-host
  compatibility proof, welcome preview proof, README screenshot freshness
  proof, every runtime/theme/GPU smoke, and
  `cargo bench --bench parser_throughput -- --list`. The `linux-packaging`
  job passed Debian package install and payload checks plus the helper-backed
  Linux release-install proof with Arch metadata checksum extras. The
  `linux-compatibility` job passed the current-host compatibility helper, and
  `arch-packaging` passed full `makepkg --noconfirm`, package install,
  `/usr/bin/gromaq --version`, payload listing, and `.SRCINFO` generation.
- GitHub Actions CI run `28310371344` completed green for commit `3474653` on
  2026-06-28 UTC. The macOS `rust` job ran `sh -n
  scripts/prove-welcome-preview.sh`, ran `scripts/prove-welcome-preview.sh`,
  uploaded `target/welcome-preview-proof/*` as the
  `gromaq-welcome-preview-proof` artifact, and still passed formatting,
  whitespace, clippy, `cargo test --all`, the runtime/theme/GPU smoke suite,
  and `cargo bench --bench parser_throughput -- --list`. The downloaded proof
  artifact contained `gromaq-welcome-preview.ppm`, `welcome-preview.log`, and
  `gromaq-welcome-preview.png`. The `linux-packaging` job passed the
  helper-backed Linux release-install proof and the `arch-packaging` job passed
  full `makepkg --noconfirm`, package install, `/usr/bin/gromaq --version`, and
  installed-payload listing under `archlinux:base-devel`.
- GitHub Actions CI run `28309262840` completed green for commit `461006d` on
  2026-06-28 UTC. The `linux-packaging` job passed
  Debian package install and payload checks, then ran
  `GROMAQ_CHECKSUM_EXTRA_FILES="packaging/arch/PKGBUILD packaging/arch/.SRCINFO packaging/arch/gromaq.install" scripts/prove-linux-release-install.sh`
  and verified `target/release-install-proof/bin/gromaq` exists. The
  `arch-packaging` job passed full `makepkg --noconfirm`, `pacman -U`,
  `/usr/bin/gromaq --version`, `pacman -Ql gromaq-git`, and
  `makepkg --printsrcinfo`; the macOS `rust` job passed formatting,
  whitespace, clippy, `cargo test --all`, the runtime/theme/GPU smoke suite,
  and `cargo bench --bench parser_throughput -- --list`.
- GitHub Actions release workflow run `28303243197` completed successfully on
  2026-06-27 for commit `12a38e8`. The Linux workflow artifact contained
  `target/dist/gromaq-0.1.0-linux-x86_64.tar.gz`,
  `target/dist/gromaq_0.1.0_amd64.deb`, `target/dist/SHA256SUMS`,
  `packaging/arch/PKGBUILD`, and the hidden `packaging/arch/.SRCINFO`; tarball
  inspection confirmed `bin/gromaq`, README, license, desktop file, AppStream
  metainfo, and hicolor icon payloads, and `ar -t` on the Debian package listed
  `debian-binary`, `control.tar.gz`, and `data.tar.gz`. The Linux checksum
  manifest listed `.SRCINFO`, `PKGBUILD`, the tarball, and the Debian package.
  The macOS workflow artifact contained `Gromaq-macos-app.zip` and
  `SHA256SUMS`; the zipped app contains `MacOS/gromaq`, `Resources/AppIcon.icns`,
  `Info.plist`, and `PkgInfo`, and `Info.plist` includes
  `LSApplicationCategoryType=public.app-category.utilities`. The manual-dispatch
  publish steps were skipped as expected because the run was not tag-triggered.
- GitHub Actions CI run `28303175039` completed successfully on 2026-06-27 for
  commit `12a38e8`. The `arch-packaging` job passed
  `makepkg --nobuild --noconfirm` and `makepkg --printsrcinfo`, the
  `linux-packaging` job completed the Linux install-root, tarball, Debian,
  checksum-extra, and local release-method install proof, and the macOS `rust`
  job passed formatting, whitespace, clippy, `cargo test --all`, the
  runtime/theme/GPU smoke suite, and `cargo bench --bench parser_throughput -- --list`.
- GitHub Actions CI run `28308158338` completed successfully on 2026-06-28 UTC
  for commit `5d204f2`. The `arch-packaging` job ran in
  `archlinux:base-devel`, passed `makepkg --nobuild --noconfirm`, then passed
  full `makepkg --noconfirm`, `pacman -U --noconfirm`,
  `test -x /usr/bin/gromaq`, `/usr/bin/gromaq --version`,
  `pacman -Ql gromaq-git`, and `makepkg --printsrcinfo`, proving the Arch
  package build, install, executable launch metadata, and installed payload
  listing remotely.
- GitHub Actions release workflow run `28302556353` completed successfully on
  2026-06-27 for commit `c4bb4f1`. The downloaded Linux workflow artifact
  contained `target/dist/gromaq-0.1.0-linux-x86_64.tar.gz`,
  `target/dist/gromaq_0.1.0_amd64.deb`, `target/dist/SHA256SUMS`,
  `packaging/arch/PKGBUILD`, and the hidden `packaging/arch/.SRCINFO`, proving
  the `include-hidden-files: true` release-artifact upload path. The downloaded
  Linux checksum manifest listed `.SRCINFO`, `PKGBUILD`, the tarball, and the
  Debian package. The downloaded macOS workflow artifact contained
  `Gromaq-macos-app.zip` and `SHA256SUMS`; inspecting the zipped
  `Gromaq.app/Contents/Info.plist` proved
  `LSApplicationCategoryType=public.app-category.utilities`.
- GitHub Actions release workflow success: manual `workflow_dispatch` run
  `28298839954` for `Release Artifacts` completed successfully on 2026-06-27.
  The `linux-tarball` job ran project policy, packaged the Linux tarball and
  Debian package, generated checksums, and uploaded artifacts. The downloaded
  `gromaq-linux-tarball` artifact contains `gromaq-0.1.0-linux-x86_64.tar.gz`,
  `gromaq_0.1.0_amd64.deb`, and `SHA256SUMS`; `ar -t` on the downloaded `.deb`
  listed `debian-binary`, `control.tar.gz`, and `data.tar.gz`. The `macos-app`
  job ran project policy, packaged and zipped `Gromaq.app`, generated
  checksums, and uploaded artifacts.
- GitHub Actions release workflow run `28301764662` completed successfully on
  2026-06-27 for commit `c4feef2`. It proved the Linux and macOS release jobs
  still complete with the Arch metadata checksum path, and the downloaded Linux
  checksum manifest listed both `PKGBUILD` and `.SRCINFO`. Artifact inspection
  also exposed that `actions/upload-artifact` omitted hidden `.SRCINFO` from
  the workflow artifact unless `include-hidden-files: true` is set; later run
  `28302556353` proved that corrected hidden-file artifact upload.
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
- installed macOS app-bundle LaunchServices smoke from the configured
  `GROMAQ_MACOS_APP_DIR`, proven with `open -W -n ... --args --window-smoke`
- local ad-hoc codesigning of the macOS app bundle with
  `GROMAQ_CODESIGN_IDENTITY=-` and strict `codesign --verify`
- notarization helper dry-run with `GROMAQ_NOTARY_DRY_RUN=1`, which creates the
  notary zip and prints the planned notarytool, stapler, and validation steps
- packaged macOS app executable launch via
  `target/dist/Gromaq.app/Contents/MacOS/gromaq --window-smoke`
- refreshed macOS `.app` package summary generation via
  `GROMAQ_BINARY_PATH=target/debug/gromaq scripts/package-macos-app.sh`, which
  wrote `target/dist/Gromaq-macos-app-summary.txt`, preserved the
  `dev.gromaq.Gromaq` bundle id, `AppIcon` icon key, and utilities category,
  and produced a packaged executable that reports `gromaq 0.1.0`
- packaged macOS app LaunchServices launch via
  `open -W -n -o target/macos-open-proof.stdout --stderr target/macos-open-proof.stderr target/dist/Gromaq.app --args --window-smoke`,
  which returned 0 and captured `window smoke: ok`
- live packaged macOS app identity registration via
  `scripts/prove-macos-app-identity.sh`; while the app was running, System
  Events returned `gromaq` for bundle identifier `dev.gromaq.Gromaq`,
  `lsappinfo` reported `CFBundleIdentifier=dev.gromaq.Gromaq` and
  `LSDisplayName=Gromaq`, `pgrep` observed the bundled
  `Contents/MacOS/gromaq --window-screenshot-smoke` process, and the smoke
  completed with 900 presented frames. The helper writes `summary.txt` under
  `target/macos-app-identity-proof` after all checks pass. A later current-host
  rerun found the bundled process and LaunchServices identity but did not
  refresh that summary because `--window-screenshot-smoke` reported 3600 redraw
  attempts, 0 timeouts, and 3600 fully occluded surface acquisitions; the helper
  now reports that condition directly instead of collapsing it into a generic
  missing-success error.
- Linux install-root desktop asset placement without network or home writes
- Linux and macOS installer dry-run planning without Cargo, network, home, or
  install-root/app-directory writes
- Linux release-tarball installer path with
  `GROMAQ_INSTALL_METHOD=release`, `GROMAQ_RELEASE_BASE=file://...`, and
  `GROMAQ_BIN_DIR=<temp-bin>`, proven by
  `tests/install_dry_run.rs::install_script_installs_linux_release_tarball_from_local_base`
  against a locally generated tarball and matching checksum manifest without
  network or home writes
- live GitHub Release installer proof helper
  `scripts/prove-github-release-install.sh`, which is Linux-only and installs
  real GitHub Release assets into `target/github-release-install-proof` with
  checksum verification enabled and a `summary.txt` success handle once tagged
  release assets exist
- Linux desktop metadata/cache discovery proof helper
  `scripts/prove-linux-desktop-discovery.sh`, which is Linux-only and validates
  installed desktop identity metadata under an isolated proof root when the
  relevant desktop metadata tools are available. CI run `28314822034` passed it
  in the Ubuntu `linux-packaging` job after installing those tools; it still
  does not prove live menu UI rendering
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
  `makepkg --printsrcinfo`; repository policy also guards the full
  `makepkg --noconfirm`, `pacman -U`, `gromaq --version`, and installed payload
  check markers
- tag-triggered GitHub Release publication path is configured locally in
  `.github/workflows/release.yml` and guarded by
  `tests/project_policy/ci.rs::release_workflow_publishes_tag_assets_to_github_releases`,
  which checks the required `gh release create`, `gh release upload`, token
  permission, tag-only condition, Arch `PKGBUILD` plus `.SRCINFO` upload, and
  unique checksum manifest markers
- release checksum manifest generation for local tarball, Debian package,
  optional extra release assets such as the Arch `PKGBUILD` and `.SRCINFO`, and
  macOS zip artifacts
- release workflow hidden-file upload guard with `include-hidden-files: true`
  for the Linux artifact set that carries `packaging/arch/.SRCINFO`
- shell syntax checks for install and packaging scripts
- project policy tests covering required release files and workflow markers

Not yet proven:

- live tag-triggered GitHub Release asset publication
- live Linux release-method install from GitHub Release assets
- Developer ID signed and notarized macOS app distribution
- live Linux desktop menu UI discovery after install
- manual macOS Dock/Finder UI behavior from a launched packaged app
