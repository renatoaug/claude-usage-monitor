#!/usr/bin/env python3
"""Generate one narration clip per beat, then verify each one by transcription.

  LANG=en|pt                  which narration.<lang>.json to read (default en)

  TTS_PROVIDER=elevenlabs     the user's cloned voice
    ELEVENLABS_API_KEY=...  TTS_VOICE=<voice_id>
    TTS_MODEL=eleven_multilingual_v2
    TTS_STABILITY=0.45  TTS_SIMILARITY=0.85  TTS_SPEED=1.0
  TTS_PROVIDER=openai         OPENAI_API_KEY, TTS_VOICE=alloy
  TTS_PROVIDER=say            macOS built-in, placeholder quality (default)

Clips land in clips/<lang>/<beat_id>.mp3. Only beats whose text (or voice
config) changed are regenerated — the hash lives in clips/<lang>/manifest.json.
"""
import hashlib, json, os, re, subprocess, sys, unicodedata, urllib.request, uuid
from pathlib import Path

HERE = Path(__file__).parent
LANG = os.environ.get("LANG_", os.environ.get("NARRATION_LANG", "en"))
SCRIPT = HERE / f"narration.{LANG}.json"
CLIPS = HERE / "clips" / LANG
CLIPS.mkdir(parents=True, exist_ok=True)
PROVIDER = os.environ.get("TTS_PROVIDER", "say")
MAX_TRIES = 3


# ---------------- providers ----------------
def synth_say(text, out):
    voice = os.environ.get("TTS_VOICE", "Samantha")
    rate = os.environ.get("TTS_RATE", "172")
    aiff = out.with_suffix(".aiff")
    subprocess.run(["say", "-v", voice, "-r", rate, "-o", str(aiff), text], check=True)
    subprocess.run(["ffmpeg", "-nostdin", "-y", "-loglevel", "error", "-i", str(aiff),
                    "-ar", "44100", "-ac", "1", "-b:a", "128k", str(out)], check=True)
    aiff.unlink()


def synth_elevenlabs(text, out):
    key = os.environ["ELEVENLABS_API_KEY"]
    voice = os.environ["TTS_VOICE"]
    model = os.environ.get("TTS_MODEL", "eleven_multilingual_v2")
    payload = {
        "text": text,
        "model_id": model,
        "voice_settings": {
            "stability": float(os.environ.get("TTS_STABILITY", 0.45)),
            "similarity_boost": float(os.environ.get("TTS_SIMILARITY", 0.85)),
            "speed": float(os.environ.get("TTS_SPEED", 1.0)),
        },
    }
    if model.startswith("eleven_multilingual") or model.startswith("eleven_turbo"):
        payload["language_code"] = LANG
    req = urllib.request.Request(
        f"https://api.elevenlabs.io/v1/text-to-speech/{voice}",
        data=json.dumps(payload).encode(),
        headers={"xi-api-key": key, "Content-Type": "application/json"})
    with urllib.request.urlopen(req) as r:
        out.write_bytes(r.read())


def synth_openai(text, out):
    key = os.environ["OPENAI_API_KEY"]
    req = urllib.request.Request(
        "https://api.openai.com/v1/audio/speech",
        data=json.dumps({
            "model": os.environ.get("TTS_MODEL", "gpt-4o-mini-tts"),
            "voice": os.environ.get("TTS_VOICE", "alloy"),
            "input": text, "response_format": "mp3",
        }).encode(),
        headers={"Authorization": f"Bearer {key}", "Content-Type": "application/json"})
    with urllib.request.urlopen(req) as r:
        out.write_bytes(r.read())


SYNTH = {"say": synth_say, "elevenlabs": synth_elevenlabs, "openai": synth_openai}


# ---------------- verification ----------------
def transcribe(path):
    """ElevenLabs scribe; returns '' when no key is configured (verify is skipped)."""
    key = os.environ.get("ELEVENLABS_API_KEY")
    if not key:
        return ""
    b = uuid.uuid4().hex
    parts = []
    for k, v in (("model_id", "scribe_v1"), ("language_code", LANG)):
        parts.append(f'--{b}\r\nContent-Disposition: form-data; name="{k}"\r\n\r\n{v}\r\n'.encode())
    parts.append(
        f'--{b}\r\nContent-Disposition: form-data; name="file"; filename="a.mp3"\r\n'
        f"Content-Type: audio/mpeg\r\n\r\n".encode() + path.read_bytes() + b"\r\n")
    parts.append(f"--{b}--\r\n".encode())
    req = urllib.request.Request(
        "https://api.elevenlabs.io/v1/speech-to-text", data=b"".join(parts),
        headers={"xi-api-key": key, "Content-Type": f"multipart/form-data; boundary={b}"})
    with urllib.request.urlopen(req) as r:
        return json.loads(r.read()).get("text", "")


