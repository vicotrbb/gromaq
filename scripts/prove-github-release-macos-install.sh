#!/bin/sh
set -eu

root="$(CDPATH= cd "$(dirname "$0")/.." && pwd)"
repo="${GROMAQ_GITHUB_REPO:-vicotrbb/gromaq}"
package="${GROMAQ_PACKAGE:-gromaq}"
app_name="${GROMAQ_APP_NAME:-Gromaq}"
version="${GROMAQ_VERSION:-v0.2.1}"
version_without_prefix="${version#v}"
proof_root="${GROMAQ_MACOS_RELEASE_PROOF_ROOT:-${root}/target/github-release-macos-install-proof}"
release_base="${GROMAQ_RELEASE_BASE:-https://github.com/${repo}/releases/download/${version}}"
asset_name="${GROMAQ_MACOS_RELEASE_ASSET:-Gromaq-macos-app.zip}"
checksum_name="${GROMAQ_CHECKSUM_ASSET:-SHA256SUMS-macos-app}"
expected_archs="${GROMAQ_EXPECT_MACOS_ARCHS:-x86_64 arm64}"
expect_notarized="${GROMAQ_EXPECT_NOTARIZED:-0}"
downloads_dir="${proof_root}/downloads"
extract_dir="${proof_root}/extract"
app_dir="${proof_root}/Applications"
downloaded_zip="${downloads_dir}/${asset_name}"
downloaded_checksums="${downloads_dir}/${checksum_name}"
extracted_app="${extract_dir}/${app_name}.app"
installed_app="${app_dir}/${app_name}.app"
summary_path="${proof_root}/summary.txt"

if [ "$(uname -s)" != "Darwin" ]; then
  printf '%s\n' "error: GitHub release macOS install proof must run on Darwin." >&2
  exit 1
fi

for tool in gh curl plutil codesign file lipo; do
  if ! command -v "${tool}" >/dev/null 2>&1; then
    printf '%s\n' "error: ${tool} is required for GitHub release macOS install proof." >&2
    exit 1
  fi
done

hash_file() {
  path="$1"
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "${path}" | awk '{print $1}'
    return
  fi
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "${path}" | awk '{print $1}'
    return
  fi
  printf '%s\n' "error: shasum or sha256sum is required to verify release checksums." >&2
  exit 1
}

extract_release_zip() {
  archive="$1"
  destination="$2"
  if command -v unzip >/dev/null 2>&1; then
    unzip -q "${archive}" -d "${destination}"
    return
  fi
  if command -v ditto >/dev/null 2>&1; then
    ditto -x -k "${archive}" "${destination}"
    return
  fi
  printf '%s\n' "error: unzip or ditto is required to extract ${asset_name}." >&2
  exit 1
}

verify_release_asset() {
  asset="$1"
  if ! printf '%s\n' "${release_assets}" | grep -Fx "${asset}" >/dev/null 2>&1; then
    printf '%s\n' "error: published release asset missing: ${asset}" >&2
    exit 1
  fi
}

verify_checksum() {
  manifest="$1"
  archive="$2"
  archive_name="$3"
  expected="$(
    awk -v name="${archive_name}" '$2 == name { print $1; found = 1 } END { if (!found) exit 1 }' "${manifest}"
  )" || {
    printf '%s\n' "error: checksum manifest did not contain ${archive_name}." >&2
    exit 1
  }
  actual="$(hash_file "${archive}")"
  if [ "${actual}" != "${expected}" ]; then
    printf '%s\n' "error: checksum mismatch for ${archive_name}: expected ${expected}, got ${actual}." >&2
    exit 1
  fi
}

