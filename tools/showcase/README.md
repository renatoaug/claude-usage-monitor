# Showcase — the 92-second product video

A short, music-only product video: an HTML composition animated with GSAP and rendered by [HyperFrames](https://hyperframes.dev). The widget on screen is the real `renderer/` UI, recorded with fake usage data, so no real account ever shows up.

| File | What it is |
| --- | --- |
| `index.html` | The composition: scenes, copy and the timeline |
| `capture.mjs` | Records the widget clips into `assets/clips/<name>.webm` |
| `stage.mjs` | Loads `renderer/index.html` with a stubbed `window.api` and the fake data |
| `frames.mjs` | Frame-exact capture: fake clock, every animation paused and seeked by hand |
| `music.py` | Synthesizes `assets/music.wav` (chiptune, 120 BPM, 92s) |

## Scenes

| Time | Scene |
| --- | --- |
| 0–5s | Hook: the session hits 100% mid-task |
| 5–11s | Clauddy: what it is |
| 11–33s | Every size, one at a time: each lands big in the middle while the rest blurs, then glides into its place in the row |
| 33–40s | Activity scenes: reading, editing, running |
| 40–49.5s | The real %: session, pace projection and weekly, each pointed out |
| 49.5–56.5s | Moods: on fire, sleeping, celebrating |
| 56.5–69.5s | The voice: four remarks in the speech bubble |
| 69.5–75.5s | Claude, Codex and Cursor |
| 75.5–82.5s | Details: by model, by project, 30 days |
| 82.5–92s | Outro: `bunx clauddy` |

## Languages

The copy in `index.html` is Portuguese, and each element carries its English in a `data-en` attribute. The `lang` variable (`pt` by default) picks one at render time. A new line of copy needs both, or the English cut shows it in Portuguese. The widget itself is English in both.

## Run it

```bash
cd tools/showcase
bun install && npx playwright install chromium
node capture.mjs                                 # all clips, or: node capture.mjs trio numbers
uv run --with numpy python music.py
npx hyperframes@0.8.164 snapshot --at 2.8,11,17.6,27 --no-end -o /tmp/snap
npx hyperframes@0.8.164 render -f 30 -q high -o ~/Downloads/clauddy-tour-pt.mp4
npx hyperframes@0.8.164 render -f 30 -q high --variables '{"lang":"en"}' -o ~/Downloads/clauddy-tour.mp4
```

Clips are VP9 with alpha, so they float on the composition's background. Every `<video>` needs an `id`, or HyperFrames renders it frozen (`npx hyperframes@0.8.164 lint` catches that), and both a width and a height: the render letterboxes each frame into the element's box, so a missing side shifts the clip. `snapshot` doesn't show this, only `render` does.

The widget changes? Re-run `capture.mjs`. Each clip's length must cover its scene in `index.html`, and the callouts in the 40–49.5s scene point at rows measured in the widget (move them if its layout changes) and are timed to the `numbers` clip's data pushes (frames 60 and 120). Likewise the size clips run from each one's entrance to the end of their scene, the pet-only hovers land in its spotlight and in the final lineup, and the captions in the voice scene follow the `talk` clip's remarks (frames 10, 105, 200 and 295).
