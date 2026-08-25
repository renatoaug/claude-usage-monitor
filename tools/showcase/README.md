# Showcase — the narrated README tour

Produces `docs/media/tour.mp4` (English) and `docs/media/tour-pt.mp4` (Portuguese):
a narrated walkthrough of the widget, title card to outro.

The widget on screen is the **real** `renderer/` UI — Playwright loads
`renderer/index.html` with a stubbed `window.api` (the Electron preload bridge)
and pushes fake usage into it, so no real account data is ever recorded.

## Files

| file | what it does |
| --- | --- |
| `narration.<lang>.json` | the script — beats, kickers, spoken lines. Single source of truth. |
| `tts.py` | one clip per beat in `clips/<lang>/`; synthesizes, then verifies by transcription |
| `align.py` | word-level timings for each clip, so the on-screen text appears as it is spoken |
| `record.mjs` | one continuous Playwright take, magenta marker frames at beat boundaries |
| `markers.py` | scans the take for those frames to get exact cut points |
| `assemble.py` | trims by markers, concats, mixes narration, writes `out/<lang>-vN.mp4` |

Generated `clips/`, `takes/`, `tmp/`, `frames/` and `out/` are gitignored.

## Run it

```bash
cd tools/showcase
bun install && npx playwright install chromium

export TTS_PROVIDER=elevenlabs
export ELEVENLABS_API_KEY=...          # never commit this
export TTS_VOICE=<your cloned voice id>
export TTS_MODEL=eleven_multilingual_v2
export TTS_STABILITY=0.45 TTS_SIMILARITY=0.85 TTS_SPEED=1.0

for L in en pt; do
  LANG_=$L python3 tts.py       # clips/<lang>/ + manifest.json with durations
  LANG_=$L python3 align.py     # clips/<lang>/<beat>.words.json — per-word timings
  LANG_=$L node record.mjs      # takes/<lang>/main.webm + cuts.json
  LANG_=$L python3 assemble.py v1
done
```

Beat durations come from the clips, so **TTS runs first** — the recorder holds
each scene for exactly its narration length plus a breath. `TTS_PROVIDER=openai`
(`OPENAI_API_KEY`, `TTS_VOICE=alloy`) and `TTS_PROVIDER=say` (macOS built-in,
placeholder quality) also work.

## Verification

`tts.py` transcribes every clip it generates (ElevenLabs `scribe_v1`) and checks
the transcript against the script. A clip that drops words or opens a gap wider
than 0.75s is re-rolled up to three times; if it still fails the run exits
non-zero and names the beat — **rephrase it rather than retrying**, expressive
models mangle the same phrase consistently.

`ALIASES` in `tts.py` absorbs the transcriber's own biases (it hears "Claude" as
"cloud"); that is not the voice mispronouncing anything. `PHRASES` does the same
for spans no 1:1 word alias can bridge — the transcriber writes numbers as
digits, so "a hundred percent" comes back as "100%". Extend either one; don't
lower the coverage threshold.

Before shipping a round, tile the render and look at every frame:

```bash
ffmpeg -i out/en-v1.mp4 -vf "fps=1/6,scale=430:-1,tile=5x3" -frames:v 1 sheet.png
ffmpeg -i out/en-v1.mp4 -af silencedetect=noise=-45dB:d=1.0 -f null -   # gaps
```

## How the text animates

The line on screen is the line being narrated, so it is revealed one word at a
time, each word delayed by **its own timestamp in the clip** — never by a guessed
offset. `align.py` transcribes each clip with word granularity and maps the
transcript back onto the script's own tokens (the script keeps the punctuation
and accents, the transcript supplies the timing).

`record.mjs` renders one `<span class="w" style="--d:1240ms">` per word; CSS does
the rest — rise, unblur and a spring overshoot on entry, a fast reverse-stagger
on exit. Words listed in a beat's `emph` array get the coral treatment and a
scale pop. `PAD_LEAD` in `record.mjs` must match `LEAD` in `assemble.py`, or the
words drift against the voice.

Re-run `align.py` whenever a clip is re-synthesized — new audio means new timings.

## Changing the script

Edit `narration.<lang>.json`. Keep beat `id`s stable — clips, markers and cut
points all key off them, and only beats whose text changed get re-synthesized.
Adding a beat means adding a matching scene in `record.mjs`: `exitCopy()` clears
the previous line, `prepare()` builds the word spans, `ring()` highlights an
element, `marker()` stamps the cut, `revealCopy()` starts the reveal, and
`push()` feeds usage data.

A voice swap changes **every** duration — that is why the take is re-recorded
rather than re-cut.
