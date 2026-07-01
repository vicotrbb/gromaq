#!/bin/sh
set -eu

if [ "$(uname -s)" != "Darwin" ]; then
  printf '%s\n' "error: macOS app identity proof requires Darwin." >&2
  exit 1
fi

for tool in cargo osascript lsappinfo pgrep open; do
  if ! command -v "${tool}" >/dev/null 2>&1; then
    printf '%s\n' "error: ${tool} is required for macOS app identity proof." >&2
    exit 1
  fi
done

root="$(CDPATH= cd "$(dirname "$0")/.." && pwd)"
proof_dir="${GROMAQ_MACOS_IDENTITY_PROOF_DIR:-${root}/target/macos-app-identity-proof}"
dist_dir="${GROMAQ_MACOS_IDENTITY_DIST_DIR:-${proof_dir}/dist}"
bundle_id="${GROMAQ_BUNDLE_ID:-dev.gromaq.Gromaq}"
app_name="${GROMAQ_APP_NAME:-Gromaq}"
binary_path="${GROMAQ_BINARY_PATH:-${root}/target/debug/gromaq}"
app_path="${dist_dir}/${app_name}.app"
open_stdout="${proof_dir}/open.stdout"
open_stderr="${proof_dir}/open.stderr"
package_log="${proof_dir}/package.log"
system_events_log="${proof_dir}/system-events.txt"
lsappinfo_log="${proof_dir}/lsappinfo.txt"
pgrep_log="${proof_dir}/pgrep.txt"
summary_path="${proof_dir}/summary.txt"
launch_delay="${GROMAQ_MACOS_IDENTITY_DELAY_SECONDS:-1}"

mkdir -p "${proof_dir}" "${dist_dir}"
rm -f \
  "${open_stdout}" \
  "${open_stderr}" \
  "${package_log}" \
  "${system_events_log}" \
  "${lsappinfo_log}" \
  "${pgrep_log}" \
  "${summary_path}"

(
  cd "${root}"
  cargo build
) > "${proof_dir}/cargo-build.log" 2>&1

GROMAQ_BINARY_PATH="${binary_path}" \
  GROMAQ_DIST_DIR="${dist_dir}" \
  "${root}/scripts/package-macos-app.sh" > "${package_log}" 2>&1

open -W -n \
  -o "${open_stdout}" \
  --stderr "${open_stderr}" \
  "${app_path}" \
  --args --window-screenshot-smoke &
open_pid="$!"

sleep "${launch_delay}"

osascript -e "tell application \"System Events\" to get the name of every process whose bundle identifier is \"${bundle_id}\"" > "${system_events_log}" 2>&1
if ! grep -q "gromaq" "${system_events_log}"; then
  wait "${open_pid}" || true
  printf '%s\n' "error: System Events did not find a running ${bundle_id} process; see ${system_events_log}." >&2
  exit 1
fi

app_info_id="$(lsappinfo find "bundleid=${bundle_id}" | head -n 1 || true)"
if [ -z "${app_info_id}" ]; then
  wait "${open_pid}" || true
  printf '%s\n' "error: lsappinfo did not find bundle id ${bundle_id}." >&2
  exit 1
fi

lsappinfo info -only bundleid,name "${app_info_id}" > "${lsappinfo_log}" 2>&1
if ! grep -q "\"CFBundleIdentifier\"=\"${bundle_id}\"" "${lsappinfo_log}"; then
  wait "${open_pid}" || true
  printf '%s\n' "error: lsappinfo did not report CFBundleIdentifier=${bundle_id}; see ${lsappinfo_log}." >&2
  exit 1
fi
if ! grep -q "\"LSDisplayName\"=\"${app_name}\"" "${lsappinfo_log}"; then
  wait "${open_pid}" || true
  printf '%s\n' "error: lsappinfo did not report LSDisplayName=${app_name}; see ${lsappinfo_log}." >&2
  exit 1
fi

pgrep -fl "${app_path}/Contents/MacOS/gromaq" > "${pgrep_log}" 2>&1
if ! grep -q "Contents/MacOS/gromaq --window-screenshot-smoke" "${pgrep_log}"; then
  wait "${open_pid}" || true
  printf '%s\n' "error: bundled Contents/MacOS/gromaq process not found; see ${pgrep_log}." >&2
  exit 1
fi

open_status=0
wait "${open_pid}" || open_status="$?"
if [ "${open_status}" -ne 0 ]; then
  printf '%s\n' "error: packaged app LaunchServices smoke exited with ${open_status}; see ${open_stdout} and ${open_stderr}." >&2
  exit "${open_status}"
fi

if ! grep -q "window screenshot smoke: ok" "${open_stdout}"; then
  if grep -q "surface occluded" "${open_stderr}"; then
    printf '%s\n' "error: packaged app smoke never presented a surface frame because the macOS window was fully surface occluded; see ${open_stderr}." >&2
    exit 1
  fi
  printf '%s\n' "error: packaged app smoke did not report success; see ${open_stdout}." >&2
  exit 1
fi
if ! grep -q "presented frame limit: 900" "${open_stdout}"; then
  printf '%s\n' "error: packaged app smoke did not use the bounded 900-frame screenshot host; see ${open_stdout}." >&2
  exit 1
fi
for required_smoke_marker in \
  "default startup content checked: true" \
  "tmux status strip rendered: true" \
  "tmux manager panel rendered: true"
do
  if ! grep -F "${required_smoke_marker}" "${open_stdout}" >/dev/null; then
    printf '%s\n' "error: packaged app smoke did not report ${required_smoke_marker}; see ${open_stdout}." >&2
    exit 1
  fi
done

{
  printf '%s\n' "macOS app identity proof: ok"
  printf '%s\n' "App bundle: ${app_path}"
  printf '%s\n' "System Events proof: ${system_events_log}"
  printf '%s\n' "lsappinfo proof: ${lsappinfo_log}"
  printf '%s\n' "process proof: ${pgrep_log}"
  printf '%s\n' "LaunchServices smoke stdout: ${open_stdout}"
  printf '%s\n' "LaunchServices smoke stderr: ${open_stderr}"
  printf '%s\n' "Package log: ${package_log}"
} | tee "${summary_path}"