def words(s):
    s = unicodedata.normalize("NFKD", s.lower())
    s = "".join(c for c in s if not unicodedata.combining(c))
    return [w for w in re.split(r"[^a-z0-9]+", s) if w]


# the transcriber's own biases — not the voice mispronouncing anything
ALIASES = {"cloud": "claude", "clode": "claude", "claudy": "clauddy", "cloudy": "clauddy",
           "cloudi": "clauddy", "claud": "claude", "claudia": "clauddy"}


# ...and its habit of writing numbers as digits, which no 1:1 word alias can bridge:
# "a hundred percent" comes back as "100%". Applied to both sides before tokenizing.
PHRASES = {"a hundred percent": "100", "one hundred percent": "100",
           "cem por cento": "100", "100 por cento": "100", "100 percent": "100"}


def fold_phrases(s):
    out = s.lower()
    for k, v in PHRASES.items():
        out = out.replace(k, v)
    return out


def coverage(said, want):
    got = [ALIASES.get(w, w) for w in words(fold_phrases(said))]
    exp = [ALIASES.get(w, w) for w in words(fold_phrases(want))]
    pool = list(got)
    missing = []
    for w in exp:
        if w in pool:
            pool.remove(w)
        else:
            missing.append(w)
    return 1 - len(missing) / max(1, len(exp)), missing


def probe(p, key):
    r = subprocess.run(["ffprobe", "-v", "error", "-show_entries", f"format={key}",
                        "-of", "csv=p=0", str(p)], capture_output=True, text=True, check=True)
    return float(r.stdout.strip())


def max_silence(p):
    r = subprocess.run(["ffmpeg", "-nostdin", "-i", str(p),
                        "-af", "silencedetect=noise=-38dB:d=0.35", "-f", "null", "-"],
                       capture_output=True, text=True)
    gaps = [float(l.split("silence_duration:")[1]) for l in r.stderr.splitlines()
            if "silence_duration:" in l]
    return max(gaps) if gaps else 0.0


# ---------------- main ----------------
def main():
    beats = json.loads(SCRIPT.read_text())["beats"]
    mpath = CLIPS / "manifest.json"
    manifest = json.loads(mpath.read_text()) if mpath.exists() else {}
    cfg = f"{PROVIDER}:{os.environ.get('TTS_VOICE','')}:{os.environ.get('TTS_MODEL','')}:" \
          f"{os.environ.get('TTS_STABILITY','')}:{os.environ.get('TTS_SIMILARITY','')}:" \
          f"{os.environ.get('TTS_SPEED','')}"
    out, bad = {}, []
    print(f"[{LANG}] provider={PROVIDER} voice={os.environ.get('TTS_VOICE','-')}\n")
    for b in beats:
        mp3 = CLIPS / f"{b['id']}.mp3"
        h = hashlib.sha256((cfg + b["text"]).encode()).hexdigest()[:16]
        if manifest.get(b["id"], {}).get("hash") == h and mp3.exists():
            info = manifest[b["id"]]
            print(f"  = {b['id']:12s} {info['duration']:5.2f}s  (cached)")
            out[b["id"]] = info
            continue
        # expressive models are stochastic — re-roll a clip that drops words
        for attempt in range(1, MAX_TRIES + 1):
            SYNTH[PROVIDER](b["text"], mp3)
            d, sil = probe(mp3, "duration"), max_silence(mp3)
            cov, missing = coverage(transcribe(mp3), b["text"])
            ok = cov >= 0.92 and sil <= 0.75
            if ok or attempt == MAX_TRIES:
                break
            print(f"    ↻ {b['id']} attempt {attempt}: coverage {cov:.0%}, gap {sil:.2f}s")
        info = {"hash": h, "duration": d, "max_silence": sil, "coverage": cov}
        flag = ""
        if cov < 0.92:
            flag = f"  ⚠ missing {missing[:6]}"
            bad.append(b["id"])
        elif sil > 0.75:
            flag = f"  ⚠ {sil:.2f}s internal gap"
            bad.append(b["id"])
        print(f"  + {b['id']:12s} {d:5.2f}s  gap {sil:4.2f}s  said {cov:.0%}{flag}")
        out[b["id"]] = info
    mpath.write_text(json.dumps(out, indent=2))
    total = sum(v["duration"] for v in out.values())
    print(f"\n  {len(beats)} clips · narration {total:.1f}s")
    if bad:
        print(f"  ⚠ rephrase these beats rather than retrying: {', '.join(bad)}")
        sys.exit(1)


main()
