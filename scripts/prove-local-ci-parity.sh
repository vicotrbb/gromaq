#!/bin/sh
set -eu

root="$(CDPATH= cd "$(dirname "$0")/.." && pwd)"

run_step() {
  label="$1"
  shift

  printf '\n==> %s\n' "${label}"
  printf '$ %s\n' "$*"
  "$@"
}

cd "${root}"

run_step "format" cargo fmt --check
run_step "whitespace" git diff --check
run_step "staged whitespace" git diff --cached --check
run_step "clippy" cargo clippy --all-targets --all-features -- -D warnings
run_step "tests" cargo test --all
run_step "theme preview proof" scripts/prove-theme-preview.sh
run_step "avatar asset freshness proof" node images/avatar/generate.mjs --check
run_step "welcome preview proof" scripts/prove-welcome-preview.sh
run_step "README welcome preview freshness proof" scripts/prove-readme-welcome-preview.sh
run_step "current-host compatibility proof" scripts/prove-current-host-compatibility.sh
run_step "parser benchmark inventory" cargo bench --bench parser_throughput -- --list

printf '\n%s\n' "Local CI parity proof: ok"
