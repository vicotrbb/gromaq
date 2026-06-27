#!/bin/sh
set -eu
LC_ALL=C
export LC_ALL

root="$(CDPATH= cd "$(dirname "$0")/.." && pwd)"
package="${GROMAQ_PACKAGE:-gromaq}"
version="$(
  sed -n 's/^version = "\(.*\)"/\1/p' "${root}/Cargo.toml" | head -n 1
)"
binary_path="${GROMAQ_BINARY_PATH:-${root}/target/release/${package}}"
dist_dir="${GROMAQ_DIST_DIR:-${root}/target/dist}"
deb_arch="${GROMAQ_DEB_ARCH:-}"

mkdir -p "${dist_dir}"
dist_dir="$(CDPATH= cd "${dist_dir}" && pwd)"

if [ -z "${deb_arch}" ]; then
  case "$(uname -m)" in
    x86_64 | amd64)
      deb_arch="amd64"
      ;;
    aarch64 | arm64)
      deb_arch="arm64"
      ;;
    *)
      deb_arch="$(uname -m)"
      ;;
  esac
fi

staging_dir="${dist_dir}/${package}_${version}_${deb_arch}"
control_dir="${staging_dir}/control"
data_dir="${staging_dir}/data"
package_path="${dist_dir}/${package}_${version}_${deb_arch}.deb"

if [ ! -x "${binary_path}" ]; then
  cargo build --release
fi

rm -rf "${staging_dir}" "${package_path}"
mkdir -p \
  "${control_dir}" \
  "${data_dir}/usr/bin" \
  "${data_dir}/usr/share/doc/${package}" \
  "${data_dir}/usr/share/applications" \
  "${data_dir}/usr/share/icons/hicolor/256x256/apps" \
  "${data_dir}/usr/share/metainfo"

cp "${binary_path}" "${data_dir}/usr/bin/${package}"
chmod 755 "${data_dir}/usr/bin/${package}"
cp "${root}/README.md" "${data_dir}/usr/share/doc/${package}/README.md"
cp "${root}/LICENSE" "${data_dir}/usr/share/doc/${package}/copyright"
cp "${root}/packaging/linux/dev.gromaq.Gromaq.desktop" \
  "${data_dir}/usr/share/applications/dev.gromaq.Gromaq.desktop"
cp "${root}/packaging/linux/dev.gromaq.Gromaq.metainfo.xml" \
  "${data_dir}/usr/share/metainfo/dev.gromaq.Gromaq.metainfo.xml"
cp "${root}/images/logos/logo-icon-256.png" \
  "${data_dir}/usr/share/icons/hicolor/256x256/apps/dev.gromaq.Gromaq.png"

cat > "${control_dir}/control" <<EOF
Package: ${package}
Version: ${version}
Section: utils
Priority: optional
Architecture: ${deb_arch}
Maintainer: Gromaq contributors
Homepage: https://gromaq.dev
Description: Native Rust GPU-rendered terminal emulator foundation for gromaq.dev
 Gromaq is a native Rust terminal emulator built with winit, wgpu, real PTYs,
 and a performance-first renderer.
EOF

printf '2.0\n' > "${staging_dir}/debian-binary"
tar -C "${control_dir}" -czf "${staging_dir}/control.tar.gz" ./control
tar -C "${data_dir}" -czf "${staging_dir}/data.tar.gz" ./usr

(
  cd "${staging_dir}"
  ar -qc "$(basename "${package_path}")" debian-binary control.tar.gz data.tar.gz
  mv "$(basename "${package_path}")" "${package_path}"
)

printf '%s\n' "Packaged ${package_path}"
