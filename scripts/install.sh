#!/bin/sh
set -eu

repo="${GROMAQ_REPO:-https://github.com/vicotrbb/gromaq}"
branch="${GROMAQ_BRANCH:-main}"
package="${GROMAQ_PACKAGE:-gromaq}"
raw_base="${GROMAQ_RAW_BASE:-https://raw.githubusercontent.com/vicotrbb/gromaq/${branch}}"
asset_root="${GROMAQ_ASSET_ROOT:-}"
bin_dir="${GROMAQ_BIN_DIR:-${CARGO_HOME:-${HOME}/.cargo}/bin}"
platform="${GROMAQ_PLATFORM:-$(uname -s)}"
dry_run="${GROMAQ_DRY_RUN:-0}"
install_method="${GROMAQ_INSTALL_METHOD:-cargo}"
release_version="${GROMAQ_VERSION:-v0.2.0}"
release_package_version="${release_version#v}"
release_base="${GROMAQ_RELEASE_BASE:-${repo}/releases/download/${release_version}}"
verify_checksums="${GROMAQ_VERIFY_CHECKSUMS:-1}"
install_temp_root=""
macos_asset_root=""
release_temp_root=""
installed_linux_desktop_assets=0

cleanup_install_temp_root() {
  if [ -n "${install_temp_root}" ]; then
    rm -rf "${install_temp_root}"
  fi
  if [ -n "${release_temp_root}" ]; then
    rm -rf "${release_temp_root}"
  fi
}

trap cleanup_install_temp_root EXIT

linux_data_home() {
  if [ -n "${GROMAQ_INSTALL_ROOT:-}" ]; then
    printf '%s\n' "${GROMAQ_INSTALL_ROOT}/share"
  else
    printf '%s\n' "${XDG_DATA_HOME:-${HOME}/.local/share}"
  fi
}

linux_release_target() {
  if [ -n "${GROMAQ_RELEASE_TARGET:-}" ]; then
    printf '%s\n' "${GROMAQ_RELEASE_TARGET}"
    return
  fi

  arch="$(uname -m)"
  case "${arch}" in
    amd64)
      arch="x86_64"
      ;;
    arm64)
      arch="aarch64"
      ;;
  esac
  printf 'linux-%s\n' "${arch}"
}

release_asset_name() {
  printf '%s-%s-%s.tar.gz\n' "${package}" "${release_package_version}" "$(linux_release_target)"
}

release_asset_url() {
  printf '%s/%s\n' "${release_base}" "$(release_asset_name)"
}

checksum_asset_name() {
  printf '%s\n' "${GROMAQ_CHECKSUM_ASSET:-SHA256SUMS-$(linux_release_target)}"
}

checksum_asset_url() {
  printf '%s/%s\n' "${release_base}" "$(checksum_asset_name)"
}

checksum_command() {
  if command -v shasum >/dev/null 2>&1; then
    printf '%s\n' "shasum"
    return
  fi
  if command -v sha256sum >/dev/null 2>&1; then
    printf '%s\n' "sha256sum"
    return
  fi
  printf '%s\n' "error: shasum or sha256sum is required to verify release checksums." >&2
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

print_dry_run_and_exit() {
  printf '%s\n' "Dry run: would install ${package} from ${repo} (${branch})."
  if [ "${GROMAQ_SKIP_CARGO_INSTALL:-0}" = "1" ]; then
    printf '%s\n' "Dry run: would skip cargo install because GROMAQ_SKIP_CARGO_INSTALL=1."
  elif [ "${install_method}" = "release" ]; then
    printf '%s\n' "Dry run: would download release asset $(release_asset_url)."
    if [ "${verify_checksums}" != "0" ]; then
      printf '%s\n' "Dry run: would verify release checksum from $(checksum_asset_url)."
    fi
    printf '%s\n' "Dry run: would install binary to ${bin_dir}/${package}."
  else
    printf '%s\n' "Dry run: would run cargo install --git ${repo} --branch ${branch} --locked --force ${package}."
    printf '%s\n' "Dry run: expected binary path is ${bin_dir}/${package}."
  fi

  case "${platform}" in
    Linux)
      if [ "${GROMAQ_INSTALL_DESKTOP_ASSETS:-1}" != "0" ]; then
        printf '%s\n' "Dry run: would install Linux desktop assets under $(linux_data_home)."
      else
        printf '%s\n' "Dry run: would skip Linux desktop assets because GROMAQ_INSTALL_DESKTOP_ASSETS=0."
      fi
      ;;
    Darwin)
      if [ "${GROMAQ_INSTALL_APP_BUNDLE:-0}" = "1" ]; then
        app_name="${GROMAQ_APP_NAME:-Gromaq}"
        app_dir="${GROMAQ_MACOS_APP_DIR:-${HOME}/Applications}"
        printf '%s\n' "Dry run: would install macOS app bundle to ${app_dir}/${app_name}.app."
      fi
      ;;
  esac

  printf '%s\n' "Dry run complete; no files written."
  exit 0
}

if [ "${dry_run}" = "1" ]; then
  print_dry_run_and_exit
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

refresh_linux_desktop_database() {
  applications_dir="$1"
  if command -v update-desktop-database >/dev/null 2>&1; then
    if update-desktop-database "${applications_dir}" >/dev/null 2>&1; then
      printf '%s\n' "Refreshed Linux desktop database under ${applications_dir}."
    else
      printf '%s\n' "warning: update-desktop-database failed for ${applications_dir}; continuing." >&2
    fi
  fi
}

