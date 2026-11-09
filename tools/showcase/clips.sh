#!/usr/bin/env bash
# Renders the app's clips frame by frame (native/src/screens/showcase.rs) and
# encodes each into assets/clips/<name>.webm: VP9 with alpha, so the clips
# float on the composition's own background.
#   ./clips.sh            every clip
#   ./clips.sh picker,talk  only these
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
tmp="$here/tmp"
out="$here/assets/clips"
mkdir -p "$tmp" "$out"

(cd "$here/../../native" && SHOWCASE_OUT="$tmp" SHOWCASE_ONLY="${1:-}" \
  cargo test --release showcase -- --ignored --nocapture 2>&1 | grep -E 'frames|panicked')

for dir in "$tmp"/*/; do
  name="$(basename "$dir")"
  if [ -n "${1:-}" ] && [[ ",$1," != *",$name,"* ]]; then continue; fi
  ffmpeg -v error -y -framerate 30 -i "$dir/%04d.png" \
    -c:v libvpx-vp9 -pix_fmt yuva420p -b:v 6M -auto-alt-ref 0 "$out/$name.webm"
  # a still of the first frame, handy for layout checks
  cp "$dir/0000.png" "$out/$name.png"
  echo "$name.webm"
done
