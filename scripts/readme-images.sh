#!/bin/bash
# Renders the README screenshots into docs/images from the fixture daemon, in both themes.
# Run it after a UI change so the README shows the app as it is. pngquant, when installed, shrinks them.
set -euo pipefail
cd "$(dirname "$0")/.."
if [[ -d "$PWD/.cache/rustup/toolchains" ]]; then
  export RUSTUP_HOME="$PWD/.cache/rustup"
  export CARGO_HOME="$PWD/.cache/cargo"
fi
cargo build --locked --bin reoclo-desktop
app=./target/debug/reoclo-desktop
out=docs/images
mkdir -p "$out"
for theme in light dark; do
  "$app" --fixture --screen tunnels --state loaded --theme "$theme" --snap "$out/tunnels-$theme.png"
  "$app" --fixture --screen panel --state loaded --theme "$theme" --snap "$out/panel-$theme.png"
  "$app" --fixture --screen add --state loaded --theme "$theme" --snap "$out/add-tunnel-$theme.png"
  "$app" --fixture --screen get-started --theme "$theme" --snap "$out/get-started-$theme.png"
  "$app" --fixture --screen certificates --state loaded --theme "$theme" --snap "$out/certificates-$theme.png"
  "$app" --fixture --screen hosts --state loaded --theme "$theme" --snap "$out/hosts-$theme.png"
done
if command -v pngquant >/dev/null; then
  pngquant --quality 85-98 --strip --force --ext .png "$out"/*.png
fi
ls -l "$out"
printf 'README IMAGES PASS\n'
