#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
if [[ $# -ne 1 ]]; then
  printf 'Usage: scripts/bundle-macos.sh /absolute/path/to/reoclo\n' >&2
  exit 2
fi
cli="$(cd "$(dirname "$1")" && pwd)/$(basename "$1")"
[[ -x "$cli" ]] || { printf 'CLI binary is not executable: %s\n' "$cli" >&2; exit 2; }
if [[ -d "$PWD/.cache/rustup/toolchains" ]]; then
  export RUSTUP_HOME="$PWD/.cache/rustup"
  export CARGO_HOME="$PWD/.cache/cargo"
fi
mkdir -p dist
cargo build --release --locked --bin reoclo-desktop
stage="$(mktemp -d "$PWD/dist/bundle.XXXXXX")"
trap 'rm -rf "$stage"' EXIT
app="$stage/Reoclo.app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp target/release/reoclo-desktop "$app/Contents/MacOS/reoclo-desktop"
cp "$cli" "$app/Contents/Resources/reoclo"
cp assets/Info.plist "$app/Contents/Info.plist"
cp assets/Reoclo.icns "$app/Contents/Resources/Reoclo.icns"
cp assets/fonts/OFL-*.txt "$app/Contents/Resources/"
plutil -lint "$app/Contents/Info.plist"
codesign --force --sign - "$app/Contents/Resources/reoclo"
codesign --force --sign - "$app"
codesign --verify --deep --strict --verbose=2 "$app"
ditto "$app" dist/Reoclo.app
hdiutil create -volname Reoclo -srcfolder "$stage" -ov -format UDZO dist/Reoclo.dmg
shasum -a 256 dist/Reoclo.dmg
printf 'BUNDLE PASS: %s/dist/Reoclo.app\nDMG PASS: %s/dist/Reoclo.dmg\n' "$PWD" "$PWD"
