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

  if owner == targetTitle || name == targetTitle || owner.localizedCaseInsensitiveContains(targetTitle) || name.localizedCaseInsensitiveContains(targetTitle) {
    let x = Int(bounds["X"] as? Double ?? 0)
    let y = Int(bounds["Y"] as? Double ?? 0)
    let width = Int(bounds["Width"] as? Double ?? 0)
    let height = Int(bounds["Height"] as? Double ?? 0)
    guard width > 0, height > 0 else {
      continue
    }
    print("\(number) \(x) \(y) \(width) \(height)")
    exit(0)
  }
}

exit(1)
SWIFT
}

validate_screenshot_contains_terminal_background() {
  swift - "${1}" "${2}" <<'SWIFT'
import AppKit
import Foundation

let path = CommandLine.arguments.dropFirst().first ?? ""
let minimum = Int(CommandLine.arguments.dropFirst(2).first ?? "") ?? 2500
guard let image = NSImage(contentsOfFile: path),
      let tiff = image.tiffRepresentation,
      let bitmap = NSBitmapImageRep(data: tiff) else {
  fputs("error: could not read screenshot image for validation.\n", stderr)
  exit(1)
}

let width = bitmap.pixelsWide
let height = bitmap.pixelsHigh
let sampleStep = 4
let target = (red: 0x10, green: 0x12, blue: 0x16)
let tolerance = 3
var matches = 0

for y in stride(from: 0, to: height, by: sampleStep) {
  for x in stride(from: 0, to: width, by: sampleStep) {
    guard let color = bitmap.colorAt(x: x, y: y)?.usingColorSpace(.sRGB) else {
      continue
    }
    let red = Int((color.redComponent * 255.0).rounded())
    let green = Int((color.greenComponent * 255.0).rounded())
    let blue = Int((color.blueComponent * 255.0).rounded())
    if abs(red - target.red) <= tolerance &&
       abs(green - target.green) <= tolerance &&
       abs(blue - target.blue) <= tolerance {
      matches += 1
    }
  }
}

print("terminal background sampled pixels: \(matches)")
if matches < minimum {
  fputs("error: screenshot did not contain enough default terminal background pixels; got \(matches), need \(minimum).\n", stderr)
  exit(1)
}
SWIFT
}

root="$(CDPATH= cd "$(dirname "$0")/.." && pwd)"
output="${1:-${root}/target/gromaq-live-window-proof.png}"
log_path="${GROMAQ_WINDOW_PROOF_LOG:-${root}/target/gromaq-live-window-proof.log}"
capture_stderr="${log_path}.screencapture"
validation_stderr="${log_path}.validation"
delay="${GROMAQ_SCREENSHOT_DELAY_SECONDS:-0.05}"
window_title="${GROMAQ_WINDOW_TITLE:-Gromaq}"
min_background_pixels="${GROMAQ_SCREENSHOT_MIN_BACKGROUND_PIXELS:-2500}"
window_lookup_attempts="${GROMAQ_WINDOW_LOOKUP_ATTEMPTS:-20}"
window_lookup_interval="${GROMAQ_WINDOW_LOOKUP_INTERVAL_SECONDS:-0.2}"

mkdir -p "$(dirname "${output}")" "$(dirname "${log_path}")"
rm -f "${capture_stderr}" "${validation_stderr}"

(
  cd "${root}"
  cargo run -- --window-screenshot-smoke
) > "${log_path}" 2>&1 &
app_pid="$!"

sleep "${delay}"

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
  app_status=0
  wait "${app_pid}" || app_status="$?"
  printf '%s\n' "error: could not find a visible Gromaq window for screenshot proof." >&2
  printf '%s\n' "error: window smoke exited with status ${app_status}; see ${log_path}." >&2
  exit 1
fi

set -- ${window_info}
window_id="$1"
window_x="$2"
window_y="$3"
window_width="$4"
window_height="$5"
window_region="${window_x},${window_y},${window_width},${window_height}"
capture_status=0
screencapture -x -l "${window_id}" "${output}" 2> "${capture_stderr}" || capture_status="$?"
capture_method="window-id"

if [ "${capture_status}" -ne 0 ]; then
  {
    printf '%s\n' "window-id capture failed; attempting bounded region capture: ${window_region}"
    if [ -s "${capture_stderr}" ]; then
      cat "${capture_stderr}"
    fi
  } >> "${log_path}"
  capture_status=0
  screencapture -x -R"${window_region}" "${output}" 2> "${capture_stderr}" || capture_status="$?"
  capture_method="window-region"
fi

app_status=0
wait "${app_pid}" || app_status="$?"

{
  printf '%s\n' "macOS window id: ${window_id}"
  printf '%s\n' "macOS window region: ${window_region}"
  printf '%s\n' "macOS capture method: ${capture_method}"
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

validation_status=0
validate_screenshot_contains_terminal_background "${output}" "${min_background_pixels}" >> "${log_path}" 2> "${validation_stderr}" || validation_status="$?"
if [ -s "${validation_stderr}" ]; then
  cat "${validation_stderr}" >> "${log_path}"
fi
rm -f "${validation_stderr}"
if [ "${validation_status}" -ne 0 ]; then
  rm -f "${output}"
  printf '%s\n' "error: screenshot validation rejected ${output}; see ${log_path}." >&2
  exit "${validation_status}"
fi

printf '%s\n' "Wrote screenshot proof: ${output}"
printf '%s\n' "Wrote window smoke log: ${log_path}"
