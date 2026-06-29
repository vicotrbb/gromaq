#!/bin/sh
set -eu

root="$(CDPATH= cd "$(dirname "$0")/.." && pwd)"
package="${GROMAQ_PACKAGE:-gromaq}"
app_name="${GROMAQ_APP_NAME:-Gromaq}"
version="${GROMAQ_VERSION:-v0.2.1}"
version_without_prefix="${version#v}"
proof_root="${GROMAQ_MANUAL_INPUT_PROOF_ROOT:-${root}/target/macos-live-input-manual-proof}"
default_installed_app="${root}/target/macos-release-install-proof/Applications/${app_name}.app"
default_dist_app="${root}/target/dist/${app_name}.app"
app_path="${GROMAQ_MANUAL_INPUT_APP:-}"
config_path="${proof_root}/gromaq-manual-input.toml"
shell_path="${proof_root}/manual-input-shell.sh"
summary_path="${proof_root}/summary.txt"

if [ "$(uname -s)" != "Darwin" ]; then
  printf '%s\n' "error: macOS live input manual proof must run on Darwin." >&2
  exit 1
fi

if ! command -v open >/dev/null 2>&1; then
  printf '%s\n' "error: open is required for macOS live input manual proof." >&2
  exit 1
fi

if [ -z "${app_path}" ]; then
  if [ -d "${default_installed_app}" ]; then
    app_path="${default_installed_app}"
  else
    app_path="${default_dist_app}"
  fi
fi

executable="${app_path}/Contents/MacOS/${package}"
if [ ! -x "${executable}" ]; then
  printf '%s\n' "error: app executable not found: ${executable}" >&2
  printf '%s\n' "Set GROMAQ_MANUAL_INPUT_APP=/path/to/Gromaq.app to choose the installed app." >&2
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

cat > "${shell_path}" <<EOF
#!/bin/sh
set -eu
printf '%s\n' ready > "${proof_root}/ready"
printf '%s\n' "Gromaq manual input proof ready."
printf '%s\n' "Type exactly: ls"
IFS= read -r typed_ls
printf '%s\n' "\${typed_ls}" > "${proof_root}/typed-ls.txt"
printf '%s\n' "Type exactly: pwd"
IFS= read -r typed_pwd
printf '%s\n' "\${typed_pwd}" > "${proof_root}/typed-pwd.txt"
printf '%s\n' "Type exactly: unicode:界́🙂"
IFS= read -r typed_unicode
printf '%s\n' "\${typed_unicode}" > "${proof_root}/typed-unicode.txt"
printf '%s\n' done > "${proof_root}/done"
EOF
chmod 755 "${shell_path}"

cat > "${config_path}" <<EOF
[terminal]
cols = 100
rows = 30
scrollback_lines = 1000

[shell]
program = "${shell_path}"
args = []
cwd = "${root}"

[welcome]
enabled = false
EOF

printf '%s\n' "macOS live input manual proof"
printf '%s\n' "App: ${app_path}"
printf '%s\n' "Version: ${actual_version}"
printf '%s\n' "A Gromaq window will open. Type each requested line exactly:"
printf '%s\n' "  1. Type exactly: ls"
printf '%s\n' "  2. Type exactly: pwd"
printf '%s\n' "  3. Type exactly: unicode:界́🙂"
printf '%s\n' "After the third line, close the Gromaq window if it remains open."

open -W -n \
  -o "${proof_root}/open.stdout" \
  --stderr "${proof_root}/open.stderr" \
  "${app_path}" \
  --args --config "${config_path}"

typed_ls="$(cat "${proof_root}/typed-ls.txt" 2>/dev/null || true)"
typed_pwd="$(cat "${proof_root}/typed-pwd.txt" 2>/dev/null || true)"
typed_unicode="$(cat "${proof_root}/typed-unicode.txt" 2>/dev/null || true)"

if [ "${typed_ls}" != "ls" ]; then
  printf '%s\n' "error: expected first typed line 'ls', got '${typed_ls}'." >&2
  exit 1
fi
if [ "${typed_pwd}" != "pwd" ]; then
  printf '%s\n' "error: expected second typed line 'pwd', got '${typed_pwd}'." >&2
  exit 1
fi
if [ "${typed_unicode}" != "unicode:界́🙂" ]; then
  printf '%s\n' "error: expected third typed line 'unicode:界́🙂', got '${typed_unicode}'." >&2
  exit 1
fi

{
  printf '%s\n' "macOS live input manual proof: ok"
  printf '%s\n' "App: ${app_path}"
  printf '%s\n' "Executable: ${executable}"
  printf '%s\n' "Version: ${actual_version}"
  printf '%s\n' "typed-ls.txt: ${typed_ls}"
  printf '%s\n' "typed-pwd.txt: ${typed_pwd}"
  printf '%s\n' "typed-unicode.txt: ${typed_unicode}"
  printf '%s\n' "Proof root: ${proof_root}"
} | tee "${summary_path}"
