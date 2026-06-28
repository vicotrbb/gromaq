#!/bin/sh
set -eu

root="$(CDPATH= cd "$(dirname "$0")/.." && pwd)"
image="${GROMAQ_ARCH_PROOF_IMAGE:-archlinux:base-devel}"
platform="${GROMAQ_ARCH_PROOF_PLATFORM:-linux/amd64}"

if ! command -v docker >/dev/null 2>&1; then
  printf '%s\n' "error: docker is required for Arch package proof." >&2
  exit 1
fi

if ! docker version >/dev/null 2>&1; then
  printf '%s\n' "error: docker daemon is not reachable for Arch package proof." >&2
  exit 1
fi

docker run --rm \
  --platform="${platform}" \
  --mount "type=bind,src=${root},dst=/work,readonly" \
  "${image}" \
  bash -lc '
set -euo pipefail

pacman -Syu --noconfirm git rust
useradd -m builder
install -d -o builder -g builder /build
cp /work/packaging/arch/PKGBUILD /build/PKGBUILD
cp /work/packaging/arch/.SRCINFO /build/.SRCINFO
cp /work/packaging/arch/gromaq.install /build/gromaq.install
chown -R builder:builder /build

su builder -c "cd /build && makepkg --noconfirm"
pacman -U --noconfirm /build/gromaq-git-*.pkg.tar.*
test -x /usr/bin/gromaq
/usr/bin/gromaq --version

pacman -Ql gromaq-git | tee /tmp/gromaq-arch-payload.txt
grep -F /usr/bin/gromaq /tmp/gromaq-arch-payload.txt
grep -F /usr/share/doc/gromaq/README.md /tmp/gromaq-arch-payload.txt
grep -F /usr/share/licenses/gromaq/LICENSE /tmp/gromaq-arch-payload.txt
grep -F /usr/share/applications/dev.gromaq.Gromaq.desktop /tmp/gromaq-arch-payload.txt
grep -F /usr/share/metainfo/dev.gromaq.Gromaq.metainfo.xml /tmp/gromaq-arch-payload.txt
grep -F /usr/share/icons/hicolor/256x256/apps/dev.gromaq.Gromaq.png /tmp/gromaq-arch-payload.txt
'

printf '%s\n' "Arch package proof: ok"
