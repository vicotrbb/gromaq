#!/bin/sh
set -eu

root="$(CDPATH= cd "$(dirname "$0")/.." && pwd)"
package="${GROMAQ_PACKAGE:-gromaq}"
app_name="${GROMAQ_APP_NAME:-Gromaq}"
version="${GROMAQ_VERSION:-v0.2.1}"
version_without_prefix="${version#v}"
proof_root="${GROMAQ_MANUAL_TMUX_PROOF_ROOT:-${root}/target/macos-native-tmux-manual-proof}"
default_installed_app="${root}/target/macos-release-install-proof/Applications/${app_name}.app"
default_dist_app="${root}/target/dist/${app_name}.app"
default_debug_binary="${root}/target/debug/gromaq"
app_path="${GROMAQ_MANUAL_TMUX_APP:-}"
binary_path="${GROMAQ_MANUAL_TMUX_BINARY:-}"
open_manager_on_start="${GROMAQ_MANUAL_TMUX_OPEN_ON_START:-false}"
preflight_only="${GROMAQ_MANUAL_TMUX_PREFLIGHT_ONLY:-false}"
config_path="${proof_root}/gromaq-native-tmux-manual.toml"
shell_path="${proof_root}/manual-tmux-shell.sh"
summary_path="${proof_root}/summary.txt"
git_status_path="${proof_root}/git-status.txt"
session="gromaq-manual-tmux-$$"
kill_session="${session}-kill"
workspace_session="${session}-workspace"
started_session="${session}-started"
initial_windows_path="${proof_root}/tmux-initial-windows.txt"
initial_panes_path="${proof_root}/tmux-initial-panes.txt"
started_session_exists_path="${proof_root}/tmux-start-session-exists.txt"
workspace_exists_path="${proof_root}/tmux-workspace-session-exists.txt"
post_windows_path="${proof_root}/tmux-post-windows.txt"
post_panes_path="${proof_root}/tmux-post-panes.txt"
kill_absent_path="${proof_root}/tmux-kill-session-absent.txt"
live_window_started_at_path="${proof_root}/live-window-started-at.txt"
live_window_finished_at_path="${proof_root}/live-window-finished-at.txt"

cleanup() {
  tmux kill-session -t "${session}" >/dev/null 2>&1 || true
  tmux kill-session -t "${kill_session}" >/dev/null 2>&1 || true
  tmux kill-session -t "${workspace_session}" >/dev/null 2>&1 || true
  tmux kill-session -t "${started_session}" >/dev/null 2>&1 || true
}

if [ "$(uname -s)" != "Darwin" ]; then
  printf '%s\n' "error: macOS native tmux manual proof must run on Darwin." >&2
  exit 1
fi

if ! command -v tmux >/dev/null 2>&1; then
  printf '%s\n' "error: tmux is required for macOS native tmux manual proof." >&2
  exit 1
fi
tmux_version="$(tmux -V)"
head_sha="$(git -C "${root}" rev-parse --short HEAD)"
git_branch="$(git -C "${root}" branch --show-current)"
git_dirty="clean"
if [ -n "$(git -C "${root}" status --short)" ]; then
  git_dirty="dirty"
fi
startup_marker="tmux Cmd/Ctrl+Shift+T"
old_startup_marker="keyboard, mouse, paste"
binary_markers_path="${proof_root}/tmux-binary-markers.txt"
window_smoke_stdout_path="${proof_root}/tmux-window-smoke.stdout"
window_smoke_stderr_path="${proof_root}/tmux-window-smoke.stderr"
runtime_tmux_ui_smoke_stdout_path="${proof_root}/tmux-runtime-tmux-ui-smoke.stdout"
runtime_tmux_ui_smoke_stderr_path="${proof_root}/tmux-runtime-tmux-ui-smoke.stderr"
manager_reference_ppm_path="${proof_root}/tmux-manager-reference.ppm"
manager_reference_png_path="${proof_root}/tmux-manager-reference.png"
manager_reference_stdout_path="${proof_root}/tmux-manager-reference.stdout"
manager_reference_stderr_path="${proof_root}/tmux-manager-reference.stderr"
native_window_proof_attempts="${GROMAQ_NATIVE_WINDOW_PROOF_ATTEMPTS:-3}"
native_window_attempt_log_path="${proof_root}/native-window-proof-attempts.txt"
manual_checklist_path="${proof_root}/manual-checklist.txt"

