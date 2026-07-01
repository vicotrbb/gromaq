#!/bin/sh
set -eu

root="$(CDPATH= cd "$(dirname "$0")/.." && pwd)"
proof_root="${GROMAQ_DEFAULT_CARGO_TMUX_PROOF_ROOT:-${root}/target/macos-native-tmux-default-cargo-run-proof}"
preflight_only="${GROMAQ_DEFAULT_CARGO_TMUX_PREFLIGHT_ONLY:-false}"
summary_path="${proof_root}/summary.txt"
git_status_path="${proof_root}/git-status.txt"
session="gromaq-default-cargo-tmux-$$"
kill_session="${session}-kill"
started_session="${session}-started"
initial_windows_path="${proof_root}/tmux-default-cargo-run-initial-windows.txt"
initial_panes_path="${proof_root}/tmux-default-cargo-run-initial-panes.txt"
started_session_exists_path="${proof_root}/tmux-default-cargo-run-start-session-exists.txt"
post_windows_path="${proof_root}/tmux-default-cargo-run-post-windows.txt"
post_panes_path="${proof_root}/tmux-default-cargo-run-post-panes.txt"
kill_absent_path="${proof_root}/tmux-default-cargo-run-kill-session-absent.txt"
live_window_started_at_path="${proof_root}/tmux-default-cargo-run-live-window-started-at.txt"
live_window_finished_at_path="${proof_root}/tmux-default-cargo-run-live-window-finished-at.txt"

cleanup() {
  tmux kill-session -t "${session}" >/dev/null 2>&1 || true
  tmux kill-session -t "${kill_session}" >/dev/null 2>&1 || true
  tmux kill-session -t "${started_session}" >/dev/null 2>&1 || true
}

if [ "$(uname -s)" != "Darwin" ]; then
  printf '%s\n' "error: macOS native tmux default cargo run proof must run on Darwin." >&2
  exit 1
fi

if ! command -v tmux >/dev/null 2>&1; then
  printf '%s\n' "error: tmux is required for macOS native tmux default cargo run proof." >&2
  exit 1
fi
case "${preflight_only}" in
  true | false) ;;
  *)
    printf '%s\n' "error: GROMAQ_DEFAULT_CARGO_TMUX_PREFLIGHT_ONLY must be true or false." >&2
    exit 1
    ;;
esac
tmux_version="$(tmux -V)"
head_sha="$(git -C "${root}" rev-parse --short HEAD)"
git_branch="$(git -C "${root}" branch --show-current)"
git_dirty="clean"
if [ -n "$(git -C "${root}" status --short)" ]; then
  git_dirty="dirty"
fi
startup_marker="tmux Cmd/Ctrl+Shift+T"
old_startup_marker="keyboard, mouse, paste"
binary_path="${root}/target/debug/gromaq"
binary_markers_path="${proof_root}/tmux-default-cargo-run-binary-markers.txt"
window_smoke_stdout_path="${proof_root}/tmux-default-cargo-run-window-smoke.stdout"
window_smoke_stderr_path="${proof_root}/tmux-default-cargo-run-window-smoke.stderr"
runtime_tmux_ui_smoke_stdout_path="${proof_root}/tmux-default-cargo-run-runtime-tmux-ui-smoke.stdout"
runtime_tmux_ui_smoke_stderr_path="${proof_root}/tmux-default-cargo-run-runtime-tmux-ui-smoke.stderr"
manager_reference_ppm_path="${proof_root}/tmux-default-cargo-run-manager-reference.ppm"
manager_reference_png_path="${proof_root}/tmux-default-cargo-run-manager-reference.png"
manager_reference_stdout_path="${proof_root}/tmux-default-cargo-run-manager-reference.stdout"
manager_reference_stderr_path="${proof_root}/tmux-default-cargo-run-manager-reference.stderr"
native_window_proof_attempts="${GROMAQ_NATIVE_WINDOW_PROOF_ATTEMPTS:-3}"
native_window_attempt_log_path="${proof_root}/native-window-proof-attempts.txt"
manual_checklist_path="${proof_root}/manual-checklist.txt"

rm -rf "${proof_root}"
mkdir -p "${proof_root}"
: > "${native_window_attempt_log_path}"
git -C "${root}" status --short --branch > "${git_status_path}"
trap cleanup EXIT INT TERM
cleanup

