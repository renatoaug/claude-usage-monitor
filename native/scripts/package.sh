#!/bin/bash
# Build and pack the release artifact for this OS into dist/, stamped with
# the version the release is about to publish (the CI build job runs it on
# each platform's own runner; see docs/design/07-releases.md).
#
#   native/scripts/package.sh 2.0.0
#
#   macOS    dist/Clauddy-<v>-arm64-mac.zip     Clauddy.app (app.clauddy, ad-hoc signed)
#   Windows  dist/Clauddy-<v>-win-x64.zip       Clauddy.exe
#   Linux    dist/Clauddy-<v>-linux-x64.tar.gz  Clauddy/clauddy + its icon
set -euo pipefail
VERSION="${1:?usage: native/scripts/package.sh <version>}"
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
DIST="$ROOT/dist"
mkdir -p "$DIST"
cd "$ROOT/native"

# the version shows in Settings and decides the update check
perl -0pi -e "s/^version = \"[^\"]*\"/version = \"$VERSION\"/m" Cargo.toml

STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT

case "$(uname -s)" in
  Darwin)
    BUNDLE_ID=app.clauddy scripts/bundle.sh
    # ditto keeps the bundle's signature and attributes, which zip would not
    ditto -c -k --keepParent target/Clauddy.app "$DIST/Clauddy-$VERSION-arm64-mac.zip"
    ;;
  Linux)
    cargo build --release
    mkdir -p "$STAGE/Clauddy"
    cp target/release/clauddy "$STAGE/Clauddy/clauddy"
    cp assets/icon.png "$STAGE/Clauddy/clauddy.png"
    tar -C "$STAGE" -czf "$DIST/Clauddy-$VERSION-linux-x64.tar.gz" Clauddy
    ;;
  MINGW* | MSYS* | CYGWIN*)
    cargo build --release
    cp target/release/clauddy.exe "$STAGE/Clauddy.exe"
    (cd "$STAGE" && 7z a -tzip -bso0 "$DIST/Clauddy-$VERSION-win-x64.zip" Clauddy.exe)
    ;;
  *)
    echo "package.sh: no release artifact for $(uname -s)" >&2
    exit 1
    ;;
esac
ls -l "$DIST"
