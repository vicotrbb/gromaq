#!/bin/sh
set -eu
LC_ALL=C
export LC_ALL

root="$(CDPATH= cd "$(dirname "$0")/.." && pwd)"
package="${GROMAQ_PACKAGE:-gromaq}"
version="$(
  sed -n 's/^version = "\(.*\)"/\1/p' "${root}/Cargo.toml" | head -n 1
)"
target_name="${GROMAQ_RELEASE_TARGET:-$(uname -s | tr '[:upper:]' '[:lower:]')-$(uname -m)}"
binary_path="${GROMAQ_BINARY_PATH:-${root}/target/release/${package}}"
dist_dir="${GROMAQ_DIST_DIR:-${root}/target/dist}"
staging_dir="${dist_dir}/${package}-${version}-${target_name}"
archive_path="${dist_dir}/${package}-${version}-${target_name}.tar.gz"
summary_path="${dist_dir}/${package}-linux-tarball-summary.txt"

if [ ! -x "${binary_path}" ]; then
  cargo build --release
fi

rm -rf "${staging_dir}" "${archive_path}"
mkdir -p \
  "${staging_dir}/bin" \
  "${staging_dir}/share/applications" \
  "${staging_dir}/share/icons/hicolor/256x256/apps" \
  "${staging_dir}/share/metainfo"

cp "${binary_path}" "${staging_dir}/bin/${package}"
chmod 755 "${staging_dir}/bin/${package}"
cp "${root}/README.md" "${staging_dir}/README.md"
cp "${root}/LICENSE" "${staging_dir}/LICENSE"
cp "${root}/packaging/linux/dev.gromaq.Gromaq.desktop" \
  "${staging_dir}/share/applications/dev.gromaq.Gromaq.desktop"
cp "${root}/packaging/linux/dev.gromaq.Gromaq.metainfo.xml" \
  "${staging_dir}/share/metainfo/dev.gromaq.Gromaq.metainfo.xml"
cp "${root}/images/logos/logo-icon-256.png" \
  "${staging_dir}/share/icons/hicolor/256x256/apps/dev.gromaq.Gromaq.png"

tar -C "${dist_dir}" -czf "${archive_path}" "$(basename "${staging_dir}")"
payload_files="$(find "${staging_dir}" -type f | wc -l | tr -d ' ')"

{
  printf '%s\n' "Linux tarball package: ok"
  printf '%s\n' "Archive: ${archive_path}"
  printf '%s\n' "Staging dir: ${staging_dir}"
  printf '%s\n' "Payload files: ${payload_files}"
  printf '%s\n' "Binary: ${staging_dir}/bin/${package}"
  printf '%s\n' "Desktop file: ${staging_dir}/share/applications/dev.gromaq.Gromaq.desktop"
  printf '%s\n' "AppStream metainfo: ${staging_dir}/share/metainfo/dev.gromaq.Gromaq.metainfo.xml"
  printf '%s\n' "Icon file: ${staging_dir}/share/icons/hicolor/256x256/apps/dev.gromaq.Gromaq.png"
} | tee "${summary_path}"
