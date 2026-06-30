#!/bin/sh
set -eu

if [ "$(uname -s)" != "Darwin" ]; then
  printf '%s\n' "error: Gromaq Pages terminal recording requires Darwin." >&2
  exit 1
fi

for tool in swift screencapture ffmpeg osascript; do
  if ! command -v "${tool}" >/dev/null 2>&1; then
    printf '%s\n' "error: ${tool} is required for Gromaq Pages terminal recording." >&2
    exit 1
  fi
done

find_window_info() {
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
  guard let bounds = window[kCGWindowBounds as String] as? [String: Any] else {
    continue
  }
  let sharingState = Int(window[kCGWindowSharingState as String] as? Int ?? -1)
  let layer = Int(window[kCGWindowLayer as String] as? Int ?? -1)
  let alpha = Double(window[kCGWindowAlpha as String] as? Double ?? -1.0)

  if owner == targetTitle || name == targetTitle || owner.localizedCaseInsensitiveContains(targetTitle) || name.localizedCaseInsensitiveContains(targetTitle) {
    let x = Int(bounds["X"] as? Double ?? 0)
    let y = Int(bounds["Y"] as? Double ?? 0)
    let width = Int(bounds["Width"] as? Double ?? 0)
    let height = Int(bounds["Height"] as? Double ?? 0)
    guard width > 0, height > 0 else {
      continue
    }
    print("\(number) \(x) \(y) \(width) \(height) \(sharingState) \(layer) \(alpha)")
    exit(0)
  }
}

exit(1)
SWIFT
}

preflight_screen_capture_access() {
  swift - <<'SWIFT'
import CoreGraphics
import Foundation

let allowed = CGPreflightScreenCaptureAccess()
print("macOS screen capture access preflight: \(allowed)")
exit(allowed ? 0 : 1)
SWIFT
}

preflight_accessibility_access() {
  swift - <<'SWIFT'
import ApplicationServices
import Foundation

let options = [kAXTrustedCheckOptionPrompt.takeUnretainedValue() as String: false] as CFDictionary
let allowed = AXIsProcessTrustedWithOptions(options)
print("macOS accessibility access preflight: \(allowed)")
exit(allowed ? 0 : 1)
SWIFT
}

focus_and_type_commands() {
  osascript - "${1}" <<'APPLESCRIPT'
on run argv
  set appPid to item 1 of argv as integer
  tell application "System Events"
    set targetProcess to first process whose unix id is appPid
    set frontmost of targetProcess to true
    delay 0.6
    set commandList to {"gromaq --version", "pwd", "printf 'frame proof line %03d\n' 1 2 3 4 5"}
    repeat with commandText in commandList
      keystroke commandText
      key code 36
      delay 1.1
    end repeat
  end tell
end run
APPLESCRIPT
}

capture_frame() {
  frame_path="${1}"
  capture_status=0
  screencapture -x -l "${window_id}" "${frame_path}" 2>>"${capture_stderr}" || capture_status="$?"
  if [ "${capture_status}" -ne 0 ]; then
    capture_status=0
    screencapture -x -R"${window_region}" "${frame_path}" 2>>"${capture_stderr}" || capture_status="$?"
  fi
  return "${capture_status}"
}

root="$(CDPATH= cd "$(dirname "$0")/.." && pwd)"
recording_root="${GROMAQ_PAGES_RECORDING_ROOT:-${root}/target/pages-recording}"
frames_dir="${recording_root}/frames"
config_path="${recording_root}/gromaq-pages.toml"
shell_path="${recording_root}/pages-shell.sh"
log_path="${recording_root}/capture.log"
summary_path="${recording_root}/summary.txt"
capture_stderr="${recording_root}/screencapture.stderr"
output="${GROMAQ_PAGES_RECORDING_OUTPUT:-${root}/site/assets/gromaq-terminal-recording.webm}"
window_title="${GROMAQ_WINDOW_TITLE:-Gromaq}"
window_lookup_attempts="${GROMAQ_WINDOW_LOOKUP_ATTEMPTS:-30}"
window_lookup_interval="${GROMAQ_WINDOW_LOOKUP_INTERVAL_SECONDS:-0.2}"
fps="${GROMAQ_PAGES_RECORDING_FPS:-12}"
frame_count="${GROMAQ_PAGES_RECORDING_FRAMES:-156}"
frame_interval="${GROMAQ_PAGES_RECORDING_FRAME_INTERVAL_SECONDS:-0.083}"

rm -rf "${recording_root}"
mkdir -p "${frames_dir}" "$(dirname "${output}")"
: > "${log_path}"
: > "${capture_stderr}"

if ! preflight_screen_capture_access >>"${log_path}" 2>&1; then
  {
    printf '%s\n' "macOS Screen Recording permission guidance:"
    printf '%s\n' "Open System Settings > Privacy & Security > Screen & System Audio Recording."
    printf '%s\n' "Grant capture permission to the terminal or automation host running this script, then rerun it."
    printf '%s\n' "Older macOS releases may label the same pane as Screen Recording."
  } >>"${log_path}"
  printf '%s\n' "error: macOS Screen Recording permission is required for Pages terminal recording; see ${log_path}." >&2
  exit 1