(
  cd "${root}"
  cargo build > "${proof_root}/cargo-build.stdout" 2> "${proof_root}/cargo-build.stderr"
)

if [ ! -x "${binary_path}" ]; then
  printf '%s\n' "error: expected cargo build debug binary at ${binary_path}." >&2
  exit 1
fi

if ! strings "${binary_path}" | grep -F "${startup_marker}" > "${binary_markers_path}"; then
  printf '%s\n' "error: ${binary_path} does not contain current startup marker '${startup_marker}'." >&2
  exit 1
fi

if strings "${binary_path}" | grep -F "${old_startup_marker}" >> "${binary_markers_path}"; then
  printf '%s\n' "error: unexpected old startup marker '${old_startup_marker}' remains in ${binary_path}." >&2
  exit 1
fi

case "${native_window_proof_attempts}" in
  '' | *[!0-9]*)
    printf '%s\n' "error: GROMAQ_NATIVE_WINDOW_PROOF_ATTEMPTS must be a positive integer." >&2
    exit 1
    ;;
  0)
    printf '%s\n' "error: GROMAQ_NATIVE_WINDOW_PROOF_ATTEMPTS must be greater than zero." >&2
    exit 1
    ;;
esac

run_native_window_proof_with_retry() {
  label="$1"
  stdout_path="$2"
  stderr_path="$3"
  shift 3
  attempt=1
  while [ "${attempt}" -le "${native_window_proof_attempts}" ]; do
    printf '%s\n' "native window proof attempt ${attempt}/${native_window_proof_attempts}: ${label}" >> "${native_window_attempt_log_path}"
    if "$@" > "${stdout_path}" 2> "${stderr_path}"; then
      printf '%s\n' "native window proof attempt ${attempt}/${native_window_proof_attempts}: ${label}: ok" >> "${native_window_attempt_log_path}"
      return 0
    fi
    if ! grep -Eq "surface occluded|no surface frame was presented" "${stderr_path}"; then
      printf '%s\n' "native window proof attempt ${attempt}/${native_window_proof_attempts}: ${label}: failed" >> "${native_window_attempt_log_path}"
      return 1
    fi
    if [ "${attempt}" -ge "${native_window_proof_attempts}" ]; then
      printf '%s\n' "native window proof attempt ${attempt}/${native_window_proof_attempts}: ${label}: surface occluded; attempts exhausted" >> "${native_window_attempt_log_path}"
      return 1
    fi
    printf '%s\n' "native window proof attempt ${attempt}/${native_window_proof_attempts} for ${label} hit surface occlusion; retrying." >> "${stderr_path}"
    printf '%s\n' "native window proof attempt ${attempt}/${native_window_proof_attempts}: ${label}: surface occluded; retrying" >> "${native_window_attempt_log_path}"
    attempt=$((attempt + 1))
    sleep 1
  done
  return 1
}

tmux new-session -d -s "${session}" -n code
tmux split-window -t "${session}:0" -h
tmux new-session -d -s "${kill_session}" -n disposable
if ! tmux list-windows -t "${session}" -F "#{window_id} #{window_name}" > "${initial_windows_path}"; then
  printf '%s\n' "error: could not read initial tmux windows for ${session}." >&2
  exit 1
fi
initial_window_count="$(wc -l < "${initial_windows_path}" | tr -d '[:space:]')"
if [ "${initial_window_count}" -lt 1 ]; then
  printf '%s\n' "error: expected at least 1 initial tmux window in ${session}, got ${initial_window_count}." >&2
  exit 1
fi
if ! tmux list-panes -s -t "${session}" -F "#{pane_id} #{pane_current_command}" > "${initial_panes_path}"; then
  printf '%s\n' "error: could not read initial tmux panes for ${session}." >&2
  exit 1
fi
initial_pane_count="$(wc -l < "${initial_panes_path}" | tr -d '[:space:]')"
if [ "${initial_pane_count}" -lt 2 ]; then
  printf '%s\n' "error: expected at least 2 initial tmux panes in ${session}, got ${initial_pane_count}." >&2
  exit 1
fi

