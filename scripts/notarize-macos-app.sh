#!/bin/sh
set -eu

if [ "$(uname -s)" != "Darwin" ]; then
  printf '%s\n' "error: macOS notarization requires Darwin." >&2
  exit 1
fi

for tool in ditto xcrun; do
  if ! command -v "${tool}" >/dev/null 2>&1; then
    printf '%s\n' "error: ${tool} is required for macOS notarization." >&2
    exit 1
  fi
done

root="$(CDPATH= cd "$(dirname "$0")/.." && pwd)"
app_path="${1:-${root}/target/dist/Gromaq.app}"
zip_path="${GROMAQ_NOTARY_ZIP_PATH:-${root}/target/dist/Gromaq-notary.zip}"
dry_run="${GROMAQ_NOTARY_DRY_RUN:-0}"
keychain_profile="${GROMAQ_NOTARY_KEYCHAIN_PROFILE:-}"
apple_id="${GROMAQ_NOTARY_APPLE_ID:-}"
password="${GROMAQ_NOTARY_PASSWORD:-}"
team_id="${GROMAQ_NOTARY_TEAM_ID:-}"

if [ ! -d "${app_path}" ]; then
  printf '%s\n' "error: app bundle not found: ${app_path}" >&2
  exit 1
fi

mkdir -p "$(dirname "${zip_path}")"
rm -f "${zip_path}"
ditto -c -k --sequesterRsrc --keepParent "${app_path}" "${zip_path}"

if [ "${dry_run}" = "1" ]; then
  printf '%s\n' "Dry run: prepared notarization archive ${zip_path}"
  printf '%s\n' "Dry run: would submit ${zip_path} with xcrun notarytool submit --wait"
  printf '%s\n' "Dry run: would staple and validate ${app_path}"
  exit 0
fi

if [ -n "${keychain_profile}" ]; then
  xcrun notarytool submit "${zip_path}" --wait --keychain-profile "${keychain_profile}"
elif [ -n "${apple_id}" ] && [ -n "${password}" ] && [ -n "${team_id}" ]; then
  xcrun notarytool submit "${zip_path}" --wait \
    --apple-id "${apple_id}" \
    --password "${password}" \
    --team-id "${team_id}"
else
  printf '%s\n' "error: set GROMAQ_NOTARY_KEYCHAIN_PROFILE or GROMAQ_NOTARY_APPLE_ID/GROMAQ_NOTARY_PASSWORD/GROMAQ_NOTARY_TEAM_ID." >&2
  exit 1
fi

xcrun stapler staple "${app_path}"
xcrun stapler validate "${app_path}"

if command -v spctl >/dev/null 2>&1; then
  spctl --assess --type execute --verbose "${app_path}"
fi

printf '%s\n' "Notarized and stapled ${app_path}"
