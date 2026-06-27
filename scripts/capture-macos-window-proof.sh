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

if ! command -v swift >/dev/null 2>&1; then
  printf '%s\n' "error: swift is required to locate the Gromaq window for screenshot proof." >&2
  exit 1
fi

find_window_id() {
  swift - "${1}" <<'SWIFT'
import CoreGraphics
import Foundation

let targetTitle = CommandLine.arguments.dropFirst().first ?? "Gromaq"
let options = CGWindowListOption(arrayLiteral: .optionOnScreenOnly, .excludeDesktopElements)

guard let windows = CGWindowListCopyWindowInfo(options, kCGNullWindowID) as? [[String: Any]] else {
  exit(1)
}

for window in windows {
  let owner = window[kCGWindowOwnerName as String] as? String ?? ""
  let name = window[kCGWindowName as String] as? String ?? ""
  guard let number = window[kCGWindowNumber as String] as? UInt32 else {
    continue
  }

  if owner == targetTitle || name == targetTitle || owner.localizedCaseInsensitiveContains(targetTitle) || name.localizedCaseInsensitiveContains(targetTitle) {
    print(number)
    exit(0)
  }
}

exit(1)
SWIFT
}

root="$(CDPATH= cd "$(dirname "$0")/.." && pwd)"
output="${1:-${root}/target/gromaq-live-window-proof.png}"
log_path="${GROMAQ_WINDOW_PROOF_LOG:-${root}/target/gromaq-live-window-proof.log}"
capture_stderr="${log_path}.screencapture"
delay="${GROMAQ_SCREENSHOT_DELAY_SECONDS:-0.8}"
window_title="${GROMAQ_WINDOW_TITLE:-Gromaq}"
window_lookup_attempts="${GROMAQ_WINDOW_LOOKUP_ATTEMPTS:-20}"
window_lookup_interval="${GROMAQ_WINDOW_LOOKUP_INTERVAL_SECONDS:-0.2}"

mkdir -p "$(dirname "${output}")" "$(dirname "${log_path}")"
rm -f "${capture_stderr}"

(
  cd "${root}"
  cargo run -- --window-screenshot-smoke
) > "${log_path}" 2>&1 &
app_pid="$!"

sleep "${delay}"

window_id=""
attempt=1
while [ "${attempt}" -le "${window_lookup_attempts}" ]; do
  if window_id="$(find_window_id "${window_title}" 2>>"${log_path}")" && [ -n "${window_id}" ]; then
    break
  fi
  sleep "${window_lookup_interval}"
  attempt=$((attempt + 1))
done

if [ -z "${window_id}" ]; then
  app_status=0
  wait "${app_pid}" || app_status="$?"
  printf '%s\n' "error: could not find a visible Gromaq window for screenshot proof." >&2
  printf '%s\n' "error: window smoke exited with status ${app_status}; see ${log_path}." >&2
  exit 1
fi

capture_status=0
screencapture -x -l "${window_id}" "${output}" 2> "${capture_stderr}" || capture_status="$?"

app_status=0
wait "${app_pid}" || app_status="$?"

{
  printf '%s\n' "macOS window id: ${window_id}"
  if [ -s "${capture_stderr}" ]; then
    cat "${capture_stderr}"
  fi
} >> "${log_path}"
rm -f "${capture_stderr}"

if [ "${capture_status}" -ne 0 ]; then
  printf '%s\n' "error: screencapture could not capture Gromaq window id ${window_id}; see ${log_path}." >&2
  exit "${capture_status}"
fi

if [ "${app_status}" -ne 0 ]; then
  printf '%s\n' "error: window smoke exited with status ${app_status}; see ${log_path}." >&2
  exit "${app_status}"
fi

printf '%s\n' "Wrote screenshot proof: ${output}"
printf '%s\n' "Wrote window smoke log: ${log_path}"
