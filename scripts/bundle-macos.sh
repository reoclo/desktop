#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
if [[ $# -lt 1 || $# -gt 2 ]]; then
  printf 'Usage: scripts/bundle-macos.sh /path/to/reoclo [/path/to/prebuilt-desktop]\n' >&2
  exit 2
fi
cli="$(cd "$(dirname "$1")" && pwd)/$(basename "$1")"
[[ -x "$cli" ]] || { printf 'CLI binary is not executable: %s\n' "$cli" >&2; exit 2; }
if [[ -d "$PWD/.cache/rustup/toolchains" ]]; then
  export RUSTUP_HOME="$PWD/.cache/rustup"
  export CARGO_HOME="$PWD/.cache/cargo"
fi
mkdir -p dist
if [[ $# -eq 2 ]]; then
  desktop="$(cd "$(dirname "$2")" && pwd)/$(basename "$2")"
  [[ -x "$desktop" ]] || { printf 'Desktop binary is not executable: %s\n' "$desktop" >&2; exit 2; }
else
  cargo build --release --locked --bin reoclo-desktop
  desktop="$PWD/target/release/reoclo-desktop"
fi
stage="$(mktemp -d "$PWD/dist/bundle.XXXXXX")"
trap 'rm -rf "$stage"' EXIT
app="$stage/Reoclo.app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
# macOS names the app menu's About, Hide and Quit items after the executable.
cp "$desktop" "$app/Contents/MacOS/Reoclo"
cp "$cli" "$app/Contents/Resources/reoclo"
cp assets/Info.plist "$app/Contents/Info.plist"
cp assets/Reoclo.icns "$app/Contents/Resources/Reoclo.icns"
cp assets/fonts/OFL-*.txt "$app/Contents/Resources/"
plutil -lint "$app/Contents/Info.plist"
# Releases set REOCLO_SIGN_IDENTITY to the Developer ID (and REOCLO_SIGN_KEYCHAIN to its
# keychain); local builds sign ad hoc. Both use the hardened runtime, so a local build
# catches anything that would break under notarization.
identity="${REOCLO_SIGN_IDENTITY:--}"
sign=(codesign --force --sign "$identity")
if [[ "$identity" == - ]]; then
  sign+=(--timestamp=none)
else
  sign+=(--timestamp)
fi
if [[ -n "${REOCLO_SIGN_KEYCHAIN:-}" ]]; then
  sign+=(--keychain "$REOCLO_SIGN_KEYCHAIN")
fi
# Nested code is signed before the bundle that seals it.
"${sign[@]}" --options runtime --identifier com.reoclo.cli \
  --entitlements assets/cli.entitlements "$app/Contents/Resources/reoclo"
"${sign[@]}" --options runtime "$app"
codesign --verify --deep --strict --verbose=2 "$app"
rm -rf dist/Reoclo.app
ditto "$app" dist/Reoclo.app
rm -f dist/Reoclo.dmg
uvx --from 'dmgbuild==1.6.7' dmgbuild -s assets/dmg/settings.py -D app="$app" Reoclo dist/Reoclo.dmg
if [[ "$identity" != - ]]; then
  "${sign[@]}" dist/Reoclo.dmg
fi
shasum -a 256 dist/Reoclo.dmg
printf 'BUNDLE PASS: %s/dist/Reoclo.app\nDMG PASS: %s/dist/Reoclo.dmg\n' "$PWD" "$PWD"
