import json, os, subprocess
from pathlib import Path

HERE = Path(__file__).parent
LANG = os.environ.get("LANG_", "en")
SRC = HERE / "takes" / LANG / "main.webm"


def marker_windows(src=None):
    """Frame-accurate cut points: the take flashes magenta at every beat boundary."""
    src = src or SRC
    r = subprocess.run(["ffprobe", "-v", "error", "-select_streams", "v:0",
                        "-show_entries", "stream=r_frame_rate", "-of", "json", str(src)],
                       capture_output=True, text=True, check=True)
    n, d = json.loads(r.stdout)["streams"][0]["r_frame_rate"].split("/")
    fps = float(n) / float(d)
    p = subprocess.run(["ffmpeg", "-nostdin", "-v", "error", "-i", str(src),
                        "-vf", "scale=8:8", "-f", "rawvideo", "-pix_fmt", "rgb24", "-"],
                       capture_output=True, check=True)
    buf, size = p.stdout, 8 * 8 * 3
    hits = []
    for i in range(len(buf) // size):
        px = buf[i * size:(i + 1) * size]
        r_, g_, b_ = (sum(px[k::3]) / 64 for k in range(3))
        if r_ > 180 and b_ > 180 and g_ < 80:
            hits.append(i)
    wins = []
    for f in hits:
        if wins and f - wins[-1][1] <= 2:
            wins[-1][1] = f
        else:
            wins.append([f, f])
    return fps, [(a / fps, (b + 1) / fps) for a, b in wins]


if __name__ == "__main__":
    fps, wins = marker_windows()
    print(f"[{LANG}] fps={fps}  markers={len(wins)}")
    for i, (a, b) in enumerate(wins):
        print(f"  {i:2d}  {a:7.3f} -> {b:7.3f}")
