#!/bin/sh
set -eu

root="$(CDPATH= cd "$(dirname "$0")/.." && pwd)"
proof_root="${GROMAQ_DESKTOP_DISCOVERY_PROOF_ROOT:-${root}/target/linux-desktop-discovery-proof}"
log_path="${GROMAQ_DESKTOP_DISCOVERY_LOG:-${proof_root}.log}"
data_home="${proof_root}/share"
applications_dir="${data_home}/applications"
icon_theme_dir="${data_home}/icons/hicolor"
desktop_file="${applications_dir}/dev.gromaq.Gromaq.desktop"
metainfo_file="${data_home}/metainfo/dev.gromaq.Gromaq.metainfo.xml"
icon_file="${icon_theme_dir}/256x256/apps/dev.gromaq.Gromaq.png"

require_command() {
  command_name="$1"
  if ! command -v "${command_name}" >/dev/null 2>&1; then
    printf '%s\n' "error: ${command_name} is required for Linux desktop discovery proof." >&2
    exit 1
  fi
}

write_hicolor_index_if_missing() {
  index_path="${icon_theme_dir}/index.theme"
  if [ -f "${index_path}" ]; then
    return
  fi
  cat > "${index_path}" <<'EOF'
[Icon Theme]
Name=Hicolor
Directories=256x256/apps

[256x256/apps]
Size=256
Type=Fixed
Context=Applications
EOF
}

if [ "$(uname -s)" != "Linux" ]; then
  printf '%s\n' "error: Linux desktop discovery proof must run on Linux." >&2
  exit 1
fi

require_command desktop-file-validate
require_command appstreamcli
require_command update-desktop-database
require_command gtk-update-icon-cache

rm -rf "${proof_root}"
mkdir -p "${proof_root}"

GROMAQ_SKIP_CARGO_INSTALL=1 \
  GROMAQ_PLATFORM=Linux \
  GROMAQ_ASSET_ROOT="${root}" \
  GROMAQ_INSTALL_ROOT="${proof_root}" \
  "${root}/scripts/install.sh" > "${log_path}" 2>&1

test -f "${desktop_file}"
test -f "${metainfo_file}"
test -f "${icon_file}"
grep -F "Name=Gromaq" "${desktop_file}" >/dev/null
grep -F "Exec=gromaq" "${desktop_file}" >/dev/null
grep -F "Icon=dev.gromaq.Gromaq" "${desktop_file}" >/dev/null
grep -F "Categories=System;TerminalEmulator;" "${desktop_file}" >/dev/null
grep -F "<launchable type=\"desktop-id\">dev.gromaq.Gromaq.desktop</launchable>" \
  "${metainfo_file}" >/dev/null

desktop-file-validate "${desktop_file}" >> "${log_path}" 2>&1
appstreamcli validate --no-net "${metainfo_file}" >> "${log_path}" 2>&1
update-desktop-database "${applications_dir}" >> "${log_path}" 2>&1
write_hicolor_index_if_missing
gtk-update-icon-cache -q -t -f "${icon_theme_dir}" >> "${log_path}" 2>&1

printf '%s\n' "Linux desktop discovery proof: ok"
printf '%s\n' "Proof root: ${proof_root}"
printf '%s\n' "Proof log: ${log_path}"
printf '%s\n' "This validates installed desktop metadata and caches; it does not prove live menu UI rendering."
