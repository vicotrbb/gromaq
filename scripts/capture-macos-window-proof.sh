#!/bin/sh
set -eu

if [ "$(uname -s)" != "Darwin" ]; then
  printf '%s\n' "error: macOS live screenshot proof requires Darwin." >&2
  exit 1
fi

if ! command -v screencapture >/dev/null 2>&1; then
  printf '%s\n' "error: screencapture is required for macOS screenshot proof." >&2
  exit 1
fi

root="$(CDPATH= cd "$(dirname "$0")/.." && pwd)"
output="${1:-${root}/target/gromaq-live-window-proof.png}"
log_path="${GROMAQ_WINDOW_PROOF_LOG:-${root}/target/gromaq-live-window-proof.log}"
delay="${GROMAQ_SCREENSHOT_DELAY_SECONDS:-0.8}"

mkdir -p "$(dirname "${output}")" "$(dirname "${log_path}")"

(
  cd "${root}"
  cargo run -- --window-screenshot-smoke
) > "${log_path}" 2>&1 &
app_pid="$!"

sleep "${delay}"
screencapture -x "${output}"

wait "${app_pid}"

printf '%s\n' "Wrote screenshot proof: ${output}"
printf '%s\n' "Wrote window smoke log: ${log_path}"
