#!/usr/bin/env bash
# Install Clauddy as a native app, with no Gatekeeper hassle.
#
#   curl -fsSL https://raw.githubusercontent.com/renatoaug/claude-usage-monitor/main/install.sh | bash
#
# Files downloaded with curl aren't quarantined like browser downloads, so the
# (unsigned, ad-hoc) app opens with a double-click — no "damaged" warning.
set -euo pipefail

REPO="renatoaug/claude-usage-monitor"
APP_NAME="Clauddy.app"
DEST="/Applications"

echo "→ Finding the latest release…"
ZIP_URL="$(
  curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" |
    grep -oE 'https://[^"]+-mac\.zip' | head -1
)"
if [ -z "${ZIP_URL:-}" ]; then
  echo "Couldn't find a macOS build in the latest release. Aborting." >&2
  exit 1
fi

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

echo "→ Downloading…"
curl -fSL --progress-bar "$ZIP_URL" -o "$TMP/app.zip"

echo "→ Installing to ${DEST}…"
unzip -q "$TMP/app.zip" -d "$TMP"
rm -rf "${DEST:?}/${APP_NAME}"
rm -rf "${DEST:?}/Claude Usage Monitor.app" # remove the pre-rebrand app if present
mv "$TMP/${APP_NAME}" "${DEST}/"

echo "→ Launching…"
open "${DEST}/${APP_NAME}"

echo "✓ Done — Clauddy is installed and will start at login."