case "${open_manager_on_start}" in
  true | false) ;;
  *)
    printf '%s\n' "error: GROMAQ_MANUAL_TMUX_OPEN_ON_START must be true or false." >&2
    exit 1
    ;;
esac

case "${preflight_only}" in
  true | false) ;;
  *)
    printf '%s\n' "error: GROMAQ_MANUAL_TMUX_PREFLIGHT_ONLY must be true or false." >&2
    exit 1
    ;;
esac

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

launch_mode="app"
if [ -n "${binary_path}" ]; then
  launch_mode="binary"
elif [ -z "${app_path}" ]; then
  if [ -d "${default_installed_app}" ]; then
    app_path="${default_installed_app}"
  elif [ -d "${default_dist_app}" ]; then
    app_path="${default_dist_app}"
  else
    launch_mode="binary"
    binary_path="${default_debug_binary}"
  fi
fi

if [ "${launch_mode}" = "binary" ] && [ "${binary_path}" = "${default_debug_binary}" ]; then
  (
    cd "${root}"
    cargo build
  )
fi

if [ "${launch_mode}" = "app" ]; then
  if ! command -v open >/dev/null 2>&1; then
    printf '%s\n' "error: open is required for app-bundle tmux manual proof." >&2
    exit 1
  fi
  executable="${app_path}/Contents/MacOS/${package}"
else
  executable="${binary_path}"
fi

if [ ! -x "${executable}" ]; then
  printf '%s\n' "error: app executable not found: ${executable}" >&2
  printf '%s\n' "Set GROMAQ_MANUAL_TMUX_APP=/path/to/Gromaq.app to choose the installed app." >&2
  printf '%s\n' "Set GROMAQ_MANUAL_TMUX_BINARY=/path/to/gromaq to choose a debug binary." >&2
  exit 1
fi

rm -rf "${proof_root}"
mkdir -p "${proof_root}"
: > "${native_window_attempt_log_path}"

actual_version="$("${executable}" --version 2>/dev/null || true)"
expected_version="gromaq ${version_without_prefix}"
if [ "${actual_version}" != "${expected_version}" ]; then
  printf '%s\n' "error: app executable reported '${actual_version}', expected '${expected_version}'." >&2
  exit 1
fi

if ! strings "${executable}" | grep -F "${startup_marker}" > "${binary_markers_path}"; then
  printf '%s\n' "error: ${executable} does not contain current startup marker '${startup_marker}'." >&2
  exit 1
fi

if strings "${executable}" | grep -F "${old_startup_marker}" >> "${binary_markers_path}"; then
  printf '%s\n' "error: unexpected old startup marker '${old_startup_marker}' remains in ${executable}." >&2
  exit 1
fi

trap cleanup EXIT INT TERM
cleanup
git -C "${root}" status --short --branch > "${git_status_path}"
printf '%s\n' "${launch_mode}" > "${proof_root}/launch-mode.txt"
printf '%s\n' "${open_manager_on_start}" > "${proof_root}/open-manager-on-start.txt"

tmux new-session -d -s "${session}" -n code
tmux split-window -t "${session}:0" -h
tmux new-session -d -s "${kill_session}" -n disposable
printf '%s\n' "${session}" > "${proof_root}/tmux-session.txt"
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

run_native_window_proof_with_retry \
  "selected executable window smoke" \
  "${window_smoke_stdout_path}" \
  "${window_smoke_stderr_path}" \
  "${executable}" --window-smoke

if ! grep -F "presented frame limit: 3" "${window_smoke_stdout_path}" >/dev/null ||
  ! grep -F "frames presented: 3" "${window_smoke_stdout_path}" >/dev/null ||
  ! grep -F "terminal cells:" "${window_smoke_stdout_path}" >/dev/null; then
  printf '%s\n' "error: selected executable window smoke did not present three settled frames." >&2
  exit 1
fi
if ! grep -F "default startup content checked: true" "${window_smoke_stdout_path}" >/dev/null; then
  printf '%s\n' "error: selected executable window smoke did not prove current startup content." >&2
  exit 1
fi
if ! grep -F "default startup marker: tmux Cmd/Ctrl+Shift+T" "${window_smoke_stdout_path}" >/dev/null; then
  printf '%s\n' "error: selected executable window smoke did not prove current startup marker." >&2
  exit 1
fi
if ! grep -F "tmux status strip rendered: true" "${window_smoke_stdout_path}" >/dev/null; then
  printf '%s\n' "error: selected executable window smoke did not render the tmux status strip." >&2
  exit 1
