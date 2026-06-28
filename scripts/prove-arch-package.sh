#!/bin/sh
set -eu

root="$(CDPATH= cd "$(dirname "$0")/.." && pwd)"
image="${GROMAQ_ARCH_PROOF_IMAGE:-archlinux:base-devel}"
platform="${GROMAQ_ARCH_PROOF_PLATFORM:-linux/amd64}"
proof_root="${GROMAQ_ARCH_PROOF_ROOT:-${root}/target/arch-package-proof}"
summary_path="${GROMAQ_ARCH_PROOF_SUMMARY:-${root}/target/arch-package-proof-summary.txt}"
payload_count_path="${proof_root}/payload-count.txt"

if ! command -v docker >/dev/null 2>&1; then
  printf '%s\n' "error: docker is required for Arch package proof." >&2
  exit 1
fi

if ! docker version >/dev/null 2>&1; then
  printf '%s\n' "error: docker daemon is not reachable for Arch package proof." >&2
  exit 1
fi

mkdir -p "${proof_root}"
rm -f "${payload_count_path}"

docker run --rm \
  --platform="${platform}" \
  --mount "type=bind,src=${root},dst=/work,readonly" \
  --mount "type=bind,src=${proof_root},dst=/proof" \
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
wc -l < /tmp/gromaq-arch-payload.txt | tr -d " " > /proof/payload-count.txt
'

payload_entries="$(cat "${payload_count_path}")"
mkdir -p "$(dirname "${summary_path}")"
{
  printf '%s\n' "Arch package proof: ok"
  printf '%s\n' "Container image: ${image}"
  printf '%s\n' "Container platform: ${platform}"
  printf '%s\n' "PKGBUILD: ${root}/packaging/arch/PKGBUILD"
  printf '%s\n' ".SRCINFO: ${root}/packaging/arch/.SRCINFO"
  printf '%s\n' "Install hook: ${root}/packaging/arch/gromaq.install"
  printf '%s\n' "Payload entries: ${payload_entries}"
} | tee "${summary_path}"
