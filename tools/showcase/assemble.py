#!/usr/bin/env python3
"""Trim the take by its marker frames, concat, mix narration, write out/<lang>-<ver>.mp4."""
import json, os, subprocess, sys
from pathlib import Path

from markers import marker_windows

HERE = Path(__file__).parent
LANG = os.environ.get("LANG_", "en")
SRC = HERE / "takes" / LANG / "main.webm"
CUTS = json.loads((HERE / "takes" / LANG / "cuts.json").read_text())
LEAD = 0.18   # narration starts just after the cut
VER = sys.argv[1] if len(sys.argv) > 1 else "v1"
OUT = HERE / "out" / f"{LANG}-{VER}.mp4"
OUT.parent.mkdir(exist_ok=True)
TMP = HERE / "tmp" / LANG
TMP.mkdir(parents=True, exist_ok=True)


def run(a):
    subprocess.run(a, check=True, stdin=subprocess.DEVNULL,
                   stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)


fps, wins = marker_windows(SRC)
assert len(wins) == len(CUTS), f"{len(wins)} markers vs {len(CUTS)} cuts"

# segment i runs from the end of its own marker to the start of the next one
segs = []
for i, c in enumerate(CUTS[:-1]):
    if c["id"].startswith("__"):
        continue
    start, end = wins[i][1], wins[i + 1][0]
    segs.append({"id": c["id"], "start": start, "end": end, "dur": end - start})

print(f"[{LANG}] segments:")
for s in segs:
    print(f"  {s['id']:12s} {s['start']:7.2f} -> {s['end']:7.2f}  ({s['dur']:.2f}s)")

parts = []
for i, s in enumerate(segs):
    p = TMP / f"seg{i:02d}.mp4"
    run(["ffmpeg", "-nostdin", "-y", "-v", "error", "-ss", f"{s['start']:.3f}",
         "-t", f"{s['dur']:.3f}", "-i", str(SRC),
         "-vf", "scale=1920:1080:flags=lanczos,fps=30,format=yuv420p",
         "-c:v", "libx264", "-preset", "slow", "-crf", "19", "-an", str(p)])
    parts.append(p)

lst = TMP / "list.txt"
lst.write_text("".join(f"file '{p}'\n" for p in parts))
silent = TMP / "silent.mp4"
run(["ffmpeg", "-nostdin", "-y", "-v", "error", "-f", "concat", "-safe", "0",
     "-i", str(lst), "-c", "copy", str(silent)])

total = sum(s["dur"] for s in segs)

ins, filt, labels = [], [], []
t = 0.0
for i, s in enumerate(segs):
    ins += ["-i", str(HERE / "clips" / LANG / f"{s['id']}.mp3")]
    delay = int((t + LEAD) * 1000)
    filt.append(f"[{i + 1}:a]adelay={delay}|{delay},volume=1.0[a{i}]")
    labels.append(f"[a{i}]")
    t += s["dur"]
filt.append(f"{''.join(labels)}amix=inputs={len(segs)}:normalize=0:duration=longest[mix]")
filt.append(f"[mix]apad,atrim=0:{total:.3f},afade=t=out:st={total - 0.6:.3f}:d=0.6[aout]")

run(["ffmpeg", "-nostdin", "-y", "-v", "error", "-i", str(silent), *ins,
     "-filter_complex", ";".join(filt), "-map", "0:v", "-map", "[aout]",
     "-vf", f"fade=t=in:st=0:d=0.6,fade=t=out:st={total - 0.8:.3f}:d=0.8",
     "-c:v", "libx264", "-preset", "veryslow", "-crf", "25", "-pix_fmt", "yuv420p",
     "-profile:v", "high", "-level", "4.0", "-movflags", "+faststart",
     "-c:a", "aac", "-b:a", "128k", str(OUT)])

print(f"\n  → {OUT.relative_to(HERE)}  ({total:.1f}s, {OUT.stat().st_size / 1e6:.1f} MB)")