fi
if ! grep -F "tmux status pane command rendered: true" "${window_smoke_stdout_path}" >/dev/null; then
  printf '%s\n' "error: selected executable window smoke did not render the tmux active pane command." >&2
  exit 1
fi
if ! grep -F "tmux manager panel rendered: true" "${window_smoke_stdout_path}" >/dev/null; then
  printf '%s\n' "error: selected executable window smoke did not render the tmux manager panel." >&2
  exit 1
fi

(
  cd "${root}"
  cargo run -- --runtime-tmux-ui-smoke > "${runtime_tmux_ui_smoke_stdout_path}" 2> "${runtime_tmux_ui_smoke_stderr_path}"
)

if ! grep -F "outside attach target checked: true" "${runtime_tmux_ui_smoke_stdout_path}" >/dev/null; then
  printf '%s\n' "error: runtime tmux UI smoke did not prove concrete outside-tmux attach guidance." >&2
  exit 1
fi

run_native_window_proof_with_retry \
  "selected executable manager reference" \
  "${manager_reference_stdout_path}" \
  "${manager_reference_stderr_path}" \
  "${executable}" --window-tmux-manager-snapshot "${manager_reference_ppm_path}"

if ! grep -F "default startup content checked: true" "${manager_reference_stdout_path}" >/dev/null; then
  printf '%s\n' "error: selected executable manager reference did not prove current startup content." >&2
  exit 1
fi
if ! grep -F "default startup marker: tmux Cmd/Ctrl+Shift+T" "${manager_reference_stdout_path}" >/dev/null; then
  printf '%s\n' "error: selected executable manager reference did not prove current startup marker." >&2
  exit 1
fi
if ! grep -F "tmux status strip rendered: true" "${manager_reference_stdout_path}" >/dev/null; then
  printf '%s\n' "error: selected executable manager reference did not render the tmux status strip." >&2
  exit 1
fi
if ! grep -F "tmux status pane command rendered: true" "${manager_reference_stdout_path}" >/dev/null; then
  printf '%s\n' "error: selected executable manager reference did not render the tmux active pane command." >&2
  exit 1
fi
if ! grep -F "tmux manager panel rendered: true" "${manager_reference_stdout_path}" >/dev/null; then
  printf '%s\n' "error: selected executable manager reference did not render the tmux manager panel." >&2
  exit 1
fi
if ! grep -F "terminal cells:" "${manager_reference_stdout_path}" >/dev/null; then
  printf '%s\n' "error: selected executable manager reference did not report terminal cells." >&2
  exit 1
fi
for count_label in "tmux manager sessions:" "tmux manager windows:" "tmux manager panes:"; do
  if ! grep -F "${count_label}" "${manager_reference_stdout_path}" >/dev/null; then
    printf '%s\n' "error: selected executable manager reference did not report ${count_label}" >&2
    exit 1
  fi
done

if command -v sips >/dev/null 2>&1; then
  sips -s format png "${manager_reference_ppm_path}" --out "${manager_reference_png_path}" >/dev/null
fi

cat > "${manual_checklist_path}" <<EOF
macOS native tmux manual checklist
launch mode: ${launch_mode}
open manager on start: ${open_manager_on_start}
target session: ${session}
disposable kill target: ${kill_session}
workspace preset session: ${workspace_session}
expected started session: ${started_session}