(
  cd "${root}"
  run_native_window_proof_with_retry \
    "default cargo run window smoke" \
    "${window_smoke_stdout_path}" \
    "${window_smoke_stderr_path}" \
    cargo run -- --window-smoke
)

if ! grep -F "default startup content checked: true" "${window_smoke_stdout_path}" >/dev/null; then
  printf '%s\n' "error: default window smoke did not prove current startup content." >&2
  exit 1
fi
if ! grep -F "default startup marker: tmux Cmd/Ctrl+Shift+T" "${window_smoke_stdout_path}" >/dev/null; then
  printf '%s\n' "error: default window smoke did not prove current startup marker." >&2
  exit 1
fi

if ! grep -F "presented frame limit: 3" "${window_smoke_stdout_path}" >/dev/null ||
  ! grep -F "frames presented: 3" "${window_smoke_stdout_path}" >/dev/null ||
  ! grep -F "terminal cells:" "${window_smoke_stdout_path}" >/dev/null; then
  printf '%s\n' "error: default window smoke did not present three settled frames." >&2
  exit 1
fi

if ! grep -F "tmux status strip rendered: true" "${window_smoke_stdout_path}" >/dev/null; then
  printf '%s\n' "error: default window smoke did not render the tmux status strip." >&2
  exit 1
fi

if ! grep -F "tmux status pane command rendered: true" "${window_smoke_stdout_path}" >/dev/null; then
  printf '%s\n' "error: default window smoke did not render the tmux active pane command." >&2
  exit 1
fi

if ! grep -F "tmux manager panel rendered: true" "${window_smoke_stdout_path}" >/dev/null; then
  printf '%s\n' "error: default window smoke did not render the tmux manager panel." >&2
  exit 1
fi

(
  cd "${root}"
  cargo run -- --runtime-tmux-ui-smoke > "${runtime_tmux_ui_smoke_stdout_path}" 2> "${runtime_tmux_ui_smoke_stderr_path}"
)

if ! grep -F "startup manager small-grid cells: 69x17" "${runtime_tmux_ui_smoke_stdout_path}" >/dev/null; then
  printf '%s\n' "error: runtime tmux UI smoke did not prove small-grid startup manager rendering." >&2
  exit 1
fi

if ! grep -F "manager header status checked: true" "${runtime_tmux_ui_smoke_stdout_path}" >/dev/null; then
  printf '%s\n' "error: runtime tmux UI smoke did not prove manager header status rendering." >&2
  exit 1
fi

if ! grep -F "outside attach target checked: true" "${runtime_tmux_ui_smoke_stdout_path}" >/dev/null; then
  printf '%s\n' "error: runtime tmux UI smoke did not prove concrete outside-tmux attach guidance." >&2
  exit 1
fi

if ! grep -F "skipped pty handoffs checked: attach=true start=true workspace=true" "${runtime_tmux_ui_smoke_stdout_path}" >/dev/null; then
  printf '%s\n' "error: runtime tmux UI smoke did not prove skipped PTY handoffs." >&2
  exit 1
fi

if ! grep -F "workspace duplicate prevented: true" "${runtime_tmux_ui_smoke_stdout_path}" >/dev/null; then
  printf '%s\n' "error: runtime tmux UI smoke did not prove workspace duplicate prevention." >&2
  exit 1
fi

(
  cd "${root}"
  run_native_window_proof_with_retry \
    "default cargo run manager reference" \
    "${manager_reference_stdout_path}" \
    "${manager_reference_stderr_path}" \
    cargo run -- --window-tmux-manager-snapshot "${manager_reference_ppm_path}"
)

if ! grep -F "default startup content checked: true" "${manager_reference_stdout_path}" >/dev/null; then
  printf '%s\n' "error: default manager reference did not prove current startup content." >&2
  exit 1
fi
if ! grep -F "default startup marker: tmux Cmd/Ctrl+Shift+T" "${manager_reference_stdout_path}" >/dev/null; then
  printf '%s\n' "error: default manager reference did not prove current startup marker." >&2
  exit 1
fi

if ! grep -F "tmux status strip rendered: true" "${manager_reference_stdout_path}" >/dev/null; then
  printf '%s\n' "error: default manager reference did not render the tmux status strip." >&2
  exit 1
fi

