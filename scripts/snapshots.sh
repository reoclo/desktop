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
# The panel's window fits its content, so a panel capture crops to the panel rather than the
# launch canvas, and records the height the panel opens at.
crop_for() { if [[ $1 == panel ]]; then echo "--crop menu-panel"; fi; }
for screen in tunnels logs add hosts certificates settings panel; do
  for state in loaded loading empty terminal offline signed-out device-code error; do
    for theme in light dark; do
      name="$screen-$state-$theme"
      "$app" --fixture --at 1 --screen "$screen" --state "$state" --theme "$theme" $(crop_for "$screen") \
        --snap "snapshots/$name.png" --tree "snapshots/$name.json"
      printf 'CAPTURE %s\n' "$name"
    done
  done
done
for theme in light dark; do
  for spec in logs:filtered logs:paused logs:long tunnels:traffic-all tunnels:traffic-selected tunnels:traffic-terminal tunnels:retrying certificates:domain-invalid certificates:domain-valid; do
    screen="${spec%:*}"
    state="${spec#*:}"
    for size in 1100x760 760x620 390x844; do
      name="$screen-$state-$size-$theme"
      "$app" --fixture --at 1 --screen "$screen" --state "$state" --theme "$theme" --size "$size" \
        --snap "snapshots/$name.png" --tree "snapshots/$name.json"
      printf 'CAPTURE %s\n' "$name"
    done
  done
  "$app" --fixture --at 1 --state traffic-selected --reduced-motion --theme "$theme" \
    --snap "snapshots/traffic-reduced-$theme.png" --tree "snapshots/traffic-reduced-$theme.json"
  for screen in settings panel; do
    for state in update-available update-ready update-error update-disabled; do
      "$app" --fixture --at 1 --screen "$screen" --state "$state" --theme "$theme" $(crop_for "$screen") \
        --snap "snapshots/$screen-$state-$theme.png" --tree "snapshots/$screen-$state-$theme.json"
      for size in 760x620 390x844; do
        "$app" --fixture --at 1 --screen "$screen" --state "$state" --theme "$theme" --size "$size" $(crop_for "$screen") \
          --snap "snapshots/$screen-$state-$size-$theme.png" --tree "snapshots/$screen-$state-$size-$theme.json"
      done
    done
  done
  for screen in tunnels panel; do
    for state in shared-failure long-names events; do
      "$app" --fixture --at 1 --screen "$screen" --state "$state" --theme "$theme" $(crop_for "$screen") \
        --snap "snapshots/$screen-$state-$theme.png" --tree "snapshots/$screen-$state-$theme.json"
    done
  done
  for state in busy quiet failure; do
    "$app" --fixture --at 1 --screen panel --state "$state" --theme "$theme" --crop menu-panel \
      --snap "snapshots/panel-$state-$theme.png" --tree "snapshots/panel-$state-$theme.json"
  done
  for size in 1100x760 1920x1080; do
    "$app" --fixture --at 1 --screen tunnels --state busy --size "$size" --theme "$theme" \
      --snap "snapshots/tunnels-busy-$size-$theme.png" --tree "snapshots/tunnels-busy-$size-$theme.json"
  done
  "$app" --fixture --state busy --theme "$theme" --script scripts/sidebar-collapse.json --at 1 \
    --snap "snapshots/sidebar-collapsed-$theme.png" --tree "snapshots/sidebar-collapsed-$theme.json"
  "$app" --fixture --screen logs --state busy --theme "$theme" --script scripts/logs-filter.json --at 1 \
    --snap "snapshots/logs-dropdown-$theme.png" --tree "snapshots/logs-dropdown-$theme.json"
  "$app" --fixture --screen panel --theme "$theme" --size 390x844 --at 1 --crop menu-panel \
    --snap "snapshots/panel-390x844-$theme.png" --tree "snapshots/panel-390x844-$theme.json"
  "$app" --fixture --state events --theme "$theme" --script scripts/window-end.json --at 1 \
    --snap "snapshots/tunnels-events-end-$theme.png" --tree "snapshots/tunnels-events-end-$theme.json"
  for size in 760x620 1920x1080; do
    "$app" --fixture --at 1 --state long-names --size "$size" --theme "$theme" \
      --snap "snapshots/tunnels-long-names-$size-$theme.png" --tree "snapshots/tunnels-long-names-$size-$theme.json"
  done
  "$app" --fixture --theme "$theme" --script scripts/focus-switch.json --at 1 \
    --snap "snapshots/focus-switch-$theme.png" --tree "snapshots/focus-switch-$theme.json"
  "$app" --fixture --screen remove --theme "$theme" --script scripts/focus-confirm.json --at 1 \
    --snap "snapshots/focus-confirm-$theme.png" --tree "snapshots/focus-confirm-$theme.json"
  "$app" --fixture --state shared-failure --theme "$theme" --script scripts/shared-failure-settings.json --at 1 \
    --snap "snapshots/shared-failure-settings-$theme.png" --tree "snapshots/shared-failure-settings-$theme.json"
  for dialog in edit remove discard start setup trust untrust open-link; do
    "$app" --fixture --screen "$dialog" --theme "$theme" \
      --snap "snapshots/dialog-$dialog-$theme.png" --tree "snapshots/dialog-$dialog-$theme.json"
  done
  for state in welcome welcome-failed link installing waiting pairing pairing-failed pairing-helper tools tools-homebrew tools-failed ready ready-agents ready-connected loading; do
    for size in 1100x760 760x620; do
      "$app" --fixture --at 1 --screen first-run --state "$state" --theme "$theme" --size "$size" \
        --snap "snapshots/first-run-$state-$size-$theme.png" --tree "snapshots/first-run-$state-$size-$theme.json"
      printf 'CAPTURE first-run-%s-%s-%s\n' "$state" "$size" "$theme"
    done
  done
  "$app" --fixture --screen add --theme "$theme" --script scripts/port-picker.json --at 1 \
    --snap "snapshots/add-port-picker-$theme.png" --tree "snapshots/add-port-picker-$theme.json"
  "$app" --fixture --screen add --theme "$theme" --script scripts/form-end.json --at 1 \
    --snap "snapshots/add-end-$theme.png" --tree "snapshots/add-end-$theme.json"
  "$app" --fixture --screen panel --theme "$theme" --script scripts/panel-end.json --at 1 --crop menu-panel \
    --snap "snapshots/panel-end-$theme.png" --tree "snapshots/panel-end-$theme.json"
  for screen in tunnels hosts certificates settings add; do
    "$app" --fixture --screen "$screen" --theme "$theme" --size 760x620 \
      --script scripts/window-end.json --at 1 \
      --snap "snapshots/$screen-end-760x620-$theme.png" --tree "snapshots/$screen-end-760x620-$theme.json"
    for size in 760x620 1920x1080; do
      "$app" --fixture --at 1 --screen "$screen" --theme "$theme" --size "$size" \
        --snap "snapshots/$screen-$size-$theme.png" --tree "snapshots/$screen-$size-$theme.json"
    done
  done
  for state in loaded outdated cli-old; do
    "$app" --fixture --screen settings --state "$state" --theme "$theme" \
      --script scripts/window-end.json --at 1 \
      --snap "snapshots/settings-cli-$state-$theme.png" --tree "snapshots/settings-cli-$state-$theme.json"
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