- Confirm the persistent tmux status strip is visible. Type exactly: status-strip-visible
- Press Cmd/Ctrl+Shift+T if the manager is closed, then confirm the real manager is visible. Type exactly: manager-visible
- Confirm the manager is a real panel, not a tiny hint or palette. Type exactly: not-hint
- Confirm sessions/windows/panes/current target/pane command text are visible. Type exactly: state-visible
- Navigate with arrows or h/j/k/l and click at least one session/window/pane/action/workspace row. Type exactly: navigation-checked
- Verify prompt/right-prompt layout remains legible with the tmux surfaces visible. Type exactly: right-prompt-legible
- Confirm the UI feels like native terminal control, not web UI. Type exactly: native-control-plane
- Start a tmux session named ${started_session} from the UI. Type exactly: start-session
- Attach ${session} from the UI so active-target actions can run. Type exactly: attach-session
- Run one safe split-pane action from the UI. Type exactly: safe-action
- Create a tmux window from the UI. Type exactly: new-window
- Press r and verify the manager refreshes without sending shell input. Type exactly: refresh-checked
- Use q to run kill-session, verify inline confirmation appears, and only confirm against ${kill_session}. Type exactly: destructive-confirmation
- Launch the configured workspace preset and verify it is listed with root/windows summary. Type exactly: workspace-launched
- Close the manager and verify normal shell input still reaches this prompt. Type exactly: normal-shell-input
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
    printf '%s\n' "macOS native tmux manual preflight: ok"
    printf '%s\n' "manual preflight only requested; skipping live app-window launch"
    printf '%s\n' "live app-window proof: not run"
    printf '%s\n' "Launch mode: ${launch_mode}"
    printf '%s\n' "Open manager on start: ${open_manager_on_start}"
    printf '%s\n' "App: ${app_path:-none}"
    printf '%s\n' "Executable: ${executable}"
    printf '%s\n' "Version: ${actual_version}"
    printf '%s\n' "tmux: ${tmux_version}"
    printf '%s\n' "git HEAD: ${head_sha}"
    printf '%s\n' "git branch: ${git_branch}"
    printf '%s\n' "git dirty: ${git_dirty}"
    printf '%s\n' "git-status.txt: ${git_status_path}"
    printf '%s\n' "manual checklist: ${manual_checklist_path}"
    printf '%s\n' "manual-checklist.txt: ${manual_checklist_path}"
    printf '%s\n' "tmux-binary-markers.txt: ${startup_marker}"
    printf '%s\n' "tmux-window-smoke.stdout: ${window_smoke_stdout_path}"
    printf '%s\n' "tmux-window-smoke.stderr: ${window_smoke_stderr_path}"
    printf '%s\n' "native window proof attempts: ${native_window_proof_attempts}"
    printf '%s\n' "native window proof attempt log: ${native_window_attempt_log_path}"
    printf '%s\n' "native-window-proof-attempts.txt: ${native_window_attempt_log_path}"
    grep -F "default startup marker: tmux Cmd/Ctrl+Shift+T" "${window_smoke_stdout_path}"
    grep -F "presented frame limit: 3" "${window_smoke_stdout_path}"
    grep -F "frames presented: 3" "${window_smoke_stdout_path}"
    grep -F "terminal cells:" "${window_smoke_stdout_path}"
    grep -F "tmux status strip rendered: true" "${window_smoke_stdout_path}"
    grep -F "tmux status pane command rendered: true" "${window_smoke_stdout_path}"
    grep -F "tmux manager panel rendered: true" "${window_smoke_stdout_path}"
    printf '%s\n' "tmux-runtime-tmux-ui-smoke.stdout: ${runtime_tmux_ui_smoke_stdout_path}"
    printf '%s\n' "tmux-runtime-tmux-ui-smoke.stderr: ${runtime_tmux_ui_smoke_stderr_path}"
    grep -F "outside attach target checked: true" "${runtime_tmux_ui_smoke_stdout_path}"
    printf '%s\n' "tmux-manager-reference.ppm: ${manager_reference_ppm_path}"
    if [ -f "${manager_reference_png_path}" ]; then
      printf '%s\n' "tmux-manager-reference.png: ${manager_reference_png_path}"
    fi
    printf '%s\n' "tmux-manager-reference.stdout: ${manager_reference_stdout_path}"
    printf '%s\n' "tmux-manager-reference.stderr: ${manager_reference_stderr_path}"
    grep -F "default startup marker: tmux Cmd/Ctrl+Shift+T" "${manager_reference_stdout_path}"
    grep -F "terminal cells:" "${manager_reference_stdout_path}"
    grep -F "tmux manager sessions:" "${manager_reference_stdout_path}"
    grep -F "tmux manager windows:" "${manager_reference_stdout_path}"
    grep -F "tmux manager panes:" "${manager_reference_stdout_path}"
    printf '%s\n' "session: ${session}"
    printf '%s\n' "tmux-initial-windows.txt: ${initial_windows_path}"
    printf '%s\n' "initial tmux windows: ${initial_window_count}"
    printf '%s\n' "tmux-initial-panes.txt: ${initial_panes_path}"
    printf '%s\n' "initial tmux panes: ${initial_pane_count}"
    printf '%s\n' "Proof root: ${proof_root}"
  } | tee "${summary_path}"
  exit 0
fi

