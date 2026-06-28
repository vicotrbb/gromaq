#!/bin/sh
set -eu

root="$(CDPATH= cd "$(dirname "$0")/.." && pwd)"
app_name="${GROMAQ_APP_NAME:-Gromaq}"
bundle_id="${GROMAQ_BUNDLE_ID:-dev.gromaq.Gromaq}"
binary_name="${GROMAQ_PACKAGE:-gromaq}"
version="$(
  sed -n 's/^version = "\(.*\)"/\1/p' "${root}/Cargo.toml" | head -n 1
)"
binary_path="${GROMAQ_BINARY_PATH:-${root}/target/release/${binary_name}}"
dist_dir="${GROMAQ_DIST_DIR:-${root}/target/dist}"
codesign_identity="${GROMAQ_CODESIGN_IDENTITY:-}"
app_dir="${dist_dir}/${app_name}.app"
contents_dir="${app_dir}/Contents"
macos_dir="${contents_dir}/MacOS"
resources_dir="${contents_dir}/Resources"
iconset_dir="${dist_dir}/${app_name}.iconset"
summary_path="${dist_dir}/${app_name}-macos-app-summary.txt"

if [ "$(uname -s)" != "Darwin" ]; then
  printf '%s\n' "error: macOS app bundle packaging requires Darwin." >&2
  exit 1
fi

for tool in cargo iconutil sips; do
  if ! command -v "${tool}" >/dev/null 2>&1; then
    printf '%s\n' "error: ${tool} is required to package ${app_name}.app." >&2
    exit 1
  fi
done

if [ ! -x "${binary_path}" ]; then
  cargo build --release
fi

rm -rf "${app_dir}" "${iconset_dir}"
rm -f "${summary_path}"
mkdir -p "${macos_dir}" "${resources_dir}" "${iconset_dir}"

copy_icon() {
  size="$1"
  scale="$2"
  name="$3"
  source="${root}/images/logos/logo-transparent.png"
  if [ "${size}" = "128" ] && [ "${scale}" = "1" ]; then
    source="${root}/images/logos/logo-icon-128.png"
  elif [ "${size}" = "256" ] && [ "${scale}" = "1" ]; then
    source="${root}/images/logos/logo-icon-256.png"
  elif [ "${size}" = "512" ] && [ "${scale}" = "1" ]; then
    source="${root}/images/logos/logo-icon-512.png"
  fi
  sips -z "${size}" "${size}" "${source}" --out "${iconset_dir}/${name}" >/dev/null
}

copy_icon 16 1 icon_16x16.png
copy_icon 32 2 icon_16x16@2x.png
copy_icon 32 1 icon_32x32.png
copy_icon 64 2 icon_32x32@2x.png
copy_icon 128 1 icon_128x128.png
copy_icon 256 2 icon_128x128@2x.png
copy_icon 256 1 icon_256x256.png
copy_icon 512 2 icon_256x256@2x.png
copy_icon 512 1 icon_512x512.png
copy_icon 1024 2 icon_512x512@2x.png

iconutil -c icns "${iconset_dir}" -o "${resources_dir}/AppIcon.icns"
cp "${binary_path}" "${macos_dir}/${binary_name}"
chmod 755 "${macos_dir}/${binary_name}"

cat > "${contents_dir}/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key>
  <string>en</string>
  <key>CFBundleDisplayName</key>
  <string>${app_name}</string>
  <key>CFBundleExecutable</key>
  <string>${binary_name}</string>
  <key>CFBundleIconFile</key>
  <string>AppIcon</string>
  <key>CFBundleIdentifier</key>
  <string>${bundle_id}</string>
  <key>CFBundleName</key>
  <string>${app_name}</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleShortVersionString</key>
  <string>${version}</string>
  <key>CFBundleVersion</key>
  <string>${version}</string>
  <key>LSMinimumSystemVersion</key>
  <string>12.0</string>
  <key>LSApplicationCategoryType</key>
  <string>public.app-category.utilities</string>
  <key>NSHighResolutionCapable</key>
  <true/>
</dict>
</plist>
EOF

printf '%s' 'APPL????' > "${contents_dir}/PkgInfo"
rm -rf "${iconset_dir}"

if [ -n "${codesign_identity}" ]; then
  if ! command -v codesign >/dev/null 2>&1; then
    printf '%s\n' "error: codesign is required when GROMAQ_CODESIGN_IDENTITY is set." >&2
    exit 1
  fi
  if [ "${codesign_identity}" = "-" ]; then
    codesign --force --deep --sign - "${app_dir}"
  else
    codesign --force --deep --options runtime --timestamp --sign "${codesign_identity}" "${app_dir}"
  fi
  printf '%s\n' "Codesigned ${app_dir}"
fi

{
  printf '%s\n' "macOS app package: ok"
  printf '%s\n' "App bundle: ${app_dir}"
  printf '%s\n' "Bundle identifier: ${bundle_id}"
  printf '%s\n' "Executable: ${macos_dir}/${binary_name}"
  printf '%s\n' "Info.plist: ${contents_dir}/Info.plist"
  printf '%s\n' "Icon: ${resources_dir}/AppIcon.icns"
  if [ -n "${codesign_identity}" ]; then
    printf '%s\n' "Codesign identity: ${codesign_identity}"
  fi
} | tee "${summary_path}"
