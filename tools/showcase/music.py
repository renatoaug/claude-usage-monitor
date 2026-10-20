# Chiptune loop, 120 BPM, 46 bars (92 s): pulse-wave arpeggio, triangle bass,
# noise drums, a soft pad underneath. Writes assets/music.wav.
#   uv run --with numpy python music.py
import wave

import numpy as np

SR = 44100
BPM = 120
BEAT = 60 / BPM
BAR = 4 * BEAT
BARS = 46
END = int(SR * BAR * BARS)
N = END + SR * 2
rng = np.random.default_rng(7)


def hz(m):
    return 440 * 2 ** ((m - 69) / 12)


def pulse(f, n, duty=0.25):
    tt = np.arange(n) / SR
    return np.where((tt * f) % 1 < duty, 1.0, -1.0)


def tri(f, n):
    tt = np.arange(n) / SR
    return 2 * np.abs(2 * ((tt * f) % 1) - 1) - 1


def lp(x, c):
    a = np.exp(-2 * np.pi * c / SR)
    y = np.zeros_like(x)
    p = 0.0
    for i in range(len(x)):
        p = (1 - a) * x[i] + a * p
        y[i] = p
    return y


def at(bar, beat=0.0):
    return int((bar * BAR + beat * BEAT) * SR)


pad = np.zeros(N)
lead = np.zeros(N)
bass = np.zeros(N)
drums = np.zeros(N)

# C - G - Am - F, sunny
prog = [[60, 64, 67, 72], [55, 59, 62, 67], [57, 60, 64, 69], [53, 57, 60, 65]]
roots = [36, 43, 45, 41]
# a hook that answers itself every two bars
hook = [76, 79, 76, 72, 74, 76, 79, 81, 79, 76, 74, 72, 74, 76, 72, 67]

for b in range(BARS):
    ch = prog[b % 4]
    s = at(b)
    n = int(BAR * SR)
    tt = np.arange(n) / SR
    env = np.minimum(1, tt / 0.3) * np.exp(-tt * 0.4)
    for m in ch[:3]:
        pad[s : s + n] += np.sin(2 * np.pi * hz(m) * tt) * env * 0.05
    # arpeggio in 16ths from bar 1
    if b >= 1:
        seq = [ch[0], ch[1], ch[2], ch[3], ch[2], ch[1]]
        for k in range(16):
            ss = at(b, k / 4)
            nn = int(BEAT / 4 * SR * 0.8)
            e = np.exp(-np.arange(nn) / SR * 14)
            arp = pulse(hz(seq[k % 6] + 12), nn, 0.125) * e * 0.035
            lead[ss : ss + nn] += arp
    # the hook on top, 8ths, from bar 4 until the outro
    if 4 <= b < 44:
        for k in range(8):
            m = hook[(b % 2) * 8 + k]
            ss = at(b, k / 2)
            nn = int(BEAT / 2 * SR * 0.9)
            e = np.minimum(1, np.arange(nn) / SR / 0.005) * np.exp(-np.arange(nn) / SR * 5)
            lead[ss : ss + nn] += pulse(hz(m), nn, 0.5) * e * 0.05
    # triangle bass, octave bounce, from bar 2
    if b >= 2:
        r = roots[b % 4]
        for k in range(8):
            ss = at(b, k / 2)
            nn = int(BEAT / 2 * SR * 0.85)
            e = np.exp(-np.arange(nn) / SR * 4)
            bass[ss : ss + nn] += tri(hz(r + (12 if k % 2 else 0)), nn) * e * 0.22
    # drums: hats from bar 1, kick + snare from bar 4 until the outro
    for k in range(4):
        bs = at(b, k)
        if 4 <= b < 44:
            nn = int(0.25 * SR)
            tt2 = np.arange(nn) / SR
            f = 45 + 140 * np.exp(-tt2 * 40)
            drums[bs : bs + nn] += np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-tt2 * 12) * 0.5
            if k in (1, 3):
                nn = int(0.18 * SR)
                tt2 = np.arange(nn) / SR
                drums[bs : bs + nn] += rng.uniform(-1, 1, nn) * np.exp(-tt2 * 22) * 0.2
        if b >= 1:
            for h in (0, 1):
                hs = bs + int(h * BEAT / 2 * SR)
                nn = int(0.04 * SR)
                hat = rng.uniform(-1, 1, nn)
                hat = hat - lp(hat, 7000)
                drums[hs : hs + nn] += hat * np.exp(-np.arange(nn) / SR * 70) * (0.09 if h else 0.05)

# a short rise into bar 4, where the beat drops
rs = at(3)
rn = int(BAR * SR)
tt = np.arange(rn) / SR
noise = rng.uniform(-1, 1, rn)
drums[rs : rs + rn] += (noise - lp(noise, 2000)) * (tt / BAR) ** 2 * 0.1

duck = np.ones(N)
for i in range(16, 176):
    s = at(0, i)
    nn = int(BEAT * SR)
    duck[s : s + nn] = 1 - 0.4 * np.exp(-np.arange(nn) / SR * 9)

bass = lp(bass, 1200)
mix = (pad + lead + bass) * duck + drums
fade = int(2.0 * SR)
mix[END - fade : END] *= np.linspace(1, 0, fade)
mix = mix[:END]

# a little width: the lead slightly delayed on the right
d = int(0.011 * SR)
L = mix.copy()
R = mix.copy()
R[d:] += lead[: END - d] * duck[: END - d] * 0.4
L += lead[:END] * duck[:END] * 0.15
st = np.stack([L, R], 1)
st /= np.max(np.abs(st)) * 1.1
st = np.tanh(st * 1.25) / np.tanh(1.25) * 0.9

w = wave.open('assets/music.wav', 'wb')
w.setnchannels(2)
w.setsampwidth(2)
w.setframerate(SR)
w.writeframes((st * 32767).astype('<i2').tobytes())
w.close()
print('ok', END / SR)
