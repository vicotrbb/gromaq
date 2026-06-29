#!/bin/sh
set -eu

root="$(CDPATH= cd "$(dirname "$0")/.." && pwd)"
package="${GROMAQ_PACKAGE:-gromaq}"
version="${GROMAQ_VERSION:-v0.2.1}"
dist_dir="${GROMAQ_DIST_DIR:-${root}/target/dist}"
proof_root="${GROMAQ_RELEASE_PROOF_ROOT:-${root}/target/release-install-proof}"
summary_path="${proof_root}/summary.txt"

if [ "$(uname -s)" != "Linux" ]; then
  printf '%s\n' "error: Linux release install proof must run on Linux." >&2
  exit 1
fi

"${root}/scripts/package-linux-tarball.sh"
GROMAQ_CHECKSUM_MANIFEST="${dist_dir}/SHA256SUMS" "${root}/scripts/generate-checksums.sh"
cp "${dist_dir}/SHA256SUMS" "${dist_dir}/SHA256SUMS-$(uname -s | tr '[:upper:]' '[:lower:]')-$(uname -m | sed 's/^amd64$/x86_64/;s/^arm64$/aarch64/')"

rm -rf "${proof_root}"
GROMAQ_PLATFORM=Linux \
  GROMAQ_INSTALL_METHOD=release \
  GROMAQ_VERSION="${version}" \
  GROMAQ_RELEASE_BASE="file://${dist_dir}" \
  GROMAQ_BIN_DIR="${proof_root}/bin" \
  GROMAQ_INSTALL_ROOT="${proof_root}" \
  "${root}/scripts/install.sh"

test -x "${proof_root}/bin/${package}"
"${proof_root}/bin/${package}" --version
test -f "${proof_root}/share/applications/dev.gromaq.Gromaq.desktop"
test -f "${proof_root}/share/metainfo/dev.gromaq.Gromaq.metainfo.xml"
test -f "${proof_root}/share/icons/hicolor/256x256/apps/dev.gromaq.Gromaq.png"

{
  printf '%s\n' "Linux release install proof: ok"
  printf '%s\n' "Proof root: ${proof_root}"
  printf '%s\n' "Installed binary: ${proof_root}/bin/${package}"
  printf '%s\n' "Checksum manifest: ${dist_dir}/SHA256SUMS"
  printf '%s\n' "Platform checksum manifest: ${dist_dir}/SHA256SUMS-$(uname -s | tr '[:upper:]' '[:lower:]')-$(uname -m | sed 's/^amd64$/x86_64/;s/^arm64$/aarch64/')"
  printf '%s\n' "Desktop file: ${proof_root}/share/applications/dev.gromaq.Gromaq.desktop"
  printf '%s\n' "AppStream metainfo: ${proof_root}/share/metainfo/dev.gromaq.Gromaq.metainfo.xml"
  printf '%s\n' "Icon file: ${proof_root}/share/icons/hicolor/256x256/apps/dev.gromaq.Gromaq.png"
} | tee "${summary_path}"
