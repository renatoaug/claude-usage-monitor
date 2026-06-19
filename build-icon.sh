#!/bin/bash
# Regenerate build/icon.icns from the pixel sprite in make-icon.js.
# (electron-builder reuses build/icon.icns; it does NOT run this.)
set -e
DIR="$(cd "$(dirname "$0")" && pwd)"
PNG=/tmp/cum-icon-1024.png
SET="$DIR/build/icon.iconset"

node "$DIR/make-icon.js" # writes $PNG

rm -rf "$SET"
mkdir -p "$SET"
sips -z 16 16 "$PNG" --out "$SET/icon_16x16.png" >/dev/null
sips -z 32 32 "$PNG" --out "$SET/icon_16x16@2x.png" >/dev/null
sips -z 32 32 "$PNG" --out "$SET/icon_32x32.png" >/dev/null
sips -z 64 64 "$PNG" --out "$SET/icon_32x32@2x.png" >/dev/null
sips -z 128 128 "$PNG" --out "$SET/icon_128x128.png" >/dev/null
sips -z 256 256 "$PNG" --out "$SET/icon_128x128@2x.png" >/dev/null
sips -z 256 256 "$PNG" --out "$SET/icon_256x256.png" >/dev/null
sips -z 512 512 "$PNG" --out "$SET/icon_256x256@2x.png" >/dev/null
sips -z 512 512 "$PNG" --out "$SET/icon_512x512.png" >/dev/null
cp "$PNG" "$SET/icon_512x512@2x.png"

iconutil -c icns "$SET" -o "$DIR/build/icon.icns"
echo "built $DIR/build/icon.icns"