cat > "${shell_path}" <<EOF
#!/bin/sh
set -eu
printf '%s\n' ready > "${proof_root}/ready"
printf '%s\n' "Native tmux manager manual proof"
printf '%s\n' "Target session: ${session}"
printf '%s\n' "Disposable kill target: ${kill_session}"
printf '%s\n' "Workspace preset session: ${workspace_session}"
printf '%s\n' "Expected started session: ${started_session}"
printf '%s\n' "Use the visible Gromaq window for each step, then type the exact token requested here."
printf '%s\n' "Confirm the persistent tmux status strip is visible. Type exactly: status-strip-visible"
IFS= read -r status_strip_visible
printf '%s\n' "\${status_strip_visible}" > "${proof_root}/tmux-status-strip-visible.txt"
printf '%s\n' "Press Cmd/Ctrl+Shift+T if the manager is closed, then confirm the real manager is visible. Type exactly: manager-visible"
IFS= read -r manager_visible
printf '%s\n' "\${manager_visible}" > "${proof_root}/tmux-manager-visible.txt"
printf '%s\n' "Confirm the manager is a real panel, not a tiny hint or palette. Type exactly: not-hint"
IFS= read -r manager_not_hint
printf '%s\n' "\${manager_not_hint}" > "${proof_root}/tmux-manager-not-hint.txt"
printf '%s\n' "Confirm sessions/windows/panes/current target/pane command text are visible. Type exactly: state-visible"
IFS= read -r state_visible
printf '%s\n' "\${state_visible}" > "${proof_root}/tmux-state-visible.txt"
printf '%s\n' "Navigate with arrows or h/j/k/l and click at least one session/window/pane/action/workspace row. Type exactly: navigation-checked"
IFS= read -r navigation_checked
printf '%s\n' "\${navigation_checked}" > "${proof_root}/tmux-navigation-checked.txt"
printf '%s\n' "Verify prompt/right-prompt layout remains legible with the tmux surfaces visible. Type exactly: right-prompt-legible"
IFS= read -r right_prompt_legible
printf '%s\n' "\${right_prompt_legible}" > "${proof_root}/tmux-right-prompt-legible.txt"
printf '%s\n' "Confirm the UI feels like native terminal control, not web UI. Type exactly: native-control-plane"
IFS= read -r native_control_plane
printf '%s\n' "\${native_control_plane}" > "${proof_root}/tmux-native-control-plane.txt"
printf '%s\n' "Start a tmux session named ${started_session} from the UI. Type exactly: start-session"
IFS= read -r start_session
printf '%s\n' "\${start_session}" > "${proof_root}/tmux-start-session.txt"
printf '%s\n' "Attach ${session} from the UI so active-target actions can run. Type exactly: attach-session"
IFS= read -r attach_session
printf '%s\n' "\${attach_session}" > "${proof_root}/tmux-attach-session.txt"
printf '%s\n' "Run one safe split-pane action from the UI. Type exactly: safe-action"
IFS= read -r safe_action
printf '%s\n' "\${safe_action}" > "${proof_root}/tmux-safe-action.txt"
printf '%s\n' "Create a tmux window from the UI. Type exactly: new-window"
IFS= read -r new_window
printf '%s\n' "\${new_window}" > "${proof_root}/tmux-new-window.txt"
printf '%s\n' "Press r and verify the manager refreshes without sending shell input. Type exactly: refresh-checked"
IFS= read -r refresh_checked
printf '%s\n' "\${refresh_checked}" > "${proof_root}/tmux-refresh-checked.txt"
printf '%s\n' "Use q to run kill-session, verify inline confirmation appears, and only confirm against ${kill_session}. Type exactly: destructive-confirmation"
IFS= read -r destructive_confirmation
printf '%s\n' "\${destructive_confirmation}" > "${proof_root}/tmux-destructive-confirmation.txt"
printf '%s\n' "Launch the configured workspace preset and verify it is listed with root/windows summary. Type exactly: workspace-launched"
IFS= read -r workspace_visible
printf '%s\n' "\${workspace_visible}" > "${proof_root}/tmux-workspace-visible.txt"
printf '%s\n' "Close the manager and verify normal shell input still reaches this prompt. Type exactly: normal-shell-input"
IFS= read -r normal_shell_input
printf '%s\n' "\${normal_shell_input}" > "${proof_root}/tmux-normal-shell-input.txt"
printf '%s\n' done > "${proof_root}/done"
EOF
chmod 755 "${shell_path}"

cat > "${config_path}" <<EOF
[terminal]
cols = 120
rows = 34
scrollback_lines = 2000

[shell]
program = "${shell_path}"
args = []
cwd = "${root}"

[welcome]
enabled = false