install_linux_desktop_assets() {
  data_home="$(linux_data_home)"
  install_file "images/logos/logo-icon-256.png" \
    "${data_home}/icons/hicolor/256x256/apps/dev.gromaq.Gromaq.png"
  install_file "packaging/linux/dev.gromaq.Gromaq.desktop" \
    "${data_home}/applications/dev.gromaq.Gromaq.desktop"
  install_file "packaging/linux/dev.gromaq.Gromaq.metainfo.xml" \
    "${data_home}/metainfo/dev.gromaq.Gromaq.metainfo.xml"
  refresh_linux_desktop_database "${data_home}/applications"
  printf '%s\n' "Installed Linux desktop assets under ${data_home}."
}

install_linux_desktop_assets_from_release() {
  release_root="$1"
  data_home="$(linux_data_home)"
  mkdir -p \
    "${data_home}/applications" \
    "${data_home}/icons/hicolor/256x256/apps" \
    "${data_home}/metainfo"
  cp "${release_root}/share/applications/dev.gromaq.Gromaq.desktop" \
    "${data_home}/applications/dev.gromaq.Gromaq.desktop"
  cp "${release_root}/share/icons/hicolor/256x256/apps/dev.gromaq.Gromaq.png" \
    "${data_home}/icons/hicolor/256x256/apps/dev.gromaq.Gromaq.png"
  cp "${release_root}/share/metainfo/dev.gromaq.Gromaq.metainfo.xml" \
    "${data_home}/metainfo/dev.gromaq.Gromaq.metainfo.xml"
  refresh_linux_desktop_database "${data_home}/applications"
  installed_linux_desktop_assets=1
  printf '%s\n' "Installed Linux desktop assets under ${data_home}."
}

install_release_tarball() {
  if [ "${platform}" != "Linux" ]; then
    printf '%s\n' "error: GROMAQ_INSTALL_METHOD=release currently supports Linux tarball releases only." >&2
    exit 1
  fi
  if ! command -v curl >/dev/null 2>&1; then
    printf '%s\n' "error: curl is required to download Gromaq release assets." >&2
    exit 1
  fi

  release_temp_root="$(mktemp -d "${TMPDIR:-/tmp}/gromaq-release-install.XXXXXX")"
  archive="${release_temp_root}/$(release_asset_name)"
  extract_dir="${release_temp_root}/extract"
  release_root="${extract_dir}/${package}-${release_package_version}-$(linux_release_target)"

  printf '%s\n' "Downloading $(release_asset_url)..."
  curl -fsSL "$(release_asset_url)" -o "${archive}"
  verify_release_checksum "${archive}"
  mkdir -p "${extract_dir}"
  tar -xzf "${archive}" -C "${extract_dir}"

  if [ ! -x "${release_root}/bin/${package}" ]; then
    printf '%s\n' "error: release archive did not contain bin/${package}." >&2
    exit 1
  fi

  mkdir -p "${bin_dir}"
  cp "${release_root}/bin/${package}" "${bin_dir}/${package}"
  chmod 755 "${bin_dir}/${package}"

  if [ "${GROMAQ_INSTALL_DESKTOP_ASSETS:-1}" != "0" ]; then
    install_linux_desktop_assets_from_release "${release_root}"
  fi
}

verify_release_checksum() {
  archive="$1"
  if [ "${verify_checksums}" = "0" ]; then
    return
  fi

  manifest="${release_temp_root}/$(checksum_asset_name)"
  printf '%s\n' "Verifying checksum from $(checksum_asset_url)..."
  curl -fsSL "$(checksum_asset_url)" -o "${manifest}"
  artifact="$(basename "${archive}")"
  expected="$(awk -v artifact="${artifact}" '$2 == artifact { print $1; exit }' "${manifest}")"
  if [ -z "${expected}" ]; then
    printf '%s\n' "error: checksum manifest did not contain ${artifact}." >&2
    exit 1
  fi

  actual="$(hash_file "$(checksum_command)" "${archive}")"
  if [ "${actual}" != "${expected}" ]; then
    printf '%s\n' "error: checksum mismatch for ${artifact}." >&2
    exit 1
  fi
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

if [ "${GROMAQ_SKIP_CARGO_INSTALL:-0}" = "1" ]; then
  printf '%s\n' "Skipping Cargo install because GROMAQ_SKIP_CARGO_INSTALL=1."
elif [ "${install_method}" = "cargo" ]; then
  if ! command -v cargo >/dev/null 2>&1; then
    printf '%s\n' "error: Cargo is required to install Gromaq." >&2
    printf '%s\n' "Install Rust stable from your package manager or https://rustup.rs, then rerun this installer." >&2
    exit 1
  fi
  printf '%s\n' "Installing ${package} from ${repo} (${branch})..."
  cargo install --git "${repo}" --branch "${branch}" --locked --force "${package}"
elif [ "${install_method}" = "release" ]; then
  install_release_tarball
else
  printf '%s\n' "error: unsupported GROMAQ_INSTALL_METHOD=${install_method}; use cargo or release." >&2
  exit 1
fi

case "${platform}" in
  Linux)
    if [ "${GROMAQ_INSTALL_DESKTOP_ASSETS:-1}" != "0" ] && [ "${installed_linux_desktop_assets}" != "1" ]; then
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
