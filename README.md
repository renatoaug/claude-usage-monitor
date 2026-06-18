# 🟫 Claude Usage Monitor

A cute pixel-art desktop pet for macOS that tracks your Claude Code usage — mirroring the official **Settings → Usage** panel (current session + weekly limits, in tokens & %), with animations.

<p align="center">
  <em>The little terracotta creature lives in the corner of your screen, eats your tokens, and naps when you're idle.</em>
</p>

## What it shows

- **Current session** — % used + **"resets in Xh Ym"** (real 5-hour window, detected from your activity) + session tokens
- **Weekly · all models** — % used + tokens over the last 7 days (or since your reset anchor)
- **Status line** under the pet: `● working · 1.6M tok/min` (or today's tokens when idle)
- **By model · 7 days** — Opus / Sonnet / Haiku / Fable, in tokens
- **30-day map** — colored squares by daily tokens (green = light → red = heavy), with the monthly total

All values come from your local logs (`~/.claude/projects/**/*.jsonl`), token-based (no dollars).

## The pet's states

| State | When | Animation |
|---|---|---|
| 😌 idle | no recent activity | breathes, blinks |
| 🤩 working | Claude active in the last ~8s | hops, rosy cheeks, eats token coins |
| 🥵 on fire | session ≥ 90% | turns red, shivers, flames |
| 💤 sleeping | idle for 5+ min | eyes close, blue zzz, moonlight glow |

Plus a welcome **wave** on launch, a **poke** reaction (click → squish + hearts), and a **celebration** (confetti) when your session resets.

## Install

Requires **macOS** (Apple Silicon) and **Node.js 18+**.

```bash
# 1. clone into ~/Documents (the app reads its config from this path)
cd ~/Documents
git clone https://github.com/renatoaug/claude-usage-monitor.git
cd claude-usage-monitor

# 2. install dependencies
npm install

# 3a. dev run (live, from source)
npm start

# 3b. or build the real .app
npm run pack
```

`npm run pack` produces **`dist/mac-arm64/Claude Usage Monitor.app`** — drag it to `/Applications`. When run as a packaged app it registers itself in **Login Items**, so it starts with your Mac.

> The app is **unsigned** (personal use). If Gatekeeper complains on first open, right-click the app → **Open** → **Open**.

## Controls

- **Drag** the widget anywhere on screen
- **–** minimizes to just the pet's face (showing the live session %); the **⤢** button or a double-click on the pet expands it back
- **⚙** opens settings (pick your plan, toggle alerts, set thresholds)
- **↗** opens the official Usage page
- **×** quits

## Configure / calibrate (`config.json`)

The packaged app reads `~/Documents/claude-usage-monitor/config.json`, so you can tune without rebuilding. The percentages are **estimates** against token budgets — calibrate them against the official panel.

```jsonc
{
  "plan": "max5x",                   // "pro" | "max5x" | "max20x" — sets the % estimate
  "sessionTokenBudget": 630000000,   // tune until session % matches Settings → Usage
  "weeklyTokenBudget": 3450000000,   // tune until weekly % matches
  "weeklyAnchorIso": null,           // set to an ISO reset time to show "resets in Xd Yh"
  "alerts": true,                    // macOS notifications
  "alertThresholds": [80, 95]        // notify when session/week cross these %
}
```

To get an accurate weekly countdown: take the "Resets in …" value from the official panel, add it to the current time, and put the result in `weeklyAnchorIso` (e.g. `"2026-06-19T03:00:00"`).

## Simulate states (`./pet`)

While developing the animations, force any state from the terminal — the app watches `debug.json` and reacts live (no rebuild needed):

```bash
cd ~/Documents/claude-usage-monitor

./pet fire        # 🔥 on fire — flames, shiver, red tint
./pet sleeping    # 😴 sleeping — blue zzz, closed eyes, moonlight
./pet working     # 🍴 working — eats token coins and hops
./pet idle        # 🙂 idle — breathe + blink

./pet poke        # 💕 one-shot squish + hearts
./pet celebrate   # 🎉 one-shot jump + confetti burst

./pet auto        # ↩️  release control, back to real usage data
```

## How it works

- **`main.js`** — Electron main process: frameless, transparent, always-on-top window; polls usage; fires macOS notifications; watches `config.json` and `debug.json`.
- **`usage.js`** — reads `~/.claude/projects/**/*.jsonl`, sums tokens per model/day, detects the rolling 5-hour session window, derives % from the plan budgets.
- **`auth.js`** — optional OAuth login for *live* %  (PKCE flow, same public client as Claude Code). Token is stored locally in `auth.json` (never committed).
- **`renderer/`** — the pet itself: an SVG pixel sprite, CSS animations, and the Web Animations API for particles.
- **`make-icon.js`** — generates the app icon from the pixel sprite (`build/icon.icns`).

## Notes

- Percentages are **estimates**; dollar amounts are gone by design — everything is in tokens.
- Nothing leaves your machine except the optional OAuth call to Anthropic's own usage endpoint.
