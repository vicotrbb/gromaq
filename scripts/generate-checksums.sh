#!/bin/sh
set -eu
LC_ALL=C
export LC_ALL

root="$(CDPATH= cd "$(dirname "$0")/.." && pwd)"
dist_dir="${GROMAQ_DIST_DIR:-${root}/target/dist}"
manifest="${GROMAQ_CHECKSUM_MANIFEST:-${dist_dir}/SHA256SUMS}"
tmp_manifest="${manifest}.tmp"

checksum_command() {
  if command -v shasum >/dev/null 2>&1; then
    printf '%s\n' "shasum"
    return
  fi
  if command -v sha256sum >/dev/null 2>&1; then
    printf '%s\n' "sha256sum"
    return
  fi
  printf '%s\n' "error: shasum or sha256sum is required to write release checksums." >&2
  exit 1
}

hash_file() {
  tool="$1"
  path="$2"
  if [ "${tool}" = "shasum" ]; then
    line="$(shasum -a 256 "${path}")"
  else
    line="$(sha256sum "${path}")"
  fi
  printf '%s\n' "${line%% *}"
}

mkdir -p "${dist_dir}"
: > "${tmp_manifest}"
tool="$(checksum_command)"
found=0

for artifact in "${dist_dir}"/*.tar.gz "${dist_dir}"/*.deb "${dist_dir}"/*.zip; do
  [ -f "${artifact}" ] || continue
  checksum="$(hash_file "${tool}" "${artifact}")"
  printf '%s  %s\n' "${checksum}" "$(basename "${artifact}")" >> "${tmp_manifest}"
  found=1
done

if [ "${found}" -eq 0 ]; then
  rm -f "${tmp_manifest}"
  printf '%s\n' "error: no release archives found in ${dist_dir}." >&2
  exit 1
fi

sort "${tmp_manifest}" > "${manifest}"
rm -f "${tmp_manifest}"

printf '%s\n' "Wrote ${manifest}"
