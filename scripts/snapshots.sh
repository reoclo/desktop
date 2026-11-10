#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
if [[ -d "$PWD/.cache/rustup/toolchains" ]]; then
  export RUSTUP_HOME="$PWD/.cache/rustup"
  export CARGO_HOME="$PWD/.cache/cargo"
fi
mkdir -p snapshots evidence
cargo build --locked --bins
app=./target/debug/reoclo-desktop
for screen in tunnels add hosts certificates settings panel; do
  for state in loaded loading empty terminal offline signed-out device-code error; do
    for theme in light dark; do
      name="$screen-$state-$theme"
      "$app" --fixture --screen "$screen" --state "$state" --theme "$theme" \
        --snap "snapshots/$name.png" --tree "snapshots/$name.json"
      printf 'CAPTURE %s\n' "$name"
    done
  done
done
for theme in light dark; do
  for dialog in edit remove discard start setup trust untrust get-started get-started-progress get-started-done open-link; do
    "$app" --fixture --screen "$dialog" --theme "$theme" \
      --snap "snapshots/dialog-$dialog-$theme.png" --tree "snapshots/dialog-$dialog-$theme.json"
  done
  "$app" --fixture --screen add --theme "$theme" --script scripts/port-picker.json --at 1 \
    --snap "snapshots/add-port-picker-$theme.png" --tree "snapshots/add-port-picker-$theme.json"
  "$app" --fixture --screen add --theme "$theme" --script scripts/form-end.json --at 1 \
    --snap "snapshots/add-end-$theme.png" --tree "snapshots/add-end-$theme.json"
  "$app" --fixture --screen panel --theme "$theme" --script scripts/panel-end.json --at 1 \
    --snap "snapshots/panel-end-$theme.png" --tree "snapshots/panel-end-$theme.json"
  for screen in tunnels hosts certificates settings add; do
    "$app" --fixture --screen "$screen" --theme "$theme" --size 760x620 \
      --script scripts/window-end.json --at 1 \
      --snap "snapshots/$screen-end-760x620-$theme.png" --tree "snapshots/$screen-end-760x620-$theme.json"
    for size in 760x620 1920x1080; do
      "$app" --fixture --screen "$screen" --theme "$theme" --size "$size" \
        --snap "snapshots/$screen-$size-$theme.png" --tree "snapshots/$screen-$size-$theme.json"
    done
  done
  for state in down ready connecting active reconnecting needs_sign_in server_offline quota_reached expired closed_by_operator forbidden failed; do
    "$app" --fixture --state "$state" --theme "$theme" \
      --snap "snapshots/tunnel-state-$state-$theme.png" --tree "snapshots/tunnel-state-$state-$theme.json"
  done
  for at in 0 0.15 1; do
    "$app" --fixture --theme "$theme" --script scripts/navigation.json --at "$at" \
      --snap "snapshots/navigation-$at-$theme.png" --tree "snapshots/navigation-$at-$theme.json"
  done
  "$app" --fixture --theme "$theme" --script scripts/navigation.json --reduced-motion --at 1 \
    --snap "snapshots/navigation-reduced-$theme.png" --tree "snapshots/navigation-reduced-$theme.json"
done
./target/debug/snapshot-check snapshots
find "$PWD/snapshots" -maxdepth 1 -name '*.png' -print | sort > evidence/snapshot-paths.txt
printf 'SNAPSHOTS PASS\n'
