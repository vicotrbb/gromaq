#!/bin/sh
set -eu

root="$(CDPATH= cd "$(dirname "$0")/.." && pwd)"
package="${GROMAQ_PACKAGE:-gromaq}"
version="${GROMAQ_VERSION:-v0.2.0}"
version_without_prefix="${version#v}"
proof_root="${GROMAQ_RELEASE_PROOF_ROOT:-${root}/target/github-release-install-proof}"
release_base="${GROMAQ_RELEASE_BASE:-https://github.com/vicotrbb/gromaq/releases/download/${version}}"
summary_path="${proof_root}/summary.txt"

if [ "$(uname -s)" != "Linux" ]; then
  printf '%s\n' "error: GitHub release install proof must run on Linux." >&2
  exit 1
fi

if ! command -v gh >/dev/null 2>&1; then
  printf '%s\n' "error: gh is required to verify published GitHub Release assets." >&2
  exit 1
fi

release_arch="$(uname -m)"
case "${release_arch}" in
  amd64)
    release_arch="x86_64"
    ;;
  arm64)
    release_arch="aarch64"
    ;;
esac

deb_arch="$(uname -m)"
case "${deb_arch}" in
  x86_64 | amd64)
    deb_arch="amd64"
    ;;
  aarch64 | arm64)
    deb_arch="arm64"
    ;;
esac

release_assets="$(
  gh release view "${version}" \
    --repo vicotrbb/gromaq \
    --json assets \
    --jq '.assets[].name'
)"

verify_release_asset() {
  asset="$1"
  if ! printf '%s\n' "${release_assets}" | grep -Fx "${asset}" >/dev/null 2>&1; then
    printf '%s\n' "error: published release asset missing: ${asset}" >&2
    exit 1
  fi
}

verify_release_asset "gromaq-${version_without_prefix}-linux-${release_arch}.tar.gz"
verify_release_asset "gromaq_${version_without_prefix}_${deb_arch}.deb"
verify_release_asset "PKGBUILD"
verify_release_asset "default.SRCINFO"
verify_release_asset "gromaq.install"
verify_release_asset "Gromaq-macos-app.zip"
verify_release_asset "SHA256SUMS-linux-${release_arch}"
verify_release_asset "SHA256SUMS-macos-app"

rm -rf "${proof_root}"
GROMAQ_PLATFORM=Linux \
  GROMAQ_INSTALL_METHOD=release \
  GROMAQ_VERSION="${version}" \
  GROMAQ_RELEASE_BASE="${release_base}" \
  GROMAQ_VERIFY_CHECKSUMS=1 \
  GROMAQ_BIN_DIR="${proof_root}/bin" \
  GROMAQ_INSTALL_ROOT="${proof_root}" \
  "${root}/scripts/install.sh"

test -x "${proof_root}/bin/${package}"
"${proof_root}/bin/${package}" --version
test -f "${proof_root}/share/applications/dev.gromaq.Gromaq.desktop"
test -f "${proof_root}/share/metainfo/dev.gromaq.Gromaq.metainfo.xml"
test -f "${proof_root}/share/icons/hicolor/256x256/apps/dev.gromaq.Gromaq.png"

{
  printf '%s\n' "GitHub release install proof: ok"
  printf '%s\n' "Version: ${version}"
  printf '%s\n' "Release base: ${release_base}"
  printf '%s\n' "Verified release assets:"
  printf '%s\n' "  gromaq-${version_without_prefix}-linux-${release_arch}.tar.gz"
  printf '%s\n' "  gromaq_${version_without_prefix}_${deb_arch}.deb"
  printf '%s\n' "  PKGBUILD"
  printf '%s\n' "  default.SRCINFO"
  printf '%s\n' "  gromaq.install"
  printf '%s\n' "  Gromaq-macos-app.zip"
  printf '%s\n' "  SHA256SUMS-linux-${release_arch}"
  printf '%s\n' "  SHA256SUMS-macos-app"
  printf '%s\n' "Proof root: ${proof_root}"
  printf '%s\n' "Installed binary: ${proof_root}/bin/${package}"
  printf '%s\n' "Desktop file: ${proof_root}/share/applications/dev.gromaq.Gromaq.desktop"
  printf '%s\n' "AppStream metainfo: ${proof_root}/share/metainfo/dev.gromaq.Gromaq.metainfo.xml"
  printf '%s\n' "Icon file: ${proof_root}/share/icons/hicolor/256x256/apps/dev.gromaq.Gromaq.png"
} | tee "${summary_path}"
