#!/bin/sh
set -eu

repo="${GROMAQ_REPO:-https://github.com/vicotrbb/gromaq}"
branch="${GROMAQ_BRANCH:-main}"
package="${GROMAQ_PACKAGE:-gromaq}"
raw_base="${GROMAQ_RAW_BASE:-https://raw.githubusercontent.com/vicotrbb/gromaq/${branch}}"
asset_root="${GROMAQ_ASSET_ROOT:-}"
bin_dir="${CARGO_HOME:-${HOME}/.cargo}/bin"
platform="${GROMAQ_PLATFORM:-$(uname -s)}"

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

case "${platform}" in
  Linux)
    if [ "${GROMAQ_INSTALL_DESKTOP_ASSETS:-1}" != "0" ]; then
      install_linux_desktop_assets
    fi
    ;;
  Darwin)
    if [ "${GROMAQ_INSTALL_APP_BUNDLE:-0}" = "1" ] && [ -n "${asset_root}" ]; then
      GROMAQ_BINARY_PATH="${bin_dir}/${package}" "${asset_root}/scripts/package-macos-app.sh"
    fi
    ;;
esac

printf '%s\n' "Installed ${package}."
printf '%s\n' "Run it with: ${package}"
