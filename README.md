# 🟫 Claude Usage Monitor

A cute pixel-art desktop pet for macOS that tracks your Claude Code usage — mirroring the official **Settings → Usage** panel (current session + weekly limits, in tokens & %), with animations.

<p align="center">
  <em>The little terracotta creature lives in the corner of your screen, eats your tokens, and naps when you're idle.</em>
</p>

## What it shows

- **Current session** — real % used + **"resets in Xh Ym"** + session tokens
- **Weekly · all models** — real % used + tokens over the last 7 days
- **Status line** under the pet: `● working · 1.6M tok/min` (or today's tokens when idle)
- **By model · 7 days** — Opus / Sonnet / Haiku / Fable, in tokens
- **30-day map** — colored squares by daily tokens (green = light → red = heavy), with the monthly total

The **percentages are real**, pulled from your account (you log in once — see below). The token counts, by-model breakdown, activity status, and 30-day map come from your local logs (`~/.claude/projects/**/*.jsonl`). Everything is token-based — no dollars.

## Account & live usage

The session/weekly **%** comes straight from your Anthropic account, so it matches the official panel exactly. You connect once via a browser login:

1. Open **⚙ Settings → "Log in with browser"** — your browser opens an Anthropic auth page.
2. Log in, copy the **authentication code** shown, and paste it back into the app → **Connect**.

The token is saved locally (see [Data & privacy](#data--privacy)) and refreshed automatically. **Until you connect**, the limits area shows a *"Connect your account"* prompt instead of percentages.

## The pet's states

| State       | When                          | Animation                            |
| ----------- | ----------------------------- | ------------------------------------ |
| 😌 idle     | no recent activity            | breathes, blinks                     |
| 🤩 working  | Claude active in the last ~8s | hops, rosy cheeks, eats token coins  |
| 🥵 on fire  | session ≥ 90%                 | turns red, shivers, flames           |
| 💤 sleeping | idle for 5+ min               | eyes close, blue zzz, moonlight glow |

Plus a welcome **wave** on launch, a **poke** reaction (click → squish + hearts), and a **celebration** (confetti) when your session resets.

## Install

Requires **macOS** (Apple Silicon), **Node.js 24** (see `.nvmrc`), and **[Bun](https://bun.sh)**. Clone it anywhere — the app keeps its data in `~/.claude-usage-monitor`, independent of where the repo lives.

```bash
git clone https://github.com/renatoaug/claude-usage-monitor.git
cd claude-usage-monitor

nvm install   # use the pinned Node version (reads .nvmrc)
bun install   # install dependencies

bun start     # dev run (live, from source)
# or
bun run pack  # build the real .app
```

`bun run pack` produces **`dist/mac-arm64/Claude Usage Monitor.app`** — drag it to `/Applications`. When run as a packaged app it registers itself in **Login Items**, so it starts with your Mac.

> The app is **unsigned** (personal use). If Gatekeeper complains on first open, right-click the app → **Open** → **Open**.

## Controls

- **Drag** the widget anywhere on screen
- **–** minimizes to just the pet's face (showing the live session %); the **⤢** button or a double-click on the pet expands it back
- **⚙** opens settings (log in, toggle alerts, set thresholds)
- **↗** opens the official Usage page
- **×** quits

## Alerts

Optional **macOS notifications** when your session or weekly usage crosses the thresholds you set (default **80%** and **95%**) — e.g. *"Your session is over 80% — now at 82%"*. They re-arm automatically once usage drops back below a threshold (after a reset). Toggle them and edit the thresholds in **⚙ Settings**.

## Configure (`config.json`)

Settings saved from the UI live in `~/.claude-usage-monitor/config.json`, so you can tweak them without rebuilding:

```jsonc
{
  "alerts": true,            // macOS notifications on/off
  "alertThresholds": [80, 95], // notify when session/week cross these %
  "pollIntervalMs": 4000,    // how often local logs are re-read
  "activeThresholdMs": 8000, // "working" if Claude was active within this window
  "sleepThresholdMs": 300000 // "sleeping" after this much idle time (5 min)
}
```

## Simulate states (`./pet`)

While developing the animations, force any state from the terminal — the app watches `~/.claude-usage-monitor/debug.json` and reacts live (no rebuild needed). Run from the repo root:

```bash
./pet fire        # 🔥 on fire — flames, shivers, red tint
./pet sleeping    # 😴 sleeping — blue zzz, closed eyes, moonlight
./pet working     # 🍴 working — eats token coins and hops
./pet idle        # 🙂 idle — breathe + blink

./pet poke        # 💕 one-shot squish + hearts
./pet celebrate   # 🎉 one-shot jump + confetti burst

./pet auto        # ↩️  release control, back to real usage data
```

## How it works

- **`main.js`** — Electron main process: frameless, transparent, always-on-top window; polls usage; fires macOS notifications; watches `config.json` and `debug.json`.
- **`usage.js`** — reads `~/.claude/projects/**/*.jsonl`, sums tokens per model/day, detects the rolling 5-hour session window, and the working/sleeping activity status.
- **`auth.js`** — OAuth login (PKCE, same public client as Claude Code) that fetches the authoritative usage %. Token stored locally, never committed.
- **`renderer/`** — the pet itself: an SVG pixel sprite, CSS animations, and the Web Animations API for particles.
- **`make-icon.js`** — generates the app icon from the pixel sprite (`build/icon.icns`).

## Data & privacy

Everything lives on your machine, in `~/.claude-usage-monitor/`:

- `auth.json` — your OAuth token (file mode `600`, never committed)
- `config.json` — your alert settings
- `debug.json` — scratch file for the `./pet` simulator

Nothing leaves your machine except the OAuth calls to Anthropic's own login and usage endpoints.

## Dev tooling

- **Bun** for install/scripts, **Node 24** pinned in `.nvmrc`
- **Biome** for format + lint (`bun run check`); a versioned **pre-commit hook** (`.githooks/pre-commit`) auto-formats staged files and blocks on errors. It's wired up automatically on `bun install` (via the `prepare` script).
