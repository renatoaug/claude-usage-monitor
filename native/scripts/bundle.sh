#!/bin/bash
# Build Clauddy.app (macOS) from the release binary: an Info.plist with the
# bundle id, the icon, no Dock icon (it's a widget), and an ad-hoc signature
# under that id. macOS delivers notifications only to a signed bundle, so the
# loose binary can't notify; this can, and needs no developer account.
#
#   native/scripts/bundle.sh            → native/target/Clauddy.app
#   BUNDLE_ID=app.clauddy native/scripts/bundle.sh   (the release id)
set -euo pipefail
[ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env" # cargo on PATH in a plain shell too
cd "$(dirname "$0")/.."

ID="${BUNDLE_ID:-app.clauddy.dev}" # its own id until it replaces the Electron app
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
APP=target/Clauddy.app

cargo build --release
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp target/release/clauddy "$APP/Contents/MacOS/clauddy"
cp ../build/icon.icns "$APP/Contents/Resources/icon.icns"
cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>Clauddy</string>
  <key>CFBundleDisplayName</key><string>Clauddy</string>
  <key>CFBundleIdentifier</key><string>$ID</string>
  <key>CFBundleExecutable</key><string>clauddy</string>
  <key>CFBundleIconFile</key><string>icon</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>$VERSION</string>
  <key>CFBundleVersion</key><string>$VERSION</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>LSApplicationCategoryType</key><string>public.app-category.developer-tools</string>
  <key>LSUIElement</key><true/>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST
codesign --force --sign - --identifier "$ID" "$APP"
echo "built $APP ($ID $VERSION)"