[tmux]
enabled = true
show_status_strip = true
open_manager_on_start = ${open_manager_on_start}

[tmux.workspaces.manual]
session = "${workspace_session}"
root = "${root}"

[[tmux.workspaces.manual.windows]]
name = "code"
panes = ["printf 'manual workspace code pane\\n'; sleep 60"]

[[tmux.workspaces.manual.windows]]
name = "test"
panes = ["printf 'manual workspace test pane\\n'; sleep 60"]
EOF

printf '%s\n' "macOS native tmux manual proof"
printf '%s\n' "Launch mode: ${launch_mode}"
printf '%s\n' "Open manager on start: ${open_manager_on_start}"
printf '%s\n' "App: ${app_path:-none}"
printf '%s\n' "Binary: ${executable}"
printf '%s\n' "Version: ${actual_version}"
printf '%s\n' "tmux: ${tmux_version}"
printf '%s\n' "git HEAD: ${head_sha}"
printf '%s\n' "git branch: ${git_branch}"
printf '%s\n' "git dirty: ${git_dirty}"
printf '%s\n' "Target session: ${session}"
printf '%s\n' "Disposable kill target: ${kill_session}"
printf '%s\n' "Workspace preset session: ${workspace_session}"
printf '%s\n' "Expected started session: ${started_session}"
printf '%s\n' "Manual checklist: ${manual_checklist_path}"
printf '%s\n' "A Gromaq window will open with tmux UI enabled and open_manager_on_start=${open_manager_on_start}."
printf '%s\n' "Follow the prompts inside the Gromaq terminal window exactly."

date -u +%Y-%m-%dT%H:%M:%SZ > "${live_window_started_at_path}"
if [ "${launch_mode}" = "app" ]; then
  open -W -n \
    -o "${proof_root}/open.stdout" \
    --stderr "${proof_root}/open.stderr" \
    "${app_path}" \
    --args --config "${config_path}"
else
  "${executable}" --config "${config_path}" \
    > "${proof_root}/binary.stdout" \
    2> "${proof_root}/binary.stderr"
fi
date -u +%Y-%m-%dT%H:%M:%SZ > "${live_window_finished_at_path}"

manager_visible="$(cat "${proof_root}/tmux-manager-visible.txt" 2>/dev/null || true)"
manager_not_hint="$(cat "${proof_root}/tmux-manager-not-hint.txt" 2>/dev/null || true)"
status_strip_visible="$(cat "${proof_root}/tmux-status-strip-visible.txt" 2>/dev/null || true)"
state_visible="$(cat "${proof_root}/tmux-state-visible.txt" 2>/dev/null || true)"
navigation_checked="$(cat "${proof_root}/tmux-navigation-checked.txt" 2>/dev/null || true)"
right_prompt_legible="$(cat "${proof_root}/tmux-right-prompt-legible.txt" 2>/dev/null || true)"
native_control_plane="$(cat "${proof_root}/tmux-native-control-plane.txt" 2>/dev/null || true)"
start_session="$(cat "${proof_root}/tmux-start-session.txt" 2>/dev/null || true)"
attach_session="$(cat "${proof_root}/tmux-attach-session.txt" 2>/dev/null || true)"
safe_action="$(cat "${proof_root}/tmux-safe-action.txt" 2>/dev/null || true)"
new_window="$(cat "${proof_root}/tmux-new-window.txt" 2>/dev/null || true)"
refresh_checked="$(cat "${proof_root}/tmux-refresh-checked.txt" 2>/dev/null || true)"
destructive_confirmation="$(cat "${proof_root}/tmux-destructive-confirmation.txt" 2>/dev/null || true)"
workspace_visible="$(cat "${proof_root}/tmux-workspace-visible.txt" 2>/dev/null || true)"
normal_shell_input="$(cat "${proof_root}/tmux-normal-shell-input.txt" 2>/dev/null || true)"

if [ "${status_strip_visible}" != "status-strip-visible" ]; then
  printf '%s\n' "error: expected status-strip-visible confirmation, got '${status_strip_visible}'." >&2
  exit 1
fi
if [ "${manager_visible}" != "manager-visible" ]; then
  printf '%s\n' "error: expected manager-visible confirmation, got '${manager_visible}'." >&2
  exit 1
fi
if [ "${manager_not_hint}" != "not-hint" ]; then
  printf '%s\n' "error: expected not-hint confirmation, got '${manager_not_hint}'." >&2
  exit 1