if ! grep -F "tmux status pane command rendered: true" "${manager_reference_stdout_path}" >/dev/null; then
  printf '%s\n' "error: default manager reference did not render the tmux active pane command." >&2
  exit 1
fi

if ! grep -F "tmux manager panel rendered: true" "${manager_reference_stdout_path}" >/dev/null; then
  printf '%s\n' "error: default manager reference did not render the tmux manager panel." >&2
  exit 1
fi

if ! grep -F "terminal cells:" "${manager_reference_stdout_path}" >/dev/null; then
  printf '%s\n' "error: default manager reference did not report terminal cells." >&2
  exit 1
fi

for count_label in "tmux manager sessions:" "tmux manager windows:" "tmux manager panes:"; do
  if ! grep -F "${count_label}" "${manager_reference_stdout_path}" >/dev/null; then
    printf '%s\n' "error: default manager reference did not report ${count_label}" >&2
    exit 1
  fi
done

if command -v sips >/dev/null 2>&1; then
  sips -s format png "${manager_reference_ppm_path}" --out "${manager_reference_png_path}" >/dev/null
fi

cat > "${manual_checklist_path}" <<EOF
macOS native tmux default cargo run checklist
target session: ${session}
expected started session: ${started_session}
disposable kill target: ${kill_session}

- The welcome Input row says '${startup_marker}', not the old keyboard/mouse/paste copy. Type exactly: current-startup-copy
- Persistent tmux status strip is visible and legible. Type exactly: status-strip-visible
- If the manager is already visible on startup, close it with Esc.
- Cmd/Ctrl+Shift+T opens or reopens a real manager panel, not a tiny hint. Type exactly: manager-visible
- Sessions, windows, panes, current target, and pane command text are visible. Type exactly: state-visible
- Keyboard navigation changes selection. Type exactly: navigation-checked
- Start a new tmux session named ${started_session} from the UI. Type exactly: start-session
- Attach ${session} from the UI, then run a safe split-pane action. Type exactly: attach-session and safe-action
- Create a tmux window from the UI. Type exactly: new-window
- A destructive action shows inline confirmation before running. Type exactly: destructive-confirmation
- Confirm a kill-session action only against ${kill_session}. Type exactly: isolated-kill-confirmed
- Close the panel and verify normal shell input still reaches the prompt. Type exactly: shell-input
- Check prompt/right-prompt layout for legible overlap behavior. Type exactly: right-prompt-legible
- Confirm the UI felt like native terminal control, not web UI. Type exactly: native-control-plane
EOF

if ! grep -F "Cmd/Ctrl+Shift+T" "${manual_checklist_path}" >/dev/null; then
  printf '%s\n' "error: manual checklist missing current shortcut copy." >&2
  exit 1
fi
if grep -F "Control/Super Shift" "${manual_checklist_path}" >/dev/null; then
  printf '%s\n' "error: manual checklist retained stale shortcut copy." >&2
  exit 1
fi

