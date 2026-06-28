#!/bin/sh
set -eu

root="$(CDPATH= cd "$(dirname "$0")/.." && pwd)"
package="${GROMAQ_PACKAGE:-gromaq}"
dist_dir="${GROMAQ_DIST_DIR:-${root}/target/dist}"

if ! command -v dpkg >/dev/null 2>&1; then
  printf '%s\n' "error: dpkg is required for Debian package proof." >&2
  exit 1
fi

if ! command -v sudo >/dev/null 2>&1; then
  printf '%s\n' "error: sudo is required to install the Debian package proof." >&2
  exit 1
fi

"${root}/scripts/package-debian-deb.sh"

set -- "${dist_dir}"/"${package}"_*.deb
if [ ! -f "$1" ]; then
  printf '%s\n' "error: Debian package not found under ${dist_dir}." >&2
  exit 1
fi

sudo dpkg -i "$1"
test -x "/usr/bin/${package}"
"/usr/bin/${package}" --version

dpkg -L "${package}" | tee "${dist_dir}/${package}-debian-payload.txt"
grep -F "/usr/bin/${package}" "${dist_dir}/${package}-debian-payload.txt"
grep -F "/usr/share/doc/${package}/README.md" "${dist_dir}/${package}-debian-payload.txt"
grep -F "/usr/share/doc/${package}/copyright" "${dist_dir}/${package}-debian-payload.txt"
grep -F "/usr/share/applications/dev.gromaq.Gromaq.desktop" "${dist_dir}/${package}-debian-payload.txt"
grep -F "/usr/share/metainfo/dev.gromaq.Gromaq.metainfo.xml" "${dist_dir}/${package}-debian-payload.txt"
grep -F "/usr/share/icons/hicolor/256x256/apps/dev.gromaq.Gromaq.png" "${dist_dir}/${package}-debian-payload.txt"

printf '%s\n' "Debian package proof: ok"