fi
if [ "${state_visible}" != "state-visible" ]; then
  printf '%s\n' "error: expected state-visible confirmation, got '${state_visible}'." >&2
  exit 1
fi
if [ "${navigation_checked}" != "navigation-checked" ]; then
  printf '%s\n' "error: expected navigation-checked confirmation, got '${navigation_checked}'." >&2
  exit 1
fi
if [ "${right_prompt_legible}" != "right-prompt-legible" ]; then
  printf '%s\n' "error: expected right-prompt-legible confirmation, got '${right_prompt_legible}'." >&2
  exit 1
fi
if [ "${native_control_plane}" != "native-control-plane" ]; then
  printf '%s\n' "error: expected native-control-plane confirmation, got '${native_control_plane}'." >&2
  exit 1
fi
if [ "${start_session}" != "start-session" ]; then
  printf '%s\n' "error: expected start-session confirmation, got '${start_session}'." >&2
  exit 1
fi
if [ "${attach_session}" != "attach-session" ]; then
  printf '%s\n' "error: expected attach-session confirmation, got '${attach_session}'." >&2
  exit 1
fi
if [ "${safe_action}" != "safe-action" ]; then
  printf '%s\n' "error: expected safe-action confirmation, got '${safe_action}'." >&2
  exit 1
fi
if [ "${new_window}" != "new-window" ]; then
  printf '%s\n' "error: expected new-window confirmation, got '${new_window}'." >&2
  exit 1
fi
if [ "${refresh_checked}" != "refresh-checked" ]; then
  printf '%s\n' "error: expected refresh-checked confirmation, got '${refresh_checked}'." >&2
  exit 1
fi
if [ "${destructive_confirmation}" != "destructive-confirmation" ]; then
  printf '%s\n' "error: expected destructive-confirmation confirmation, got '${destructive_confirmation}'." >&2
  exit 1
fi
if [ "${workspace_visible}" != "workspace-launched" ]; then
  printf '%s\n' "error: expected workspace-launched confirmation, got '${workspace_visible}'." >&2
  exit 1
fi
if [ "${normal_shell_input}" != "normal-shell-input" ]; then
  printf '%s\n' "error: expected normal-shell-input confirmation, got '${normal_shell_input}'." >&2
  exit 1
fi

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

if ! tmux has-session -t "${workspace_session}" >/dev/null 2>&1; then
  printf '%s\n' "false" > "${workspace_exists_path}"
  printf '%s\n' "error: expected workspace session ${workspace_session} to exist after workspace proof." >&2
  exit 1
fi
printf '%s\n' "true" > "${workspace_exists_path}"