if [ "${preflight_only}" = "true" ]; then
  {
    printf '%s\n' "macOS native tmux default cargo run preflight: ok"
    printf '%s\n' "default cargo run preflight only requested; skipping live cargo run window"
    printf '%s\n' "live app-window proof: not run"
    printf '%s\n' "tmux: ${tmux_version}"
    printf '%s\n' "git HEAD: ${head_sha}"
    printf '%s\n' "git branch: ${git_branch}"
    printf '%s\n' "git dirty: ${git_dirty}"
    printf '%s\n' "git-status.txt: ${git_status_path}"
    printf '%s\n' "manual checklist: ${manual_checklist_path}"
    printf '%s\n' "manual-checklist.txt: ${manual_checklist_path}"
    printf '%s\n' "debug binary: ${binary_path}"
    printf '%s\n' "session: ${session}"
    printf '%s\n' "tmux-default-cargo-run-initial-windows.txt: ${initial_windows_path}"
    printf '%s\n' "initial tmux windows: ${initial_window_count}"
    printf '%s\n' "tmux-default-cargo-run-initial-panes.txt: ${initial_panes_path}"
    printf '%s\n' "initial tmux panes: ${initial_pane_count}"
    printf '%s\n' "cargo-build stdout: ${proof_root}/cargo-build.stdout"
    printf '%s\n' "cargo-build stderr: ${proof_root}/cargo-build.stderr"
    printf '%s\n' "tmux-default-cargo-run-binary-markers.txt: ${startup_marker}"
    printf '%s\n' "tmux-default-cargo-run-window-smoke.stdout: ${window_smoke_stdout_path}"
    printf '%s\n' "tmux-default-cargo-run-window-smoke.stderr: ${window_smoke_stderr_path}"
    printf '%s\n' "native window proof attempts: ${native_window_proof_attempts}"
    printf '%s\n' "native window proof attempt log: ${native_window_attempt_log_path}"
    printf '%s\n' "native-window-proof-attempts.txt: ${native_window_attempt_log_path}"
    grep -F "default startup content checked: true" "${window_smoke_stdout_path}"
    grep -F "default startup marker: tmux Cmd/Ctrl+Shift+T" "${window_smoke_stdout_path}"
    grep -F "presented frame limit: 3" "${window_smoke_stdout_path}"
    grep -F "frames presented: 3" "${window_smoke_stdout_path}"
    grep -F "terminal cells:" "${window_smoke_stdout_path}"
    grep -F "tmux status strip rendered: true" "${window_smoke_stdout_path}"
    grep -F "tmux status pane command rendered: true" "${window_smoke_stdout_path}"
    grep -F "tmux manager panel rendered: true" "${window_smoke_stdout_path}"
    printf '%s\n' "tmux-default-cargo-run-runtime-tmux-ui-smoke.stdout: ${runtime_tmux_ui_smoke_stdout_path}"
    printf '%s\n' "tmux-default-cargo-run-runtime-tmux-ui-smoke.stderr: ${runtime_tmux_ui_smoke_stderr_path}"
    grep -F "startup manager small-grid cells: 69x17" "${runtime_tmux_ui_smoke_stdout_path}"
    grep -F "manager header status checked: true" "${runtime_tmux_ui_smoke_stdout_path}"
    grep -F "outside attach target checked: true" "${runtime_tmux_ui_smoke_stdout_path}"
    printf '%s\n' "tmux-default-cargo-run-manager-reference.ppm: ${manager_reference_ppm_path}"
    if [ -f "${manager_reference_png_path}" ]; then
      printf '%s\n' "tmux-default-cargo-run-manager-reference.png: ${manager_reference_png_path}"
    fi
    printf '%s\n' "tmux-default-cargo-run-manager-reference.stdout: ${manager_reference_stdout_path}"
    printf '%s\n' "tmux-default-cargo-run-manager-reference.stderr: ${manager_reference_stderr_path}"
    grep -F "default startup marker: tmux Cmd/Ctrl+Shift+T" "${manager_reference_stdout_path}"
    grep -F "terminal cells:" "${manager_reference_stdout_path}"
    printf '%s\n' "Proof root: ${proof_root}"
  } | tee "${summary_path}"
  exit 0
fi

printf '%s\n' "macOS native tmux default cargo run proof"
printf '%s\n' "tmux: ${tmux_version}"
printf '%s\n' "git HEAD: ${head_sha}"
printf '%s\n' "git branch: ${git_branch}"
printf '%s\n' "git dirty: ${git_dirty}"
printf '%s\n' "debug binary: ${binary_path}"
printf '%s\n' "Target session: ${session}"
printf '%s\n' "Expected started session: ${started_session}"
printf '%s\n' "Disposable kill target: ${kill_session}"
printf '%s\n' "Expected manager reference snapshot: ${manager_reference_ppm_path}"
if [ -f "${manager_reference_png_path}" ]; then
  printf '%s\n' "Expected manager reference PNG: ${manager_reference_png_path}"
