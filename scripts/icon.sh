#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p .cache/Reoclo.iconset
magick -density 768 -background none assets/mark.svg -resize 512x512 \
  -background '#f7f7ec' -gravity center -extent 1024x1024 assets/icon.png
for size in 16 32 128 256 512; do
  magick assets/icon.png -resize "${size}x${size}" ".cache/Reoclo.iconset/icon_${size}x${size}.png"
  double=$((size * 2))
  magick assets/icon.png -resize "${double}x${double}" ".cache/Reoclo.iconset/icon_${size}x${size}@2x.png"
done
iconutil -c icns .cache/Reoclo.iconset -o assets/Reoclo.icns
printf 'ICON PASS\n'
