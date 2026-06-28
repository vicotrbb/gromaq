#!/bin/sh
set -eu

root="$(CDPATH= cd "$(dirname "$0")/.." && pwd)"
proof_dir="${root}/target/local-ci-parity-proof"

run_step() {
  label="$1"
  shift

  printf '\n==> %s\n' "${label}"
  printf '$ %s\n' "$*"
  "$@"
}

run_shell_syntax_checks() {
  for script in scripts/*.sh; do
    run_step "shell syntax: ${script}" sh -n "${script}"
  done
  run_step "Arch PKGBUILD syntax" bash -n packaging/arch/PKGBUILD
  run_step "Arch install hook syntax" sh -n packaging/arch/gromaq.install
}

cd "${root}"
mkdir -p "${proof_dir}"

run_shell_syntax_checks
run_step "format" cargo fmt --check
run_step "whitespace" git diff --check
run_step "staged whitespace" git diff --cached --check
run_step "clippy" cargo clippy --all-targets --all-features -- -D warnings
run_step "tests" cargo test --all
run_step "theme legibility smoke" cargo run -- --theme-legibility-smoke
run_step "theme preview proof" scripts/prove-theme-preview.sh
run_step "avatar asset freshness proof" node images/avatar/generate.mjs --check
run_step "welcome preview proof" scripts/prove-welcome-preview.sh
run_step "README welcome preview freshness proof" scripts/prove-readme-welcome-preview.sh
run_step "GPU welcome image snapshot proof" \
  cargo run -- --welcome-image-snapshot "${proof_dir}/gromaq-welcome-image.ppm"
run_step "frame scheduler smoke" cargo run -- --frame-scheduler-smoke
run_step "current-host compatibility proof" scripts/prove-current-host-compatibility.sh
run_step "parser benchmark inventory" cargo bench --bench parser_throughput -- --list

printf '\n%s\n' "Local CI parity proof: ok"
