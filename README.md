# 🟫 Claude Usage Monitor

A cute pixel-art desktop pet for macOS that tracks your Claude Code usage — mirroring the official **Settings → Usage** panel (current session + weekly limits, in tokens & %), with animations.

<p align="center">
  <img src="https://raw.githubusercontent.com/renatoaug/claude-usage-monitor/main/docs/media/overview.gif" width="300" alt="Claude Usage Monitor — the full widget showing session, weekly, by-model and 30-day usage" /><br />
  <em>A little terracotta creature that lives in the corner of your screen, eats your tokens, and naps when you're idle.</em>
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

The token is saved locally (see [Data & privacy](#data--privacy)) and refreshed automatically. **Until you connect**, the limits area shows a _"Connect your account"_ prompt instead of percentages.

## The pet's states

<table>
  <tr>
    <td align="center"><img src="https://raw.githubusercontent.com/renatoaug/claude-usage-monitor/main/docs/media/idle.gif" width="280" alt="idle" /><br /><b>idle</b><br /><sub>breathes &amp; blinks</sub></td>
    <td align="center"><img src="https://raw.githubusercontent.com/renatoaug/claude-usage-monitor/main/docs/media/working.gif" width="280" alt="working" /><br /><b>working</b><br /><sub>hops &amp; eats token coins</sub></td>
  </tr>
  <tr>
    <td align="center"><img src="https://raw.githubusercontent.com/renatoaug/claude-usage-monitor/main/docs/media/on-fire.gif" width="280" alt="on fire" /><br /><b>on fire</b><br /><sub>session ≥ 90% → red, shivers, flames</sub></td>
    <td align="center"><img src="https://raw.githubusercontent.com/renatoaug/claude-usage-monitor/main/docs/media/tired.gif" width="280" alt="maxed out" /><br /><b>maxed out</b><br /><sub>session at 100% → drained, slumped, sweating</sub></td>
  </tr>
  <tr>
    <td align="center"><img src="https://raw.githubusercontent.com/renatoaug/claude-usage-monitor/main/docs/media/sleeping.gif" width="280" alt="sleeping" /><br /><b>sleeping</b><br /><sub>idle 5+ min → blue zzz &amp; moonlight</sub></td>
    <td align="center"><img src="https://raw.githubusercontent.com/renatoaug/claude-usage-monitor/main/docs/media/poke.gif" width="280" alt="poke" /><br /><b>poke</b><br /><sub>click the pet → squish &amp; hearts</sub></td>
  </tr>
  <tr>
    <td colspan="2" align="center"><img src="https://raw.githubusercontent.com/renatoaug/claude-usage-monitor/main/docs/media/celebrate.gif" width="280" alt="celebrate" /><br /><b>celebrate</b><br /><sub>session resets → jump &amp; confetti</sub></td>
  </tr>
</table>

Plus a welcome **wave** on launch. You can preview any state from the terminal with [`./pet`](#simulate-states-pet).

## Install

macOS (Apple Silicon).

**Run it instantly** (no clone) — needs [Bun](https://bun.sh) or Node 24:

```bash
bunx claude-usage-monitor
# or: npx claude-usage-monitor
```

The first run downloads Electron, so give it a moment. The widget appears in the corner; quit it with the **×** button.

**Install the real `.app`** (gets a proper app bundle that opens at login):

```bash
git clone https://github.com/renatoaug/claude-usage-monitor.git
cd claude-usage-monitor

nvm install   # use the pinned Node version (reads .nvmrc)
bun install   # install dependencies

bun run pack  # → dist/mac-arm64/Claude Usage Monitor.app
```

Drag the `.app` to `/Applications`. When run as a packaged app it registers itself in **Login Items**, so it starts with your Mac. (The `bunx` quick-run mode doesn't add a login item.)

> The app is **unsigned** (personal use). If Gatekeeper complains on first open, right-click the app → **Open** → **Open**.

The app keeps its data in `~/.claude-usage-monitor`, independent of how you run it.

## Controls

- **Drag** the widget anywhere on screen
- **–** minimizes to just the pet's face (showing the live session %); the **⤢** button or a double-click on the pet expands it back
- **⚙** opens settings (log in, toggle alerts, set thresholds)
- **↗** opens the official Usage page
- **×** quits

## Alerts

Optional **macOS notifications** when your session or weekly usage crosses the thresholds you set (default **80%** and **95%**) — e.g. _"Your session is over 80% — now at 82%"_. They re-arm automatically once usage drops back below a threshold (after a reset). Toggle them and edit the thresholds in **⚙ Settings**.

## Configure (`config.json`)

Settings saved from the UI live in `~/.claude-usage-monitor/config.json`, so you can tweak them without rebuilding:

```jsonc
{
  "alerts": true, // macOS notifications on/off
  "alertThresholds": [80, 95], // notify when session/week cross these %
  "pollIntervalMs": 4000, // how often local logs are re-read
  "activeThresholdMs": 8000, // "working" if Claude was active within this window
  "sleepThresholdMs": 300000, // "sleeping" after this much idle time (5 min)
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

./pet auto        # ↩️ release control, back to real usage data
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
