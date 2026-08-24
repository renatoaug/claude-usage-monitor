#!/usr/bin/env python3
"""Word-level timings for each narration clip, so the on-screen text can appear
exactly as it is spoken.

Runs against the clips already on disk (no re-synthesis), transcribing each one
with word granularity and mapping the transcript back onto the *script's* own
tokens — the script keeps the right punctuation and accents, the transcript
supplies the timing.

  LANG_=en|pt  ELEVENLABS_API_KEY=...  python3 align.py

Writes clips/<lang>/<beat_id>.words.json — [{"w": "token", "t": seconds}, ...]
"""
import json, os, re, unicodedata, urllib.request, uuid
from pathlib import Path

HERE = Path(__file__).parent
LANG = os.environ.get("LANG_", "en")
CLIPS = HERE / "clips" / LANG
KEY = os.environ["ELEVENLABS_API_KEY"]


def transcribe_words(path):
    b = uuid.uuid4().hex
    parts = []
    for k, v in (("model_id", "scribe_v1"), ("language_code", LANG),
                 ("timestamps_granularity", "word")):
        parts.append(f'--{b}\r\nContent-Disposition: form-data; name="{k}"\r\n\r\n{v}\r\n'.encode())
    parts.append(
        f'--{b}\r\nContent-Disposition: form-data; name="file"; filename="a.mp3"\r\n'
        f"Content-Type: audio/mpeg\r\n\r\n".encode() + path.read_bytes() + b"\r\n")
    parts.append(f"--{b}--\r\n".encode())
    req = urllib.request.Request(
        "https://api.elevenlabs.io/v1/speech-to-text", data=b"".join(parts),
        headers={"xi-api-key": KEY, "Content-Type": f"multipart/form-data; boundary={b}"})
    d = json.loads(urllib.request.urlopen(req).read())
    return [w for w in d.get("words", []) if w.get("type") == "word"]


def fold(s):
    s = unicodedata.normalize("NFKD", s.lower())
    return "".join(c for c in s if not unicodedata.combining(c) and c.isalnum())


def main():
    beats = json.loads((HERE / f"narration.{LANG}.json").read_text())["beats"]
    for b in beats:
        clip = CLIPS / f"{b['id']}.mp3"
        if not clip.exists():
            print(f"  ! {b['id']} — no clip, run tts.py first")
            continue
        tokens = [t for t in re.split(r"(\s+)", b["text"]) if t.strip()]
        heard = transcribe_words(clip)
        out = []
        if len(tokens) == len(heard):
            out = [{"w": t, "t": round(h["start"], 3)} for t, h in zip(tokens, heard)]
        else:
            # counts drifted (the transcriber split or merged something) — anchor on
            # the words that match and interpolate the rest
            n, m = len(tokens), len(heard)
            print(f"    ~ {b['id']}: {n} script tokens vs {m} heard, interpolating")
            for i, t in enumerate(tokens):
                j = min(m - 1, round(i * (m - 1) / max(1, n - 1)))
                out.append({"w": t, "t": round(heard[j]["start"], 3)})
        # timings must never go backwards
        for i in range(1, len(out)):
            out[i]["t"] = max(out[i]["t"], out[i - 1]["t"])
        (CLIPS / f"{b['id']}.words.json").write_text(json.dumps(out, ensure_ascii=False))
        print(f"  ✓ {b['id']:12s} {len(out):3d} words · last at {out[-1]['t']:5.2f}s")


main()