{
  printf '%s\n' "macOS native tmux manual proof: ok"
  printf '%s\n' "live app-window proof: completed"
  printf '%s\n' "Launch mode: ${launch_mode}"
  printf '%s\n' "Open manager on start: ${open_manager_on_start}"
  printf '%s\n' "App: ${app_path:-none}"
  printf '%s\n' "Executable: ${executable}"
  printf '%s\n' "Version: ${actual_version}"
  printf '%s\n' "tmux: ${tmux_version}"
  printf '%s\n' "git HEAD: ${head_sha}"
  printf '%s\n' "git branch: ${git_branch}"
  printf '%s\n' "git dirty: ${git_dirty}"
  printf '%s\n' "git-status.txt: ${git_status_path}"
  printf '%s\n' "manual checklist: ${manual_checklist_path}"
  printf '%s\n' "manual-checklist.txt: ${manual_checklist_path}"
  printf '%s\n' "tmux-binary-markers.txt: ${startup_marker}"
  printf '%s\n' "tmux-window-smoke.stdout: ${window_smoke_stdout_path}"
  printf '%s\n' "tmux-window-smoke.stderr: ${window_smoke_stderr_path}"
  printf '%s\n' "native window proof attempts: ${native_window_proof_attempts}"
  printf '%s\n' "native window proof attempt log: ${native_window_attempt_log_path}"
  printf '%s\n' "native-window-proof-attempts.txt: ${native_window_attempt_log_path}"
  grep -F "default startup marker: tmux Cmd/Ctrl+Shift+T" "${window_smoke_stdout_path}"
  grep -F "presented frame limit: 3" "${window_smoke_stdout_path}"
  grep -F "frames presented: 3" "${window_smoke_stdout_path}"
  grep -F "terminal cells:" "${window_smoke_stdout_path}"
  grep -F "tmux status strip rendered: true" "${window_smoke_stdout_path}"
  grep -F "tmux status pane command rendered: true" "${window_smoke_stdout_path}"
  grep -F "tmux manager panel rendered: true" "${window_smoke_stdout_path}"
  printf '%s\n' "tmux-runtime-tmux-ui-smoke.stdout: ${runtime_tmux_ui_smoke_stdout_path}"
  printf '%s\n' "tmux-runtime-tmux-ui-smoke.stderr: ${runtime_tmux_ui_smoke_stderr_path}"
  grep -F "outside attach target checked: true" "${runtime_tmux_ui_smoke_stdout_path}"
  printf '%s\n' "tmux-manager-reference.ppm: ${manager_reference_ppm_path}"
  if [ -f "${manager_reference_png_path}" ]; then
    printf '%s\n' "tmux-manager-reference.png: ${manager_reference_png_path}"
  fi
  printf '%s\n' "tmux-manager-reference.stdout: ${manager_reference_stdout_path}"
  printf '%s\n' "tmux-manager-reference.stderr: ${manager_reference_stderr_path}"
  grep -F "default startup marker: tmux Cmd/Ctrl+Shift+T" "${manager_reference_stdout_path}"
  grep -F "terminal cells:" "${manager_reference_stdout_path}"
  grep -F "tmux manager sessions:" "${manager_reference_stdout_path}"
  grep -F "tmux manager windows:" "${manager_reference_stdout_path}"
  grep -F "tmux manager panes:" "${manager_reference_stdout_path}"
  printf '%s\n' "session: ${session}"
  printf '%s\n' "tmux-initial-windows.txt: ${initial_windows_path}"
  printf '%s\n' "initial tmux windows: ${initial_window_count}"
  printf '%s\n' "tmux-initial-panes.txt: ${initial_panes_path}"
  printf '%s\n' "initial tmux panes: ${initial_pane_count}"
  printf '%s\n' "started-session: ${started_session}"
  printf '%s\n' "kill-session target: ${kill_session}"
  printf '%s\n' "workspace-session: ${workspace_session}"
  printf '%s\n' "tmux-status-strip-visible.txt: ${status_strip_visible}"
  printf '%s\n' "tmux-manager-visible.txt: ${manager_visible}"
  printf '%s\n' "tmux-manager-not-hint.txt: ${manager_not_hint}"
  printf '%s\n' "tmux-state-visible.txt: ${state_visible}"
  printf '%s\n' "tmux-navigation-checked.txt: ${navigation_checked}"
  printf '%s\n' "tmux-right-prompt-legible.txt: ${right_prompt_legible}"
  printf '%s\n' "tmux-native-control-plane.txt: ${native_control_plane}"
  printf '%s\n' "tmux-start-session.txt: ${start_session}"
  printf '%s\n' "tmux-start-session-exists.txt: ${started_session_exists_path}"
  printf '%s\n' "started-session exists: true"
  printf '%s\n' "tmux-attach-session.txt: ${attach_session}"
  printf '%s\n' "tmux-safe-action.txt: ${safe_action}"
  printf '%s\n' "tmux-new-window.txt: ${new_window}"
  printf '%s\n' "tmux-post-windows.txt: ${post_windows_path}"
  printf '%s\n' "post tmux windows: ${post_window_count}"
  printf '%s\n' "tmux-post-panes.txt: ${post_panes_path}"
  printf '%s\n' "post tmux panes: ${post_pane_count}"
  printf '%s\n' "live-window-started-at.txt: $(cat "${live_window_started_at_path}")"
  printf '%s\n' "live-window-finished-at.txt: $(cat "${live_window_finished_at_path}")"
  printf '%s\n' "tmux-refresh-checked.txt: ${refresh_checked}"
  printf '%s\n' "tmux-destructive-confirmation.txt: ${destructive_confirmation}"
  printf '%s\n' "tmux-kill-session-absent.txt: ${kill_absent_path}"
  printf '%s\n' "kill-session absent: true"
  printf '%s\n' "tmux-workspace-visible.txt: ${workspace_visible}"
  printf '%s\n' "tmux-workspace-session-exists.txt: ${workspace_exists_path}"
  printf '%s\n' "workspace-session exists: true"
  printf '%s\n' "tmux-normal-shell-input.txt: ${normal_shell_input}"
  printf '%s\n' "launch-mode.txt: ${launch_mode}"
  printf '%s\n' "open-manager-on-start.txt: ${open_manager_on_start}"
  printf '%s\n' "Proof root: ${proof_root}"
} | tee "${summary_path}"
