#!/bin/sh
set -eu

root="$(CDPATH= cd "$(dirname "$0")/.." && pwd)"
proof_dir="${GROMAQ_144HZ_WINDOW_PERF_PROOF_DIR:-${root}/target/144hz-window-perf-proof}"
log_path="${proof_dir}/window-perf.log"
summary_path="${proof_dir}/summary.txt"
minimum_refresh_mhz=144000

mkdir -p "${proof_dir}"
rm -f "${log_path}" "${log_path}.tmp" "${summary_path}"

cd "${root}"
printf '%s\n' "$ cargo run -- --window-perf-smoke" | tee "${log_path}"
set +e
cargo run -- --window-perf-smoke >"${log_path}.tmp" 2>&1
status="$?"
set -e
cat "${log_path}.tmp" | tee -a "${log_path}"
rm -f "${log_path}.tmp"

if [ "${status}" -ne 0 ]; then
  printf '%s\n' "error: window perf smoke failed with exit ${status}; see ${log_path}" >&2
  exit "${status}"
fi

metric_value() {
  label="$1"
  sed -n "s/^${label}: //p" "${log_path}" | tail -n 1
}

require_log_marker() {
  marker="$1"
  if ! grep -Fqx "${marker}" "${log_path}"; then
    printf '%s\n' "error: 144Hz window perf proof missing log marker: ${marker}" >&2
    exit 1
  fi
}

monitor_refresh_mhz="$(metric_value "monitor refresh mhz")"
case "${monitor_refresh_mhz}" in
  ''|*[!0-9]*)
    printf '%s\n' "error: 144Hz window perf proof requires numeric monitor refresh mhz; saw ${monitor_refresh_mhz:-missing}" >&2
    exit 1
    ;;
esac

if [ "${monitor_refresh_mhz}" -lt "${minimum_refresh_mhz}" ]; then
  printf '%s\n' "error: 144Hz window perf proof requires monitor refresh mhz >= ${minimum_refresh_mhz}; saw ${monitor_refresh_mhz}" >&2
  exit 1
fi

require_log_marker "window perf smoke: ok"
require_log_marker "target fps: 144"
require_log_marker "frame interval target fps: 144"
require_log_marker "frame interval target limited by monitor: false"
require_log_marker "dropped frames: 0"
require_log_marker "frame pacing accepted: true"

{
  printf '%s\n' "144Hz window perf proof: ok"
  printf '%s\n' "Proof log: ${log_path}"
  printf '%s\n' "Monitor refresh mhz: ${monitor_refresh_mhz}"
  printf '%s\n' "Minimum refresh mhz: ${minimum_refresh_mhz}"
} | tee "${summary_path}"
