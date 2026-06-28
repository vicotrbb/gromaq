#!/bin/sh
set -eu

root="$(CDPATH= cd "$(dirname "$0")/.." && pwd)"
proof_dir="${GROMAQ_THEME_PREVIEW_PROOF_DIR:-${root}/target/theme-preview-proof}"
default_ppm="${proof_dir}/gromaq-theme-preview.ppm"
default_png="${proof_dir}/gromaq-theme-preview.png"
default_log="${proof_dir}/theme-preview.log"
config_path="${proof_dir}/gromaq-theme-preview-config.toml"
config_ppm="${proof_dir}/gromaq-theme-preview-config.ppm"
config_png="${proof_dir}/gromaq-theme-preview-config.png"
config_log="${proof_dir}/theme-preview-config.log"
summary_path="${proof_dir}/summary.txt"
metrics_path="${proof_dir}/metrics.txt"

mkdir -p "${proof_dir}"
rm -f \
  "${default_ppm}" "${default_png}" "${default_log}" \
  "${config_path}" "${config_ppm}" "${config_png}" "${config_log}" \
  "${summary_path}" "${metrics_path}"

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

run_logged "${default_log}" cargo run -- --theme-preview-snapshot "${default_ppm}"

printf '%s\n' \
  '[theme]' \
  'preset = "gromaq-graphite"' \
  'background_opacity = 0.75' \
  'cursor_opacity = 0.5' \
  'selection_opacity = 0.25' > "${config_path}"
run_logged "${config_log}" cargo run -- --theme-preview-config "${config_path}" "${config_ppm}"

require_log_marker() {
  log_path="$1"
  marker="$2"
  if ! grep -q "${marker}" "${log_path}"; then
    printf '%s\n' "error: theme preview proof missing log marker: ${marker}" >&2
    exit 1
  fi
}

metric_value() {
  log_path="$1"
  label="$2"
  sed -n "s/^${label}: //p" "${log_path}" | tail -n 1
}

require_min_metric() {
  log_path="$1"
  label="$2"
  minimum="$3"
  value="$(metric_value "${log_path}" "${label}")"
  if [ -z "${value}" ]; then
    printf '%s\n' "error: theme preview proof missing metric: ${label}" >&2
    exit 1
  fi
  if [ "${value}" -lt "${minimum}" ]; then
    printf '%s\n' "error: ${label} ${value} below minimum ${minimum}" >&2
    exit 1
  fi
}

require_exact_metric() {
  log_path="$1"
  label="$2"
  expected="$3"
  value="$(metric_value "${log_path}" "${label}")"
  if [ "${value}" != "${expected}" ]; then
    printf '%s\n' "error: ${label} ${value:-missing} did not match ${expected}" >&2
    exit 1
  fi
}

require_ppm_artifact() {
  ppm_path="$1"
  if [ ! -s "${ppm_path}" ]; then
    printf '%s\n' "error: theme preview proof did not write ${ppm_path}" >&2
    exit 1
  fi
  if [ "$(head -c 2 "${ppm_path}")" != "P6" ]; then
    printf '%s\n' "error: ${ppm_path} is not a binary PPM artifact" >&2
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

check_theme_preview_log() {
  log_path="$1"
  preset="$2"
  background_opacity="$3"
  cursor_opacity="$4"
  selection_opacity="$5"

  require_log_marker "${log_path}" "theme preview snapshot: ok"
  require_log_marker "${log_path}" "preset: ${preset}"
  require_log_marker "${log_path}" "frame size: 1036x292"
  require_exact_metric "${log_path}" "font size px" 32
  require_exact_metric "${log_path}" "cell width px" 18
  require_exact_metric "${log_path}" "line height px" 44
  require_exact_metric "${log_path}" "background opacity percent" "${background_opacity}"
  require_exact_metric "${log_path}" "cursor opacity percent" "${cursor_opacity}"
  require_exact_metric "${log_path}" "selection opacity percent" "${selection_opacity}"
  require_exact_metric "${log_path}" "surface padding px" 14
  require_exact_metric "${log_path}" "cell spacing px" 0
  require_min_metric "${log_path}" "high contrast text pixels" 9000
  require_min_metric "${log_path}" "selection pixels" 10000
  require_min_metric "${log_path}" "cursor pixels" 700
  require_min_metric "${log_path}" "prepared quads" 100
  require_exact_metric "${log_path}" "background quads" 1
  require_exact_metric "${log_path}" "cursor quads" 1
  require_min_metric "${log_path}" "atlas bytes" 1
}

check_theme_preview_log "${default_log}" "gromaq-ghostty" 100 100 100
check_theme_preview_log "${config_log}" "gromaq-graphite" 75 50 25
require_ppm_artifact "${default_ppm}"
require_ppm_artifact "${config_ppm}"
require_ppm_dimensions "${default_ppm}" 1036 292
require_ppm_dimensions "${config_ppm}" 1036 292

write_metric() {
  scope="$1"
  log_path="$2"
  label="$3"
  printf '%s.%s=%s\n' "${scope}" "${label}" "$(metric_value "${log_path}" "${label}")"
}

{
  for label in \
    "frame size" \
    "font size px" \
    "background opacity percent" \
    "cursor opacity percent" \
    "selection opacity percent" \
    "high contrast text pixels" \
    "selection pixels" \
    "cursor pixels" \
    "prepared quads" \
    "atlas bytes"; do
    write_metric "default" "${default_log}" "${label}"
    write_metric "configured" "${config_log}" "${label}"
  done
} > "${metrics_path}"

if command -v sips >/dev/null 2>&1; then
  sips -s format png "${default_ppm}" --out "${default_png}" >/dev/null
  sips -s format png "${config_ppm}" --out "${config_png}" >/dev/null
  if [ ! -s "${default_png}" ] || [ ! -s "${config_png}" ]; then
    printf '%s\n' "error: theme preview PNG conversion did not write expected artifacts" >&2
    exit 1
  fi
  printf '%s\n' "theme preview PNG: ${default_png}"
  printf '%s\n' "configured theme preview PNG: ${config_png}"
else
  printf '%s\n' "theme preview PNG: skipped (sips not available)"
fi

{
  printf '%s\n' "Theme preview proof: ok"
  printf '%s\n' "Default PPM artifact: ${default_ppm}"
  if [ -s "${default_png}" ]; then
    printf '%s\n' "Default PNG artifact: ${default_png}"
  fi
  printf '%s\n' "Configured PPM artifact: ${config_ppm}"
  if [ -s "${config_png}" ]; then
    printf '%s\n' "Configured PNG artifact: ${config_png}"
  fi
  if [ -s "${metrics_path}" ]; then
    while IFS= read -r line; do
      printf '%s\n' "Metric: ${line}"
    done < "${metrics_path}"
  fi
  printf '%s\n' "Default proof log: ${default_log}"
  printf '%s\n' "Configured proof log: ${config_log}"
  printf '%s\n' "Configured proof config: ${config_path}"
} | tee "${summary_path}"
