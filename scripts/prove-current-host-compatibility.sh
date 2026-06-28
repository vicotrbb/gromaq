#!/bin/sh
set -eu

root="$(CDPATH= cd "$(dirname "$0")/.." && pwd)"
proof_dir="${GROMAQ_COMPATIBILITY_PROOF_DIR:-${root}/target/compatibility-proof}"

mkdir -p "${proof_dir}"

inventory="${proof_dir}/tool-inventory.txt"
summary="${proof_dir}/summary.txt"
: >"${inventory}"
printf '%s\n' "Current-host compatibility proof" | tee -a "${inventory}"
printf 'timestamp_utc=%s\n' "$(date -u '+%Y-%m-%dT%H:%M:%SZ')" | tee -a "${inventory}"

tools_present=0
tools_missing=0
for tool in bash zsh fish nu vim nvim tmux less top htop btop ssh kubectl; do
  if command -v "${tool}" >/dev/null 2>&1; then
    tools_present=$((tools_present + 1))
    printf '%s=%s\n' "${tool}" "$(command -v "${tool}")" | tee -a "${inventory}"
  else
    tools_missing=$((tools_missing + 1))
    printf '%s=missing\n' "${tool}" | tee -a "${inventory}"
  fi
done

for tool in ${GROMAQ_REQUIRED_COMPAT_TOOLS:-}; do
  if ! command -v "${tool}" >/dev/null 2>&1; then
    printf '%s\n' "error: required compatibility tool missing: ${tool}" >&2
    printf 'required_%s=missing\n' "${tool}" | tee -a "${inventory}"
    exit 1
  fi
  printf 'required_%s=present\n' "${tool}" | tee -a "${inventory}"
done

run_and_capture() {
  name="$1"
  shift
  log="${proof_dir}/${name}.log"
  tmp="${log}.tmp"

  printf '$ %s\n' "$*" | tee "${log}"
  set +e
  "$@" >"${tmp}" 2>&1
  status="$?"
  set -e
  cat "${tmp}" | tee -a "${log}"
  rm -f "${tmp}"
  if [ "${status}" -ne 0 ]; then
    printf '%s\n' "error: ${name} failed with exit ${status}; see ${log}" >&2
    exit "${status}"
  fi
}

cd "${root}"
run_and_capture pty cargo test --test pty -- --nocapture
run_and_capture runtime-tool-workflow cargo run -- --runtime-tool-workflow-smoke

runtime_log="${proof_dir}/runtime-tool-workflow.log"
extract_runtime_metric() {
  label="$1"
  value="$(sed -n "s/^${label}: //p" "${runtime_log}" | tail -n 1)"
  if [ -z "${value}" ]; then
    printf '%s\n' "error: runtime tool workflow log missing metric: ${label}" >&2
    exit 1
  fi
  printf '%s' "${value}"
}

runtime_tool_workflow_checked="$(extract_runtime_metric "tools checked")"
runtime_tool_workflow_passed="$(extract_runtime_metric "passed")"
runtime_tool_workflow_skipped="$(extract_runtime_metric "skipped")"
runtime_tool_workflow_failed="$(extract_runtime_metric "failed")"

{
  printf '%s\n' "Current-host compatibility proof: ok"
  printf 'proof_dir=%s\n' "${proof_dir}"
  printf 'tools_present=%s\n' "${tools_present}"
  printf 'tools_missing=%s\n' "${tools_missing}"
  printf 'runtime_tool_workflow_checked=%s\n' "${runtime_tool_workflow_checked}"
  printf 'runtime_tool_workflow_passed=%s\n' "${runtime_tool_workflow_passed}"
  printf 'runtime_tool_workflow_skipped=%s\n' "${runtime_tool_workflow_skipped}"
  printf 'runtime_tool_workflow_failed=%s\n' "${runtime_tool_workflow_failed}"
  printf 'inventory=%s\n' "${inventory}"
  printf 'pty_log=%s\n' "${proof_dir}/pty.log"
  printf 'runtime_tool_workflow_log=%s\n' "${runtime_log}"
} | tee "${summary}"
