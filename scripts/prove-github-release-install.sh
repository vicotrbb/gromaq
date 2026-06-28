#!/bin/sh
set -eu

root="$(CDPATH= cd "$(dirname "$0")/.." && pwd)"
package="${GROMAQ_PACKAGE:-gromaq}"
version="${GROMAQ_VERSION:-v0.1.0}"
proof_root="${GROMAQ_RELEASE_PROOF_ROOT:-${root}/target/github-release-install-proof}"
release_base="${GROMAQ_RELEASE_BASE:-https://github.com/vicotrbb/gromaq/releases/download/${version}}"

if [ "$(uname -s)" != "Linux" ]; then
  printf '%s\n' "error: GitHub release install proof must run on Linux." >&2
  exit 1
fi

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

printf '%s\n' "GitHub release install proof: ok"