fi

if ! preflight_accessibility_access >>"${log_path}" 2>&1; then
  {
    printf '%s\n' "macOS Accessibility permission guidance:"
    printf '%s\n' "Open System Settings > Privacy & Security > Accessibility."
    printf '%s\n' "Grant Accessibility permission to the terminal or automation host running this script, then rerun it."
  } >>"${log_path}"
  printf '%s\n' "error: macOS Accessibility permission is required to type real shell input; see ${log_path}." >&2
  exit 1
fi

(
  cd "${root}"
  cargo build
) >>"${log_path}" 2>&1

cat >"${shell_path}" <<EOF
#!/bin/sh
export PATH="${root}/target/debug:\${PATH}"
cd "${root}"
exec /bin/sh -i
EOF
chmod 755 "${shell_path}"

cat >"${config_path}" <<EOF
[terminal]
cols = 104
rows = 32
scrollback_lines = 2000

[shell]
program = "${shell_path}"
args = []
cwd = "${root}"

[welcome]
enabled = true

[font]
size_px = 16
line_height_px = 22

[theme]
preset = "gromaq-graphite"
surface_padding_px = 18
cell_spacing_px = 0
EOF

(
  cd "${root}"
  "${root}/target/debug/gromaq" --config "${config_path}"
) >>"${log_path}" 2>&1 &
app_pid="$!"

cleanup() {
  if kill -0 "${app_pid}" >/dev/null 2>&1; then
    kill "${app_pid}" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT INT TERM

window_info=""
attempt=1
while [ "${attempt}" -le "${window_lookup_attempts}" ]; do
  if window_info="$(find_window_info "${window_title}" 2>>"${log_path}")" && [ -n "${window_info}" ]; then
    break
  fi
  sleep "${window_lookup_interval}"
  attempt=$((attempt + 1))
done

if [ -z "${window_info}" ]; then
  printf '%s\n' "error: could not find a visible Gromaq window for Pages terminal recording; see ${log_path}." >&2
  exit 1
fi

set -- ${window_info}
window_id="$1"
window_x="$2"
window_y="$3"
window_width="$4"
window_height="$5"
window_sharing_state="$6"
window_layer="$7"
window_alpha="$8"
window_region="${window_x},${window_y},${window_width},${window_height}"

{
  printf '%s\n' "macOS window id: ${window_id}"
  printf '%s\n' "macOS window region: ${window_region}"
  printf '%s\n' "macOS window sharing state: ${window_sharing_state}"
  printf '%s\n' "macOS window layer: ${window_layer}"
  printf '%s\n' "macOS window alpha: ${window_alpha}"
} >>"${log_path}"

capture_status_path="${recording_root}/capture-status"
(
  frame=1
  while [ "${frame}" -le "${frame_count}" ]; do
    frame_file="${frames_dir}/frame-$(printf '%04d' "${frame}").png"
    if ! capture_frame "${frame_file}"; then
      printf '%s\n' "capture failed at frame ${frame}" >"${capture_status_path}"
      exit 1
    fi
    frame=$((frame + 1))
    sleep "${frame_interval}"
  done
  printf '%s\n' "ok" >"${capture_status_path}"
) &
capture_pid="$!"

sleep 1.2
if ! focus_and_type_commands "${app_pid}" >>"${log_path}" 2>&1; then
  wait "${capture_pid}" || true
  printf '%s\n' "error: osascript could not type real shell input into Gromaq; see ${log_path}." >&2
  exit 1
fi

if ! wait "${capture_pid}"; then
  if [ -s "${capture_stderr}" ]; then
    cat "${capture_stderr}" >>"${log_path}"
  fi
  printf '%s\n' "error: screencapture failed while recording Gromaq frames; see ${log_path}." >&2
  exit 1
fi

rm -f "${output}"
ffmpeg \
  -y \
  -framerate "${fps}" \
  -i "${frames_dir}/frame-%04d.png" \
  -c:v libvpx-vp9 \
  -pix_fmt yuv420p \
  -b:v 0 \
  -crf 34 \
  "${output}" >>"${log_path}" 2>&1

duration="unknown"
if command -v ffprobe >/dev/null 2>&1; then
  duration="$(ffprobe -v error -show_entries format=duration -of default=noprint_wrappers=1:nokey=1 "${output}" 2>>"${log_path}" || printf '%s' unknown)"
fi

{
  printf '%s\n' "Gromaq Pages terminal recording: ok"
  printf '%s\n' "Recording: ${output}"
  printf '%s\n' "Frames: ${frames_dir}"
  printf '%s\n' "Duration seconds: ${duration}"
  printf '%s\n' "Config: ${config_path}"
  printf '%s\n' "Log: ${log_path}"
} | tee "${summary_path}"
