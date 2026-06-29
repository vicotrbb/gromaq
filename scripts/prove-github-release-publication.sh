#!/bin/sh
set -eu

root="$(CDPATH= cd "$(dirname "$0")/.." && pwd)"
repo="${GROMAQ_GITHUB_REPO:-vicotrbb/gromaq}"
version="${GROMAQ_VERSION:-v0.2.1}"
version_without_prefix="${version#v}"
release_arch="${GROMAQ_RELEASE_ARCH:-x86_64}"
deb_arch="${GROMAQ_DEB_ARCH:-amd64}"
proof_dir="${GROMAQ_RELEASE_PUBLICATION_PROOF_DIR:-${root}/target/github-release-publication-proof}"
release_json="${proof_dir}/release.json"
summary_path="${proof_dir}/summary.txt"

if ! command -v gh >/dev/null 2>&1; then
  printf '%s\n' "error: gh is required to verify published GitHub Release assets." >&2
  exit 1
fi

rm -rf "${proof_dir}"
mkdir -p "${proof_dir}"

release_view_status=0
gh release view "${version}" \
  --repo "${repo}" \
  --json tagName,isDraft,isPrerelease,url,assets > "${release_json}" 2> "${proof_dir}/release-view.stderr" || release_view_status="$?"
if [ "${release_view_status}" -ne 0 ]; then
  cat "${proof_dir}/release-view.stderr" >&2
  printf '%s\n' "error: GitHub Release ${version} was not found or could not be read." >&2
  exit "${release_view_status}"
fi
rm -f "${proof_dir}/release-view.stderr"

release_tag="$(
  gh release view "${version}" --repo "${repo}" --json tagName --jq '.tagName'
)"
is_draft="$(
  gh release view "${version}" --repo "${repo}" --json isDraft --jq '.isDraft'
)"
is_prerelease="$(
  gh release view "${version}" --repo "${repo}" --json isPrerelease --jq '.isPrerelease'
)"
release_url="$(
  gh release view "${version}" --repo "${repo}" --json url --jq '.url'
)"
release_assets="$(
  gh release view "${version}" --repo "${repo}" --json assets --jq '.assets[].name'
)"

if [ "${release_tag}" != "${version}" ]; then
  printf '%s\n' "error: release tagName mismatch: expected ${version}, saw ${release_tag}" >&2
  exit 1
fi

if [ "${is_draft}" != "false" ]; then
  printf '%s\n' "error: release ${version} is still a draft." >&2
  exit 1
fi

if [ "${is_prerelease}" != "false" ]; then
  printf '%s\n' "error: release ${version} is marked as a prerelease." >&2
  exit 1
fi

verify_release_asset() {
  asset="$1"
  if ! printf '%s\n' "${release_assets}" | grep -Fx "${asset}" >/dev/null 2>&1; then
    printf '%s\n' "error: published release asset missing: ${asset}" >&2
    exit 1
  fi
}

verify_release_asset "gromaq-${version_without_prefix}-linux-${release_arch}.tar.gz"
verify_release_asset "gromaq_${version_without_prefix}_${deb_arch}.deb"
verify_release_asset "PKGBUILD"
verify_release_asset "default.SRCINFO"
verify_release_asset "gromaq.install"
verify_release_asset "Gromaq-macos-app.zip"
verify_release_asset "SHA256SUMS-linux-${release_arch}"
verify_release_asset "SHA256SUMS-macos-app"

{
  printf '%s\n' "GitHub release publication proof: ok"
  printf '%s\n' "Repository: ${repo}"
  printf '%s\n' "Version: ${version}"
  printf '%s\n' "Release URL: ${release_url}"
  printf '%s\n' "Release JSON: ${release_json}"
  printf '%s\n' "Verified release assets:"
  printf '%s\n' "  gromaq-${version_without_prefix}-linux-${release_arch}.tar.gz"
  printf '%s\n' "  gromaq_${version_without_prefix}_${deb_arch}.deb"
  printf '%s\n' "  PKGBUILD"
  printf '%s\n' "  default.SRCINFO"
  printf '%s\n' "  gromaq.install"
  printf '%s\n' "  Gromaq-macos-app.zip"
  printf '%s\n' "  SHA256SUMS-linux-${release_arch}"
  printf '%s\n' "  SHA256SUMS-macos-app"
} | tee "${summary_path}"