fi
printf '%s\n' "Manual checklist: ${manual_checklist_path}"
printf '%s\n' "A default Gromaq window will open through plain cargo run."
printf '%s\n' "In that window, verify the native tmux UI against the checklist below."
printf '%s\n' "Close the Gromaq window when the checklist is complete; this script will then ask for exact confirmation tokens."
printf '%s\n' ""
printf '%s\n' "Checklist while cargo run is open:"
printf '%s\n' "- The welcome Input row says '${startup_marker}', not the old keyboard/mouse/paste copy."
printf '%s\n' "- Persistent tmux status strip is visible and legible."
printf '%s\n' "- If the manager is already visible on startup, close it with Esc."
printf '%s\n' "- Cmd/Ctrl+Shift+T opens or reopens a real manager panel, not a tiny hint."
printf '%s\n' "- Sessions, windows, panes, current target, and pane command text are visible."
printf '%s\n' "- Keyboard navigation changes selection."
printf '%s\n' "- Start a new tmux session named ${started_session} from the UI."
printf '%s\n' "- Attach ${session} from the UI, then run a safe split-pane action."
printf '%s\n' "- Create a tmux window from the UI."
printf '%s\n' "- A destructive action shows inline confirmation before running."
printf '%s\n' "- Confirm a kill-session action only against ${kill_session}."
printf '%s\n' "- Close the panel and verify normal shell input still reaches the prompt."
printf '%s\n' "- Check prompt/right-prompt layout for legible overlap behavior."

(
  cd "${root}"
  date -u +%Y-%m-%dT%H:%M:%SZ > "${live_window_started_at_path}"
  cargo run > "${proof_root}/cargo-run.stdout" 2> "${proof_root}/cargo-run.stderr"
  date -u +%Y-%m-%dT%H:%M:%SZ > "${live_window_finished_at_path}"
)

if [ ! -x "${binary_path}" ]; then
  printf '%s\n' "error: expected cargo run debug binary at ${binary_path}." >&2
  exit 1
fi

if ! grep -F "Running \`target/debug/gromaq\`" "${proof_root}/cargo-run.stderr" >/dev/null; then
  printf '%s\n' "error: cargo run stderr did not show target/debug/gromaq launch." >&2
  exit 1
fi

record_confirmation() {
  prompt="$1"
  token="$2"
  file="$3"
  printf '%s\n' "${prompt} Type exactly: ${token}"
  IFS= read -r value
  printf '%s\n' "${value}" > "${proof_root}/${file}"
  if [ "${value}" != "${token}" ]; then
    printf '%s\n' "error: expected ${token} confirmation, got '${value}'." >&2
    exit 1
  fi
}

record_confirmation "Confirm the welcome Input row used the current tmux shortcut copy." "current-startup-copy" "tmux-default-cargo-run-current-startup.txt"
record_confirmation "Confirm the persistent tmux status strip was visible and legible." "status-strip-visible" "tmux-default-cargo-run-status-strip.txt"
record_confirmation "Confirm Cmd/Ctrl+Shift+T opened or reopened a real manager panel." "manager-visible" "tmux-default-cargo-run-manager-visible.txt"
record_confirmation "Confirm the manager was not a tiny hint or palette." "not-hint" "tmux-default-cargo-run-not-hint.txt"
record_confirmation "Confirm sessions/windows/panes/current target/pane command text were visible." "state-visible" "tmux-default-cargo-run-state-visible.txt"
record_confirmation "Confirm keyboard navigation changed selection." "navigation-checked" "tmux-default-cargo-run-navigation.txt"
record_confirmation "Confirm a new tmux session named ${started_session} was started from the UI." "start-session" "tmux-default-cargo-run-start-session.txt"
record_confirmation "Confirm ${session} was attached from the UI." "attach-session" "tmux-default-cargo-run-attach-session.txt"
record_confirmation "Confirm a safe tmux action ran from the UI." "safe-action" "tmux-default-cargo-run-safe-action.txt"
record_confirmation "Confirm a tmux window was created from the UI." "new-window" "tmux-default-cargo-run-new-window.txt"
record_confirmation "Confirm a destructive action showed inline confirmation first." "destructive-confirmation" "tmux-default-cargo-run-destructive-confirmation.txt"
record_confirmation "Confirm a kill-session action was performed only against ${kill_session}." "isolated-kill-confirmed" "tmux-default-cargo-run-isolated-kill.txt"
record_confirmation "Confirm normal shell input still worked after closing the panel." "shell-input" "tmux-default-cargo-run-shell-input.txt"
record_confirmation "Confirm prompt/right-prompt layout stayed legible." "right-prompt-legible" "tmux-default-cargo-run-right-prompt.txt"
record_confirmation "Confirm the UI felt like native terminal control, not web UI." "native-control-plane" "tmux-default-cargo-run-native-control-plane.txt"

