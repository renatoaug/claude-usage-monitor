# Showcase — the 100-second product video

A short, music-only product video: an HTML composition animated with GSAP and rendered by [HyperFrames](https://hyperframes.dev). The widget on screen is the real native app, drawn frame by frame from the snapshot gallery's fixtures, so no real account ever shows up.

| File | What it is |
| --- | --- |
| `index.html` | The composition: scenes, copy and the timeline |
| `clips.sh` | Renders the app's clips and encodes each into `assets/clips/<name>.webm` |
| `../../native/src/screens/showcase.rs` | What each clip shows, frame by frame (an ignored test, run by `clips.sh`) |
| `music.py` | Synthesizes `assets/music.wav` (chiptune, 120 BPM, 100 s) |

## Scenes

| Time | Scene |
| --- | --- |
| 0–2s | Opening card, whole from the first frame, so a player's preview shows it |
| 2–7s | Hook: the session hits 100% mid-task |
| 7–13s | Clauddy: what it is |
| 13–35s | Every size, one at a time: each lands big in the middle while the rest blurs, then glides into its place in the row |
| 35–43.5s | The real %: session, its reset and weekly, each pointed out |
| 43.5–50.5s | Pick a pet: the picker's choice walks through every pet |
| 50.5–57s | The holidays' pets: Santa, Reindeer, Gingerbread, Pumpkin |
| 57–63.5s | Moods: working, on fire, sleeping |
| 63.5–76.5s | The voice: four remarks in the speech bubble, from the wizard |
| 76.5–82.5s | Claude, Codex and Cursor, the wizard at the desk |
| 82.5–89.5s | Details: by model, by project, 30 days |
| 89.5–100s | Outro: `bunx clauddy` |

The timeline in `index.html` is written from the hook at 0 and shifted 2s at the end (`tl.shiftChildren`) to make room for the opening; the `data-start` attributes carry the real times.

## Languages

The copy in `index.html` is Portuguese, and each element carries its English in a `data-en` attribute. The `lang` variable (`pt` by default) picks one at render time. A new line of copy needs both, or the English cut shows it in Portuguese. The widget itself is English in both.

## Run it

```bash
cd tools/showcase
./clips.sh                                       # all clips, or: ./clips.sh picker,talk
uv run --with numpy python music.py
npx hyperframes@0.8.164 snapshot --at 2.8,11,17.6,38 --no-end -o /tmp/snap
npx hyperframes@0.8.164 render -f 30 -q high -o /tmp/pt.mp4
npx hyperframes@0.8.164 render -f 30 -q high --variables '{"lang":"en"}' -o /tmp/en.mp4
```

`clips.sh` needs Rust and `ffmpeg`. Each clip is the app's own drawing code at 30 fps on a transparent background: `showcase.rs` sets what is on screen for every frame (the reading, the mood, the size, the pick, the speech bubble) and moves the pet's clock by 1/30 s each time. Clips are VP9 with alpha, so they float on the composition's background. Every `<video>` needs an `id`, or HyperFrames renders it frozen (`npx hyperframes@0.8.164 lint` catches that), and both a width and a height in the clip's aspect: the render letterboxes each frame into the element's box. `snapshot` doesn't show this, only `render` does.

The widget changes? Re-run `clips.sh`. Each clip's length must cover its scene in `index.html`, and some scenes are timed to their clip: the callouts in the 35–43.5s scene point at rows measured in the card (move them if its layout changes) and follow the `numbers` clip's session ticking up (frames 60 and 120); the picker's choice moves every second from frame 30; the pet-only hovers land in its spotlight and in the final lineup; and the captions in the voice scene follow the `talk` clip's remarks (frames 10, 105, 200 and 295).

## Exporting

The render runs ~20 MB; GitHub takes 10 MB at most. This brings it to ~5 MB at the same 1080p and length (dark, mostly still frames compress well) and embeds the opening card as cover art, which Finder uses for the thumbnail:

```bash
ffmpeg -ss 0.3 -i /tmp/en.mp4 -frames:v 1 /tmp/cover.jpg
ffmpeg -i /tmp/en.mp4 -i /tmp/cover.jpg -map 0:v -map 0:a -map 1 \
  -c:v:0 libx264 -preset slow -crf 26 -pix_fmt yuv420p -c:v:1 copy -disposition:v:1 attached_pic \
  -c:a aac -b:a 96k -movflags +faststart ~/Downloads/clauddy-tour.mp4
```
