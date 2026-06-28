#!/bin/sh
set -eu

root="$(CDPATH= cd "$(dirname "$0")/.." && pwd)"
repo="${GROMAQ_GITHUB_REPO:-vicotrbb/gromaq}"
workflow="${GROMAQ_CI_WORKFLOW:-ci.yml}"
head_sha="${GROMAQ_CI_HEAD_SHA:-$(git -C "${root}" rev-parse HEAD)}"
short_head_sha="$(printf '%s' "${head_sha}" | cut -c 1-7)"
run_id="${GROMAQ_CI_RUN_ID:-}"
proof_dir="${GROMAQ_CI_COMPATIBILITY_PROOF_DIR:-${root}/target/ci-compatibility-proof}"
macos_artifact="gromaq-current-host-compatibility-proof"
linux_artifact="gromaq-linux-compatibility-proof"
summary_path="${proof_dir}/summary.txt"

if ! command -v gh >/dev/null 2>&1; then
  printf '%s\n' "error: gh is required to download CI compatibility artifacts." >&2
  exit 1
fi

if [ -z "${run_id}" ]; then
  run_id="$(
    gh run list \
      --repo "${repo}" \
      --workflow "${workflow}" \
      --commit "${head_sha}" \
      --json databaseId,headSha,status,conclusion \
      --jq ".[] | select(.headSha == \"${head_sha}\" and .status == \"completed\" and .conclusion == \"success\") | .databaseId" |
      head -n 1
  )"
fi

if [ -z "${run_id}" ]; then
  printf '%s\n' "error: no successful completed CI run found for ${head_sha}; push the commit and wait for CI, or set GROMAQ_CI_RUN_ID." >&2
  exit 1
fi

rm -rf "${proof_dir}"
mkdir -p "${proof_dir}/${macos_artifact}" "${proof_dir}/${linux_artifact}"

gh run download "${run_id}" \
  --repo "${repo}" \
  --name "${macos_artifact}" \
  --dir "${proof_dir}/${macos_artifact}"
gh run download "${run_id}" \
  --repo "${repo}" \
  --name "${linux_artifact}" \
  --dir "${proof_dir}/${linux_artifact}"

require_summary_marker() {
  summary="$1"
  marker="$2"
  if ! grep -Fqx "${marker}" "${summary}"; then
    printf '%s\n' "error: ${summary} missing marker: ${marker}" >&2
    exit 1
  fi
}

verify_host_summary() {
  artifact="$1"
  expected_os_marker="$2"
  summary="${proof_dir}/${artifact}/summary.txt"
  if [ ! -f "${summary}" ]; then
    printf '%s\n' "error: ${artifact} did not include summary.txt" >&2
    exit 1
  fi
  require_summary_marker "${summary}" "Current-host compatibility proof: ok"
  require_summary_marker "${summary}" "${expected_os_marker}"
  require_summary_marker "${summary}" "runtime_tool_workflow_failed=0"
  require_summary_marker "${summary}" "git_dirty=false"
  require_summary_marker "${summary}" "git_commit=${short_head_sha}"
  if ! grep -Eq '^pty_tests_passed=[1-9][0-9]*$' "${summary}"; then
    printf '%s\n' "error: ${summary} missing nonzero pty_tests_passed marker" >&2
    exit 1
  fi
}

verify_host_summary "${macos_artifact}" "host_os=Darwin"
verify_host_summary "${linux_artifact}" "host_os=Linux"

{
  printf '%s\n' "CI compatibility artifact proof: ok"
  printf '%s\n' "Repository: ${repo}"
  printf '%s\n' "Workflow: ${workflow}"
  printf '%s\n' "Run id: ${run_id}"
  printf '%s\n' "Head SHA: ${head_sha}"
  printf '%s\n' "Short head SHA: ${short_head_sha}"
  printf '%s\n' "macOS artifact: ${proof_dir}/${macos_artifact}/summary.txt"
  printf '%s\n' "Linux artifact: ${proof_dir}/${linux_artifact}/summary.txt"
} | tee "${summary_path}"
