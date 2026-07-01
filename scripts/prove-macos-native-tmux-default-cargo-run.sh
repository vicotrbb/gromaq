#!/bin/sh
set -eu

root="$(CDPATH= cd "$(dirname "$0")/.." && pwd)"
proof_root="${GROMAQ_DEFAULT_CARGO_TMUX_PROOF_ROOT:-${root}/target/macos-native-tmux-default-cargo-run-proof}"
summary_path="${proof_root}/summary.txt"
session="gromaq-default-cargo-tmux-$$"
kill_session="${session}-kill"

cleanup() {
  tmux kill-session -t "${session}" >/dev/null 2>&1 || true
  tmux kill-session -t "${kill_session}" >/dev/null 2>&1 || true
}

if [ "$(uname -s)" != "Darwin" ]; then
  printf '%s\n' "error: macOS native tmux default cargo run proof must run on Darwin." >&2
  exit 1
fi

if ! command -v tmux >/dev/null 2>&1; then
  printf '%s\n' "error: tmux is required for macOS native tmux default cargo run proof." >&2
  exit 1
fi
tmux_version="$(tmux -V)"

rm -rf "${proof_root}"
mkdir -p "${proof_root}"
trap cleanup EXIT INT TERM
cleanup

tmux new-session -d -s "${session}" -n code
tmux split-window -t "${session}:0" -h
tmux new-session -d -s "${kill_session}" -n disposable

printf '%s\n' "macOS native tmux default cargo run proof"
printf '%s\n' "tmux: ${tmux_version}"
printf '%s\n' "Target session: ${session}"
printf '%s\n' "Disposable kill target: ${kill_session}"
printf '%s\n' "A default Gromaq window will open through plain cargo run."
printf '%s\n' "In that window, verify the native tmux UI against the checklist below."
printf '%s\n' "Close the Gromaq window when the checklist is complete; this script will then ask for exact confirmation tokens."
printf '%s\n' ""
printf '%s\n' "Checklist while cargo run is open:"
printf '%s\n' "- Persistent tmux status strip is visible and legible."
printf '%s\n' "- Control/Super Shift+T opens a real manager panel, not a tiny hint."
printf '%s\n' "- Sessions, windows, panes, current target, and pane command text are visible."
printf '%s\n' "- Keyboard navigation changes selection."
printf '%s\n' "- A safe tmux action, such as split pane, runs from the UI."
printf '%s\n' "- A destructive action shows inline confirmation before running."
printf '%s\n' "- Confirm a kill only against ${kill_session}."
printf '%s\n' "- Close the panel and verify normal shell input still reaches the prompt."
printf '%s\n' "- Check prompt/right-prompt layout for legible overlap behavior."

(
  cd "${root}"
  cargo run > "${proof_root}/cargo-run.stdout" 2> "${proof_root}/cargo-run.stderr"
)

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

record_confirmation "Confirm the persistent tmux status strip was visible and legible." "status-strip-visible" "tmux-default-cargo-run-status-strip.txt"
record_confirmation "Confirm Control/Super Shift+T opened a real manager panel." "manager-visible" "tmux-default-cargo-run-manager-visible.txt"
record_confirmation "Confirm the manager was not a tiny hint or palette." "not-hint" "tmux-default-cargo-run-not-hint.txt"
record_confirmation "Confirm sessions/windows/panes/current target/pane command text were visible." "state-visible" "tmux-default-cargo-run-state-visible.txt"
record_confirmation "Confirm keyboard navigation changed selection." "navigation-checked" "tmux-default-cargo-run-navigation.txt"
record_confirmation "Confirm a safe tmux action ran from the UI." "safe-action" "tmux-default-cargo-run-safe-action.txt"
record_confirmation "Confirm a destructive action showed inline confirmation first." "destructive-confirmation" "tmux-default-cargo-run-destructive-confirmation.txt"
record_confirmation "Confirm any kill action was performed only against ${kill_session}." "isolated-kill-confirmed" "tmux-default-cargo-run-isolated-kill.txt"
record_confirmation "Confirm normal shell input still worked after closing the panel." "shell-input" "tmux-default-cargo-run-shell-input.txt"
record_confirmation "Confirm prompt/right-prompt layout stayed legible." "right-prompt-legible" "tmux-default-cargo-run-right-prompt.txt"

{
  printf '%s\n' "macOS native tmux default cargo run proof: ok"
  printf '%s\n' "tmux: ${tmux_version}"
  printf '%s\n' "session: ${session}"
  printf '%s\n' "kill-session target: ${kill_session}"
  printf '%s\n' "cargo-run stdout: ${proof_root}/cargo-run.stdout"
  printf '%s\n' "cargo-run stderr: ${proof_root}/cargo-run.stderr"
  printf '%s\n' "tmux-default-cargo-run-status-strip.txt: status-strip-visible"
  printf '%s\n' "tmux-default-cargo-run-manager-visible.txt: manager-visible"
  printf '%s\n' "tmux-default-cargo-run-not-hint.txt: not-hint"
  printf '%s\n' "tmux-default-cargo-run-state-visible.txt: state-visible"
  printf '%s\n' "tmux-default-cargo-run-navigation.txt: navigation-checked"
  printf '%s\n' "tmux-default-cargo-run-safe-action.txt: safe-action"
  printf '%s\n' "tmux-default-cargo-run-destructive-confirmation.txt: destructive-confirmation"
  printf '%s\n' "tmux-default-cargo-run-isolated-kill.txt: isolated-kill-confirmed"
  printf '%s\n' "tmux-default-cargo-run-shell-input.txt: shell-input"
  printf '%s\n' "tmux-default-cargo-run-right-prompt.txt: right-prompt-legible"
  printf '%s\n' "Proof root: ${proof_root}"
} | tee "${summary_path}"
