#!/bin/sh
set -eu

repo="${GROMAQ_REPO:-https://github.com/vicotrbb/gromaq}"
branch="${GROMAQ_BRANCH:-main}"
package="${GROMAQ_PACKAGE:-gromaq}"

if ! command -v cargo >/dev/null 2>&1; then
  printf '%s\n' "error: Cargo is required to install Gromaq." >&2
  printf '%s\n' "Install Rust stable from your package manager or https://rustup.rs, then rerun this installer." >&2
  exit 1
fi

printf '%s\n' "Installing ${package} from ${repo} (${branch})..."
cargo install --git "${repo}" --branch "${branch}" --locked --force "${package}"

printf '%s\n' "Installed ${package}."
printf '%s\n' "Run it with: ${package}"
