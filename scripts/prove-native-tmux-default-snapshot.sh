#!/bin/sh
set -eu

root="$(CDPATH= cd "$(dirname "$0")/.." && pwd)"
proof_dir="${GROMAQ_NATIVE_TMUX_DEFAULT_SNAPSHOT_PROOF_DIR:-${root}/target/native-tmux-default-snapshot-proof}"
ppm_path="${proof_dir}/gromaq-native-tmux-default-snapshot.ppm"
png_path="${proof_dir}/gromaq-native-tmux-default-snapshot.png"
log_path="${proof_dir}/window-tmux-manager-snapshot.log"
summary_path="${proof_dir}/summary.txt"
snapshot_session="gromaq-default-snapshot-$$"

cleanup() {
  tmux kill-session -t "${snapshot_session}" >/dev/null 2>&1 || true
}

rm -rf "${proof_dir}"
mkdir -p "${proof_dir}"
trap cleanup EXIT INT TERM

if command -v tmux >/dev/null 2>&1; then
  cleanup
  tmux new-session -d -s "${snapshot_session}" -n code
  tmux split-window -t "${snapshot_session}:0" -h
else
  printf '%s\n' "error: tmux is required for native tmux default snapshot proof." >&2
  exit 1
fi

if ! (
  cd "${root}"
  cargo run -- --window-tmux-manager-snapshot "${ppm_path}"
) > "${log_path}" 2>&1; then
  cat "${log_path}" >&2
  exit 1
fi

for marker in \
  "default startup content checked: true" \
  "tmux status strip rendered: true" \
  "tmux manager panel rendered: true" \
  "tmux manager sessions:" \
  "tmux manager windows:" \
  "tmux manager panes:"
do
  if ! grep -F "${marker}" "${log_path}" >/dev/null; then
    printf '%s\n' "error: native tmux default snapshot proof missing '${marker}'." >&2
    cat "${log_path}" >&2
    exit 1
  fi
done

require_positive_tmux_count() {
  label="$1"
  value="$(grep -F "${label}:" "${log_path}" | tail -n 1 | awk '{print $NF}')"
  case "${value}" in
    ''|*[!0-9]*)
      printf '%s\n' "error: native tmux default snapshot proof could not parse ${label} count." >&2
      cat "${log_path}" >&2
      exit 1
      ;;
  esac
  if [ "${value}" -le 0 ]; then
    printf '%s\n' "error: native tmux default snapshot proof reported ${label}: ${value}." >&2
    cat "${log_path}" >&2
    exit 1
  fi
}

require_positive_tmux_count "tmux manager sessions"
require_positive_tmux_count "tmux manager windows"
require_positive_tmux_count "tmux manager panes"

if [ ! -s "${ppm_path}" ]; then
  printf '%s\n' "error: native tmux default snapshot proof did not write ${ppm_path}" >&2
  exit 1
fi

if [ "$(head -c 2 "${ppm_path}")" != "P6" ]; then
  printf '%s\n' "error: ${ppm_path} is not a binary PPM artifact" >&2
  exit 1
fi

if command -v sips >/dev/null 2>&1; then
  sips -s format png "${ppm_path}" --out "${png_path}" >/dev/null
  if [ ! -s "${png_path}" ]; then
    printf '%s\n' "error: native tmux default snapshot PNG conversion did not write ${png_path}" >&2
    exit 1
  fi
  printf '%s\n' "native tmux default snapshot PNG: ${png_path}"
else
  printf '%s\n' "native tmux default snapshot PNG: skipped (sips not available)"
fi

{
  printf '%s\n' "native tmux default snapshot proof: ok"
  printf '%s\n' "Command: cargo run -- --window-tmux-manager-snapshot ${ppm_path}"
  printf '%s\n' "snapshot-session: ${snapshot_session}"
  printf '%s\n' "Log: ${log_path}"
  printf '%s\n' "PPM artifact: ${ppm_path}"
  if [ -s "${png_path}" ]; then
    printf '%s\n' "PNG artifact: ${png_path}"
  fi
  grep -F "default startup content checked: true" "${log_path}"
  grep -F "tmux status strip rendered: true" "${log_path}"
  grep -F "tmux manager panel rendered: true" "${log_path}"
  grep -F "tmux manager sessions:" "${log_path}"
  grep -F "tmux manager windows:" "${log_path}"
  grep -F "tmux manager panes:" "${log_path}"
} | tee "${summary_path}"