if ! tmux list-windows -t "${session}" -F "#{window_id} #{window_name}" > "${post_windows_path}"; then
  printf '%s\n' "error: could not read post-proof tmux windows for ${session}." >&2
  exit 1
fi
post_window_count="$(wc -l < "${post_windows_path}" | tr -d '[:space:]')"
if [ "${post_window_count}" -lt 2 ]; then
  printf '%s\n' "error: expected at least 2 tmux windows in ${session} after new-window proof, got ${post_window_count}." >&2
  exit 1
fi

if ! tmux list-panes -s -t "${session}" -F "#{pane_id} #{pane_current_command}" > "${post_panes_path}"; then
  printf '%s\n' "error: could not read post-proof tmux panes for ${session}." >&2
  exit 1
fi
post_pane_count="$(wc -l < "${post_panes_path}" | tr -d '[:space:]')"
if [ "${post_pane_count}" -lt 3 ]; then
  printf '%s\n' "error: expected at least 3 tmux panes in ${session} after safe split proof, got ${post_pane_count}." >&2
  exit 1
fi

if tmux has-session -t "${kill_session}" >/dev/null 2>&1; then
  printf '%s\n' "false" > "${kill_absent_path}"
  printf '%s\n' "error: expected isolated kill-session target ${kill_session} to be absent after destructive proof." >&2
  exit 1
fi
printf '%s\n' "true" > "${kill_absent_path}"

if ! tmux has-session -t "${started_session}" >/dev/null 2>&1; then
  printf '%s\n' "false" > "${started_session_exists_path}"
  printf '%s\n' "error: expected started session ${started_session} to exist after start-session proof." >&2
  exit 1
fi
printf '%s\n' "true" > "${started_session_exists_path}"