verify_app_bundle() {
  app_path="$1"
  label="$2"
  executable="${app_path}/Contents/MacOS/${package}"
  plist="${app_path}/Contents/Info.plist"

  if [ ! -x "${executable}" ]; then
    printf '%s\n' "error: ${label} app executable missing or not executable: ${executable}" >&2
    exit 1
  fi
  if [ ! -f "${plist}" ]; then
    printf '%s\n' "error: ${label} app Info.plist missing: ${plist}" >&2
    exit 1
  fi

  actual_version="$("${executable}" --version 2>/dev/null || true)"
  expected_version="gromaq ${version_without_prefix}"
  if [ "${actual_version}" != "${expected_version}" ]; then
    printf '%s\n' "error: ${label} executable reported '${actual_version}', expected '${expected_version}'." >&2
    exit 1
  fi

  plist_version="$(plutil -extract CFBundleShortVersionString raw -o - "${plist}")"
  if [ "${plist_version}" != "${version_without_prefix}" ]; then
    printf '%s\n' "error: ${label} Info.plist CFBundleShortVersionString=${plist_version}, expected ${version_without_prefix}." >&2
    exit 1
  fi

  file "${executable}" > "${proof_root}/${label}-file.txt"
  lipo -info "${executable}" > "${proof_root}/${label}-lipo.txt"
  for expected_arch in ${expected_archs}; do
    if ! grep -F "${expected_arch}" "${proof_root}/${label}-lipo.txt" >/dev/null 2>&1; then
      printf '%s\n' "error: ${label} app binary missing architecture ${expected_arch}." >&2
      exit 1
    fi
  done

  codesign --verify --deep --strict --verbose=4 "${app_path}" > "${proof_root}/${label}-codesign-verify.txt" 2>&1
  codesign -dv --verbose=4 "${app_path}" > "${proof_root}/${label}-codesign-details.txt" 2>&1

  spctl_status=0
  if command -v spctl >/dev/null 2>&1; then
    spctl -a -vvv -t execute "${app_path}" > "${proof_root}/${label}-spctl.txt" 2>&1 || spctl_status="$?"
  else
    printf '%s\n' "spctl unavailable" > "${proof_root}/${label}-spctl.txt"
    spctl_status=127
  fi

  if [ "${expect_notarized}" = "1" ]; then
    if [ "${spctl_status}" -ne 0 ]; then
      cat "${proof_root}/${label}-spctl.txt" >&2
      printf '%s\n' "error: ${label} app failed spctl assessment but GROMAQ_EXPECT_NOTARIZED=1." >&2
      exit "${spctl_status}"
    fi
  elif [ "${spctl_status}" -eq 0 ]; then
    printf '%s\n' "error: ${label} app passed spctl, but release claim expects non-notarized/ad-hoc rejection." >&2
    exit 1
  fi
}

rm -rf "${proof_root}"
mkdir -p "${downloads_dir}" "${extract_dir}" "${app_dir}"

release_assets="$(
  gh release view "${version}" \
    --repo "${repo}" \
    --json assets \
    --jq '.assets[].name'
)"
verify_release_asset "${asset_name}"
verify_release_asset "${checksum_name}"

curl -fsSL "${release_base}/${asset_name}" -o "${downloaded_zip}"
curl -fsSL "${release_base}/${checksum_name}" -o "${downloaded_checksums}"
verify_checksum "${downloaded_checksums}" "${downloaded_zip}" "${asset_name}"
extract_release_zip "${downloaded_zip}" "${extract_dir}"
verify_app_bundle "${extracted_app}" "extracted"

GROMAQ_PLATFORM=Darwin \
  GROMAQ_INSTALL_METHOD=release \
  GROMAQ_VERSION="${version}" \
  GROMAQ_RELEASE_BASE="${release_base}" \
  GROMAQ_MACOS_APP_DIR="${app_dir}" \
  GROMAQ_VERIFY_CHECKSUMS=1 \
  "${root}/scripts/install.sh" > "${proof_root}/install.log"
verify_app_bundle "${installed_app}" "installed"

{
  printf '%s\n' "GitHub release macOS install proof: ok"
  printf '%s\n' "Repository: ${repo}"
  printf '%s\n' "Version: ${version}"
  printf '%s\n' "Release base: ${release_base}"
  printf '%s\n' "Verified release assets:"
  printf '%s\n' "  ${asset_name}"
  printf '%s\n' "  ${checksum_name}"
  printf '%s\n' "Expected architectures: ${expected_archs}"
  printf '%s\n' "Expected notarized: ${expect_notarized}"
  printf '%s\n' "Downloaded asset: ${downloaded_zip}"
  printf '%s\n' "Checksum manifest: ${downloaded_checksums}"
  printf '%s\n' "Extracted app: ${extracted_app}"
  printf '%s\n' "Installed app: ${installed_app}"
  printf '%s\n' "Installed version: $("${installed_app}/Contents/MacOS/${package}" --version)"
  printf '%s\n' "Install log: ${proof_root}/install.log"
  printf '%s\n' "Codesign proof: ${proof_root}/installed-codesign-verify.txt"
  printf '%s\n' "spctl proof: ${proof_root}/installed-spctl.txt"
} | tee "${summary_path}"
