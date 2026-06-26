#!/bin/sh
set -eu

repo="${GROMAQ_REPO:-https://github.com/vicotrbb/gromaq}"
branch="${GROMAQ_BRANCH:-main}"
package="${GROMAQ_PACKAGE:-gromaq}"
raw_base="${GROMAQ_RAW_BASE:-https://raw.githubusercontent.com/vicotrbb/gromaq/${branch}}"
asset_root="${GROMAQ_ASSET_ROOT:-}"
bin_dir="${CARGO_HOME:-${HOME}/.cargo}/bin"
platform="${GROMAQ_PLATFORM:-$(uname -s)}"
install_temp_root=""
macos_asset_root=""

cleanup_install_temp_root() {
  if [ -n "${install_temp_root}" ]; then
    rm -rf "${install_temp_root}"
  fi
}

trap cleanup_install_temp_root EXIT

if [ "${GROMAQ_SKIP_CARGO_INSTALL:-0}" != "1" ] && ! command -v cargo >/dev/null 2>&1; then
  printf '%s\n' "error: Cargo is required to install Gromaq." >&2
  printf '%s\n' "Install Rust stable from your package manager or https://rustup.rs, then rerun this installer." >&2
  exit 1
fi

if [ "${GROMAQ_SKIP_CARGO_INSTALL:-0}" = "1" ]; then
  printf '%s\n' "Skipping Cargo install because GROMAQ_SKIP_CARGO_INSTALL=1."
else
  printf '%s\n' "Installing ${package} from ${repo} (${branch})..."
  cargo install --git "${repo}" --branch "${branch}" --locked --force "${package}"
fi

install_file() {
  source="$1"
  destination="$2"
  mkdir -p "$(dirname "${destination}")"
  if [ -n "${asset_root}" ] && [ -f "${asset_root}/${source}" ]; then
    cp "${asset_root}/${source}" "${destination}"
    return
  fi
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "${raw_base}/${source}" -o "${destination}"
    return
  fi
  printf '%s\n' "warning: curl is unavailable; skipped ${destination}" >&2
}

install_required_file() {
  source="$1"
  destination="$2"
  mkdir -p "$(dirname "${destination}")"
  if [ -n "${asset_root}" ] && [ -f "${asset_root}/${source}" ]; then
    cp "${asset_root}/${source}" "${destination}"
    return
  fi
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "${raw_base}/${source}" -o "${destination}"
    return
  fi
  printf '%s\n' "error: curl is required to fetch ${source}." >&2
  exit 1
}

install_linux_desktop_assets() {
  if [ -n "${GROMAQ_INSTALL_ROOT:-}" ]; then
    data_home="${GROMAQ_INSTALL_ROOT}/share"
  else
    data_home="${XDG_DATA_HOME:-${HOME}/.local/share}"
  fi
  install_file "images/logos/logo-icon-256.png" \
    "${data_home}/icons/hicolor/256x256/apps/dev.gromaq.Gromaq.png"
  install_file "packaging/linux/dev.gromaq.Gromaq.desktop" \
    "${data_home}/applications/dev.gromaq.Gromaq.desktop"
  install_file "packaging/linux/dev.gromaq.Gromaq.metainfo.xml" \
    "${data_home}/metainfo/dev.gromaq.Gromaq.metainfo.xml"
  if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "${data_home}/applications" >/dev/null 2>&1 || true
  fi
  printf '%s\n' "Installed Linux desktop assets under ${data_home}."
}

prepare_macos_asset_root() {
  if [ -n "${asset_root}" ]; then
    macos_asset_root="${asset_root}"
    return
  fi

  install_temp_root="$(mktemp -d "${TMPDIR:-/tmp}/gromaq-install.XXXXXX")"
  install_required_file "Cargo.toml" "${install_temp_root}/Cargo.toml"
  install_required_file "scripts/package-macos-app.sh" \
    "${install_temp_root}/scripts/package-macos-app.sh"
  install_required_file "images/logos/logo-transparent.png" \
    "${install_temp_root}/images/logos/logo-transparent.png"
  install_required_file "images/logos/logo-icon-128.png" \
    "${install_temp_root}/images/logos/logo-icon-128.png"
  install_required_file "images/logos/logo-icon-256.png" \
    "${install_temp_root}/images/logos/logo-icon-256.png"
  install_required_file "images/logos/logo-icon-512.png" \
    "${install_temp_root}/images/logos/logo-icon-512.png"
  chmod 755 "${install_temp_root}/scripts/package-macos-app.sh"
  macos_asset_root="${install_temp_root}"
}

install_macos_app_bundle() {
  prepare_macos_asset_root
  app_name="${GROMAQ_APP_NAME:-Gromaq}"
  app_dir="${GROMAQ_MACOS_APP_DIR:-${HOME}/Applications}"
  app_path="${macos_asset_root}/target/dist/${app_name}.app"
  destination="${app_dir}/${app_name}.app"

  GROMAQ_BINARY_PATH="${bin_dir}/${package}" "${macos_asset_root}/scripts/package-macos-app.sh"
  mkdir -p "${app_dir}"
  rm -rf "${destination}"
  cp -R "${app_path}" "${destination}"
  printf '%s\n' "Installed macOS app bundle to ${destination}."
}

case "${platform}" in
  Linux)
    if [ "${GROMAQ_INSTALL_DESKTOP_ASSETS:-1}" != "0" ]; then
      install_linux_desktop_assets
    fi
    ;;
  Darwin)
    if [ "${GROMAQ_INSTALL_APP_BUNDLE:-0}" = "1" ]; then
      install_macos_app_bundle
    fi
    ;;
esac

printf '%s\n' "Installed ${package}."
printf '%s\n' "Run it with: ${package}"
