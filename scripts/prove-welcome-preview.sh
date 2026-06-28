#!/bin/sh
set -eu

root="$(CDPATH= cd "$(dirname "$0")/.." && pwd)"
proof_dir="${GROMAQ_WELCOME_PREVIEW_PROOF_DIR:-${root}/target/welcome-preview-proof}"
ppm_path="${GROMAQ_WELCOME_PREVIEW_PPM:-${proof_dir}/gromaq-welcome-preview.ppm}"
png_path="${GROMAQ_WELCOME_PREVIEW_PNG:-${proof_dir}/gromaq-welcome-preview.png}"
log_path="${proof_dir}/welcome-preview.log"
summary_path="${proof_dir}/summary.txt"
metrics_path="${proof_dir}/metrics.txt"
avatar_ansi="${root}/images/avatar/avatar-welcome.ansi"

mkdir -p "${proof_dir}"
rm -f "${ppm_path}" "${png_path}" "${log_path}" "${summary_path}" "${metrics_path}"

cd "${root}"

run_logged() {
  log_path="$1"
  shift
  if "$@" > "${log_path}" 2>&1; then
    cat "${log_path}"
    return 0
  fi
  status="$?"
  cat "${log_path}" >&2
  return "${status}"
}

run_logged "${log_path}" cargo run -- --welcome-preview-snapshot "${ppm_path}"

require_log_marker() {
  marker="$1"
  if ! grep -q "${marker}" "${log_path}"; then
    printf '%s\n' "error: welcome preview proof missing log marker: ${marker}" >&2
    exit 1
  fi
}

metric_value() {
  label="$1"
  sed -n "s/^${label}: //p" "${log_path}" | tail -n 1
}

require_min_metric() {
  label="$1"
  minimum="$2"
  value="$(metric_value "${label}")"
  if [ -z "${value}" ]; then
    printf '%s\n' "error: welcome preview proof missing metric: ${label}" >&2
    exit 1
  fi
  if [ "${value}" -lt "${minimum}" ]; then
    printf '%s\n' "error: ${label} ${value} below minimum ${minimum}" >&2
    exit 1
  fi
}

require_exact_metric() {
  label="$1"
  expected="$2"
  value="$(metric_value "${label}")"
  if [ "${value}" != "${expected}" ]; then
    printf '%s\n' "error: ${label} ${value:-missing} did not match ${expected}" >&2
    exit 1
  fi
}

require_ppm_dimensions() {
  path="$1"
  expected_width="$2"
  expected_height="$3"
  dimensions="$(sed -n '2p' "${path}")"
  if [ "${dimensions}" != "${expected_width} ${expected_height}" ]; then
    printf '%s\n' \
      "error: ${path} dimensions ${dimensions:-missing} did not match ${expected_width} ${expected_height}" >&2
    exit 1
  fi
}

require_avatar_rows() {
  expected="$1"
  if [ ! -s "${avatar_ansi}" ]; then
    printf '%s\n' "error: welcome preview proof missing ${avatar_ansi}" >&2
    exit 1
  fi
  avatar_rows="$(wc -l < "${avatar_ansi}" | tr -d '[:space:]')"
  if [ "${avatar_rows}" != "${expected}" ]; then
    printf '%s\n' "error: welcome avatar rows ${avatar_rows:-missing} did not match ${expected}" >&2
    exit 1
  fi
}

require_log_marker "welcome preview snapshot: ok"
require_log_marker "preset: gromaq-ghostty"
require_log_marker "frame size: 1468x820"
require_log_marker "terminal cells: 80x18"
require_avatar_rows 17

require_min_metric "high contrast text pixels" 22000
require_min_metric "avatar color pixels" 20000
require_min_metric "glyph quads" 640
require_min_metric "atlas bytes" 1
require_exact_metric "cursor quads" 0

if [ ! -s "${ppm_path}" ]; then
  printf '%s\n' "error: welcome preview proof did not write ${ppm_path}" >&2
  exit 1
fi
if [ "$(head -c 2 "${ppm_path}")" != "P6" ]; then
  printf '%s\n' "error: ${ppm_path} is not a binary PPM artifact" >&2
  exit 1
fi
require_ppm_dimensions "${ppm_path}" 1468 820

write_metric() {
  label="$1"
  printf '%s=%s\n' "${label}" "$(metric_value "${label}")"
}

write_static_metric() {
  label="$1"
  value="$2"
  printf '%s=%s\n' "${label}" "${value}"
}

{
  write_metric "frame size"
  write_metric "terminal cells"
  write_static_metric "avatar rows" "${avatar_rows}"
  write_metric "high contrast text pixels"
  write_metric "avatar color pixels"
  write_metric "glyph quads"
  write_metric "cursor quads"
  write_metric "atlas bytes"
} > "${metrics_path}"

if command -v sips >/dev/null 2>&1; then
  sips -s format png "${ppm_path}" --out "${png_path}" >/dev/null
  if [ ! -s "${png_path}" ]; then
    printf '%s\n' "error: welcome preview PNG conversion did not write ${png_path}" >&2
    exit 1
  fi
  printf '%s\n' "welcome preview PNG: ${png_path}"
else
  printf '%s\n' "welcome preview PNG: skipped (sips not available)"
fi

{
  printf '%s\n' "Welcome preview proof: ok"
  printf '%s\n' "PPM artifact: ${ppm_path}"
  if [ -s "${png_path}" ]; then
    printf '%s\n' "PNG artifact: ${png_path}"
  fi
  if [ -s "${metrics_path}" ]; then
    while IFS= read -r line; do
      printf '%s\n' "Metric: ${line}"
    done < "${metrics_path}"
  fi
  printf '%s\n' "Proof log: ${log_path}"
} | tee "${summary_path}"