{
  printf '%s\n' "macOS native tmux default cargo run proof: ok"
  printf '%s\n' "live app-window proof: completed"
  printf '%s\n' "tmux: ${tmux_version}"
  printf '%s\n' "git HEAD: ${head_sha}"
  printf '%s\n' "git branch: ${git_branch}"
  printf '%s\n' "git dirty: ${git_dirty}"
  printf '%s\n' "git-status.txt: ${git_status_path}"
  printf '%s\n' "manual checklist: ${manual_checklist_path}"
  printf '%s\n' "manual-checklist.txt: ${manual_checklist_path}"
  printf '%s\n' "debug binary: ${binary_path}"
  printf '%s\n' "session: ${session}"
  printf '%s\n' "tmux-default-cargo-run-initial-windows.txt: ${initial_windows_path}"
  printf '%s\n' "initial tmux windows: ${initial_window_count}"
  printf '%s\n' "tmux-default-cargo-run-initial-panes.txt: ${initial_panes_path}"
  printf '%s\n' "initial tmux panes: ${initial_pane_count}"
  printf '%s\n' "started-session: ${started_session}"
  printf '%s\n' "kill-session target: ${kill_session}"
  printf '%s\n' "cargo-build stdout: ${proof_root}/cargo-build.stdout"
  printf '%s\n' "cargo-build stderr: ${proof_root}/cargo-build.stderr"
  printf '%s\n' "tmux-default-cargo-run-window-smoke.stdout: ${window_smoke_stdout_path}"
  printf '%s\n' "tmux-default-cargo-run-window-smoke.stderr: ${window_smoke_stderr_path}"
  printf '%s\n' "native window proof attempts: ${native_window_proof_attempts}"
  printf '%s\n' "native window proof attempt log: ${native_window_attempt_log_path}"
  printf '%s\n' "native-window-proof-attempts.txt: ${native_window_attempt_log_path}"
  grep -F "default startup content checked: true" "${window_smoke_stdout_path}"
  grep -F "default startup marker: tmux Cmd/Ctrl+Shift+T" "${window_smoke_stdout_path}"
  grep -F "presented frame limit: 3" "${window_smoke_stdout_path}"
  grep -F "frames presented: 3" "${window_smoke_stdout_path}"
  grep -F "terminal cells:" "${window_smoke_stdout_path}"
  grep -F "tmux status strip rendered: true" "${window_smoke_stdout_path}"
  grep -F "tmux status pane command rendered: true" "${window_smoke_stdout_path}"
  grep -F "tmux manager panel rendered: true" "${window_smoke_stdout_path}"
  printf '%s\n' "tmux-default-cargo-run-runtime-tmux-ui-smoke.stdout: ${runtime_tmux_ui_smoke_stdout_path}"
  printf '%s\n' "tmux-default-cargo-run-runtime-tmux-ui-smoke.stderr: ${runtime_tmux_ui_smoke_stderr_path}"
  grep -F "startup manager small-grid cells: 69x17" "${runtime_tmux_ui_smoke_stdout_path}"
  grep -F "manager header status checked: true" "${runtime_tmux_ui_smoke_stdout_path}"
  grep -F "outside attach target checked: true" "${runtime_tmux_ui_smoke_stdout_path}"
  printf '%s\n' "tmux-default-cargo-run-manager-reference.ppm: ${manager_reference_ppm_path}"
  if [ -f "${manager_reference_png_path}" ]; then
    printf '%s\n' "tmux-default-cargo-run-manager-reference.png: ${manager_reference_png_path}"
  fi
  printf '%s\n' "tmux-default-cargo-run-manager-reference.stdout: ${manager_reference_stdout_path}"
  printf '%s\n' "tmux-default-cargo-run-manager-reference.stderr: ${manager_reference_stderr_path}"
  grep -F "default startup marker: tmux Cmd/Ctrl+Shift+T" "${manager_reference_stdout_path}"
  grep -F "terminal cells:" "${manager_reference_stdout_path}"
  printf '%s\n' "cargo-run stdout: ${proof_root}/cargo-run.stdout"
  printf '%s\n' "cargo-run stderr: ${proof_root}/cargo-run.stderr"
  printf '%s\n' "tmux-default-cargo-run-binary-markers.txt: ${startup_marker}"
  printf '%s\n' "tmux-default-cargo-run-current-startup.txt: current-startup-copy"
  printf '%s\n' "tmux-default-cargo-run-status-strip.txt: status-strip-visible"
  printf '%s\n' "tmux-default-cargo-run-manager-visible.txt: manager-visible"
  printf '%s\n' "tmux-default-cargo-run-not-hint.txt: not-hint"
  printf '%s\n' "tmux-default-cargo-run-state-visible.txt: state-visible"
  printf '%s\n' "tmux-default-cargo-run-navigation.txt: navigation-checked"
  printf '%s\n' "tmux-default-cargo-run-start-session.txt: start-session"
  printf '%s\n' "tmux-default-cargo-run-start-session-exists.txt: ${started_session_exists_path}"
  printf '%s\n' "started-session exists: true"
  printf '%s\n' "tmux-default-cargo-run-attach-session.txt: attach-session"
  printf '%s\n' "tmux-default-cargo-run-safe-action.txt: safe-action"
  printf '%s\n' "tmux-default-cargo-run-new-window.txt: new-window"
  printf '%s\n' "tmux-default-cargo-run-post-windows.txt: ${post_windows_path}"
  printf '%s\n' "post tmux windows: ${post_window_count}"
  printf '%s\n' "tmux-default-cargo-run-post-panes.txt: ${post_panes_path}"
  printf '%s\n' "post tmux panes: ${post_pane_count}"
  printf '%s\n' "tmux-default-cargo-run-live-window-started-at.txt: $(cat "${live_window_started_at_path}")"
  printf '%s\n' "tmux-default-cargo-run-live-window-finished-at.txt: $(cat "${live_window_finished_at_path}")"
  printf '%s\n' "tmux-default-cargo-run-destructive-confirmation.txt: destructive-confirmation"
  printf '%s\n' "tmux-default-cargo-run-isolated-kill.txt: isolated-kill-confirmed"
  printf '%s\n' "tmux-default-cargo-run-kill-session-absent.txt: ${kill_absent_path}"
  printf '%s\n' "kill-session absent: true"
  printf '%s\n' "tmux-default-cargo-run-shell-input.txt: shell-input"
  printf '%s\n' "tmux-default-cargo-run-right-prompt.txt: right-prompt-legible"
  printf '%s\n' "tmux-default-cargo-run-native-control-plane.txt: native-control-plane"
  printf '%s\n' "Proof root: ${proof_root}"
} | tee "${summary_path}"
