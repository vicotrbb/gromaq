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

file_size() {
  wc -c < "$1" | tr -d ' '
}

append_ar_member() {
  archive="$1"
  member="$2"
  path="$3"
  size="$(file_size "${path}")"

  printf '%-16s%-12s%-6s%-6s%-8s%-10s`\n' \
    "${member}" \
    0 \
    0 \
    0 \
    100644 \
    "${size}" >> "${archive}"
  cat "${path}" >> "${archive}"

  if [ $((size % 2)) -ne 0 ]; then
    printf '\n' >> "${archive}"
  fi
}

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

for maintainer_script in postinst postrm; do
  cat > "${control_dir}/${maintainer_script}" <<'EOF'
#!/bin/sh
set -e

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database /usr/share/applications >/dev/null 2>&1 || true
fi

if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -q -t -f /usr/share/icons/hicolor >/dev/null 2>&1 || true
fi

exit 0
EOF
  chmod 755 "${control_dir}/${maintainer_script}"
done

printf '2.0\n' > "${staging_dir}/debian-binary"
tar -C "${control_dir}" -czf "${staging_dir}/control.tar.gz" ./control ./postinst ./postrm
tar -C "${data_dir}" -czf "${staging_dir}/data.tar.gz" ./usr

archive_tmp="${staging_dir}/package.ar"
printf '!<arch>\n' > "${archive_tmp}"
append_ar_member "${archive_tmp}" debian-binary "${staging_dir}/debian-binary"
append_ar_member "${archive_tmp}" control.tar.gz "${staging_dir}/control.tar.gz"
append_ar_member "${archive_tmp}" data.tar.gz "${staging_dir}/data.tar.gz"
mv "${archive_tmp}" "${package_path}"

printf '%s\n' "Packaged ${package_path}"
