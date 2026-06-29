# Changelog

## 0.2.0

Public alpha/beta terminal foundation release for early adopters on supported
macOS and Linux systems.

Highlights:

- Native Rust terminal foundation with `winit`, `wgpu`, real PTYs, parser,
  scrollback, selection/copy, clipboard boundaries, themes, font rasterization,
  GPU smokes, and runtime proof helpers.
- GitHub Release packaging for Linux tarballs, Debian packages, Arch source
  metadata, macOS app zips, and SHA256 checksum manifests.
- Source installer, Linux release installer, macOS app-bundle installer path,
  Debian package path, and Arch metadata path documented for public use.
- Repository policy tests now protect the concise public README, proof-boundary
  docs, release scripts, packaging workflows, and native-only Rust direction.

Install options:

- Source installer:
  `curl -fsSL https://raw.githubusercontent.com/vicotrbb/gromaq/main/scripts/install.sh | sh`
- Linux release installer:
  `curl -fsSL https://raw.githubusercontent.com/vicotrbb/gromaq/main/scripts/install.sh | GROMAQ_INSTALL_METHOD=release GROMAQ_VERSION=v0.2.0 sh`
- Manual source install:
  `git clone https://github.com/vicotrbb/gromaq.git && cd gromaq && cargo install --path . --locked`
- Debian: `gromaq_0.2.0_amd64.deb`.
- macOS app artifact: `Gromaq-macos-app.zip`.
- Arch: release `PKGBUILD`, `default.SRCINFO`, and `gromaq.install`.

Known limitations:

- Not a v1.0 daily-driver stability release.
- Accepted live desktop screenshot proof is still missing.
- Hardware-backed 144 Hz proof still requires a 144 Hz-capable display.
- macOS Developer ID signing and notarization are not completed.
- Broader host compatibility, live desktop menu, and OS paste-menu workflows
  need more proof.

## 0.1.0

Initial public release artifact proof. The release published Linux tarball,
Debian package, Arch metadata, macOS app zip, and checksum assets, and is kept
as historical packaging evidence in the release documentation.
