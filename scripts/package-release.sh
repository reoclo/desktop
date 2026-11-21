#!/bin/bash
# Package an already signed bundle. Never print the signing environment.
set -euo pipefail
cd "$(dirname "$0")/.."
if [[ $# -lt 3 || $# -gt 4 ]]; then
  printf 'Usage: bash scripts/package-release.sh VERSION CLI_VERSION SIGNER [--test-key]\n' >&2
  exit 2
fi
version="$1"
cli_version="$2"
signer="$3"
test_key="${4:-}"
if [[ $# -eq 4 ]]; then
  [[ "$4" == --test-key ]] || exit 2
fi
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { printf 'Expected a stable semantic version\n' >&2; exit 2; }
bundle_version="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' dist/Reoclo.app/Contents/Info.plist)"
[[ "$bundle_version" == "$version" ]] || { printf 'Bundle version does not match release\n' >&2; exit 1; }
archive="dist/Reoclo-$version.app.tar.gz"
cp dist/Reoclo.dmg "dist/Reoclo-$version.dmg"
COPYFILE_DISABLE=1 tar -czf "$archive" -C dist Reoclo.app
printf 'Reoclo %s for macOS.\n\nIncludes Reoclo CLI %s. Existing CLI installations remain preferred.\n\nSigned with a Developer ID and notarized by Apple, so macOS opens it without an Open Anyway step.\n' \
  "$version" "$cli_version" > dist/release-notes.txt
base="${REOCLO_RELEASE_BASE_URL:-https://github.com/reoclo/desktop/releases/download/v$version}"
set -- "$archive" "$version" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  dist/release-notes.txt "$base/Reoclo-$version.app.tar.gz" dist/latest.json
if [[ "$test_key" == --test-key ]]; then
  set -- "$@" --test-key
fi
"$signer" "$@"
printf 'RELEASE PACKAGE PASS: %s\n' "$archive"
