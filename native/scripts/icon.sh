#!/bin/bash
# Build assets/icon.icns (every macOS size, 16 to 1024 px at 1x and 2x) from
# assets/icon.png, a 1024 x 1024 icon already laid out on the macOS template.
#
#   native/scripts/icon.sh
set -euo pipefail
cd "$(dirname "$0")/.."

SRC=assets/icon.png
SET=$(mktemp -d)/icon.iconset
mkdir -p "$SET"
for size in 16 32 128 256 512; do
  sips -z $size $size "$SRC" --out "$SET/icon_${size}x${size}.png" >/dev/null
  sips -z $((size * 2)) $((size * 2)) "$SRC" --out "$SET/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$SET" -o assets/icon.icns
echo "built assets/icon.icns"
