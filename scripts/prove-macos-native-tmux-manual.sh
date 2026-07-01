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
config_path="${proof_root}/gromaq-native-tmux-manual.toml"
shell_path="${proof_root}/manual-tmux-shell.sh"
summary_path="${proof_root}/summary.txt"
session="gromaq-manual-tmux-$$"
kill_session="${session}-kill"
workspace_session="${session}-workspace"

cleanup() {
  tmux kill-session -t "${session}" >/dev/null 2>&1 || true
  tmux kill-session -t "${kill_session}" >/dev/null 2>&1 || true
  tmux kill-session -t "${workspace_session}" >/dev/null 2>&1 || true
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

case "${open_manager_on_start}" in
  true | false) ;;
  *)
    printf '%s\n' "error: GROMAQ_MANUAL_TMUX_OPEN_ON_START must be true or false." >&2
    exit 1
    ;;
esac

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

actual_version="$("${executable}" --version 2>/dev/null || true)"
expected_version="gromaq ${version_without_prefix}"
if [ "${actual_version}" != "${expected_version}" ]; then
  printf '%s\n' "error: app executable reported '${actual_version}', expected '${expected_version}'." >&2
  exit 1
fi

rm -rf "${proof_root}"
mkdir -p "${proof_root}"
trap cleanup EXIT INT TERM
cleanup
printf '%s\n' "${launch_mode}" > "${proof_root}/launch-mode.txt"
printf '%s\n' "${open_manager_on_start}" > "${proof_root}/open-manager-on-start.txt"

tmux new-session -d -s "${session}" -n code
tmux split-window -t "${session}:0" -h
tmux new-session -d -s "${kill_session}" -n disposable
printf '%s\n' "${session}" > "${proof_root}/tmux-session.txt"

cat > "${shell_path}" <<EOF
#!/bin/sh
set -eu
printf '%s\n' ready > "${proof_root}/ready"
printf '%s\n' "Native tmux manager manual proof"
printf '%s\n' "Target session: ${session}"
printf '%s\n' "Disposable kill target: ${kill_session}"
printf '%s\n' "Workspace preset session: ${workspace_session}"
printf '%s\n' "Use the visible Gromaq window for each step, then type the exact token requested here."
printf '%s\n' "Confirm the persistent tmux status strip is visible. Type exactly: status-strip-visible"
IFS= read -r status_strip_visible
printf '%s\n' "\${status_strip_visible}" > "${proof_root}/tmux-status-strip-visible.txt"
printf '%s\n' "Press Control/Super Shift+T if the manager is closed, then confirm the real manager is visible. Type exactly: manager-visible"
IFS= read -r manager_visible
printf '%s\n' "\${manager_visible}" > "${proof_root}/tmux-manager-visible.txt"
printf '%s\n' "Navigate with arrows or h/j/k/l and click at least one session/window/pane/action/workspace row. Type exactly: navigation-checked"
IFS= read -r navigation_checked
printf '%s\n' "\${navigation_checked}" > "${proof_root}/tmux-navigation-checked.txt"
printf '%s\n' "Verify prompt/right-prompt layout remains legible with the tmux surfaces visible. Type exactly: right-prompt-legible"
IFS= read -r right_prompt_legible
printf '%s\n' "\${right_prompt_legible}" > "${proof_root}/tmux-right-prompt-legible.txt"
printf '%s\n' "Start a named tmux session from the UI. Type exactly: start-session"
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
printf '%s\n' "Use q or another destructive shortcut, verify inline confirmation appears, and only confirm against ${kill_session}. Type exactly: destructive-confirmation"
IFS= read -r destructive_confirmation
printf '%s\n' "\${destructive_confirmation}" > "${proof_root}/tmux-destructive-confirmation.txt"
printf '%s\n' "Launch the configured workspace preset or verify it is listed with root/windows summary. Type exactly: workspace-visible"
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
printf '%s\n' "Target session: ${session}"
printf '%s\n' "Disposable kill target: ${kill_session}"
printf '%s\n' "Workspace preset session: ${workspace_session}"
printf '%s\n' "A Gromaq window will open with tmux UI enabled and open_manager_on_start=${open_manager_on_start}."
printf '%s\n' "Follow the prompts inside the Gromaq terminal window exactly."

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

manager_visible="$(cat "${proof_root}/tmux-manager-visible.txt" 2>/dev/null || true)"
status_strip_visible="$(cat "${proof_root}/tmux-status-strip-visible.txt" 2>/dev/null || true)"
navigation_checked="$(cat "${proof_root}/tmux-navigation-checked.txt" 2>/dev/null || true)"
right_prompt_legible="$(cat "${proof_root}/tmux-right-prompt-legible.txt" 2>/dev/null || true)"
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
if [ "${navigation_checked}" != "navigation-checked" ]; then
  printf '%s\n' "error: expected navigation-checked confirmation, got '${navigation_checked}'." >&2
  exit 1
fi
if [ "${right_prompt_legible}" != "right-prompt-legible" ]; then
  printf '%s\n' "error: expected right-prompt-legible confirmation, got '${right_prompt_legible}'." >&2
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
if [ "${workspace_visible}" != "workspace-visible" ]; then
  printf '%s\n' "error: expected workspace-visible confirmation, got '${workspace_visible}'." >&2
  exit 1
fi
if [ "${normal_shell_input}" != "normal-shell-input" ]; then
  printf '%s\n' "error: expected normal-shell-input confirmation, got '${normal_shell_input}'." >&2
  exit 1
fi

{
  printf '%s\n' "macOS native tmux manual proof: ok"
  printf '%s\n' "Launch mode: ${launch_mode}"
  printf '%s\n' "Open manager on start: ${open_manager_on_start}"
  printf '%s\n' "App: ${app_path:-none}"
  printf '%s\n' "Executable: ${executable}"
  printf '%s\n' "Version: ${actual_version}"
  printf '%s\n' "tmux: ${tmux_version}"
  printf '%s\n' "session: ${session}"
  printf '%s\n' "kill-session target: ${kill_session}"
  printf '%s\n' "workspace-session: ${workspace_session}"
  printf '%s\n' "tmux-status-strip-visible.txt: ${status_strip_visible}"
  printf '%s\n' "tmux-manager-visible.txt: ${manager_visible}"
  printf '%s\n' "tmux-navigation-checked.txt: ${navigation_checked}"
  printf '%s\n' "tmux-right-prompt-legible.txt: ${right_prompt_legible}"
  printf '%s\n' "tmux-start-session.txt: ${start_session}"
  printf '%s\n' "tmux-attach-session.txt: ${attach_session}"
  printf '%s\n' "tmux-safe-action.txt: ${safe_action}"
  printf '%s\n' "tmux-new-window.txt: ${new_window}"
  printf '%s\n' "tmux-refresh-checked.txt: ${refresh_checked}"
  printf '%s\n' "tmux-destructive-confirmation.txt: ${destructive_confirmation}"
  printf '%s\n' "tmux-workspace-visible.txt: ${workspace_visible}"
  printf '%s\n' "tmux-normal-shell-input.txt: ${normal_shell_input}"
  printf '%s\n' "launch-mode.txt: ${launch_mode}"
  printf '%s\n' "open-manager-on-start.txt: ${open_manager_on_start}"
  printf '%s\n' "Proof root: ${proof_root}"
} | tee "${summary_path}"
