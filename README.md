# 🟫 Clauddy

A cute pixel-art desktop pet for macOS that tracks your Claude Code usage — mirroring the official **Settings → Usage** panel (current session + weekly limits, in tokens & %), with animations.

<p align="center">
  <img src="https://raw.githubusercontent.com/renatoaug/claude-usage-monitor/main/docs/media/overview.gif" width="300" alt="Clauddy — the full widget showing session, weekly, by-model and 30-day usage" /><br />
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

Plus a welcome **wave** on launch. You can [poke the pet from the terminal](#play-with-the-pet) too.

### What Claude's up to

While Claude Code is actively working, the pet sets up a little desk scene that
mirrors **what it's doing right now** — inferred from your local logs (the last
tool it used). The status line names the activity, and three of them get their
own animated scene:

<table>
  <tr>
    <td align="center"><img src="https://raw.githubusercontent.com/renatoaug/claude-usage-monitor/main/docs/media/reading.gif" width="280" alt="reading" /><br /><b>reading</b><br /><sub>puts on glasses &amp; flips through docs</sub></td>
    <td align="center"><img src="https://raw.githubusercontent.com/renatoaug/claude-usage-monitor/main/docs/media/editing.gif" width="280" alt="editing" /><br /><b>editing</b><br /><sub>types at the laptop, coffee in reach</sub></td>
  </tr>
  <tr>
    <td colspan="2" align="center"><img src="https://raw.githubusercontent.com/renatoaug/claude-usage-monitor/main/docs/media/running.gif" width="280" alt="running" /><br /><b>running</b><br /><sub>watches a task log tick through its checks</sub></td>
  </tr>
</table>

Other activities — **planning**, **researching**, **delegating**, **waiting** —
show up in the status line as they happen. When Claude goes quiet, the pet drops
back to plain **working** / **idle**.

## Install

**macOS (Apple Silicon)** is the first-class build. **Windows (x64)** works too, and **Linux** is next. The `bunx`/`npx` route below runs on all of them today.

### macOS

Two ways, depending on what you want:

#### 1. Install as an app — opens at login (recommended)

One command — it downloads the latest release and drops it in `/Applications`:

```bash
curl -fsSL https://raw.githubusercontent.com/renatoaug/claude-usage-monitor/main/install.sh | bash
```

**Clauddy is free and open source** — the command above just downloads the latest release from this repo and drops it in `/Applications`, nothing else (you can read [`install.sh`](install.sh) first if you'd like).

Why not a normal download? macOS blocks **unsigned** apps downloaded through a browser with a scary *"damaged, move to Trash"* warning — even when they're perfectly safe. It's a false alarm: the only way to silence it is to pay Apple **$99/year** to sign + notarize, which a free hobby app skips. Files fetched with `curl` aren't flagged, so this method simply **lets your Mac open the app** without the block. It then registers in **Login Items** and starts with your Mac — set it and forget it.

#### 2. Run it via `bunx` (no install)

Needs [Bun](https://bun.sh) (or use `npx` with Node 24):

```bash
bunx clauddy
```

The first run downloads Electron, so give it a moment. Handy for a quick run, but it stays up **only while that command is open** and won't start on its own. Quit it with the **×** button.

### Windows (x64)

The quickest path works the same as macOS — with [Bun](https://bun.sh) or Node 24 installed:

```powershell
bunx clauddy   # or: npx clauddy
```

Prefer a standalone app with no Node/Bun? Grab the **portable zip** (`Clauddy-<version>-win.zip`) from the [latest release](https://github.com/renatoaug/claude-usage-monitor/releases), unzip it anywhere, and run `Clauddy.exe`. Because the app is unsigned, Windows **SmartScreen** shows a "Windows protected your PC" prompt the first time — click **More info → Run anyway**. From then on it starts with Windows.

> Windows builds are produced by the **Build** workflow (Actions ▸ Build) — attaching them to every release automatically is on the roadmap.

### Linux

Coming soon — the build is already wired for `tar.gz` and AppImage. In the meantime, `bunx clauddy` / `npx clauddy` works today.

> The app keeps its data in `~/.claude-usage-monitor`, regardless of platform or how you run it.

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
  "alertThresholds": [80, 95], // notify when session/week cross these % (two levels)
  "fireThreshold": 90, // session % at which the pet catches fire (maxed out stays 100)
  "pollIntervalMs": 4000, // how often local logs are re-read
  "activeThresholdMs": 20000, // "active" if Claude wrote to its logs within this window
  "sleepThresholdMs": 300000, // "sleeping" after this much idle time (5 min)
}
```

## Play with the pet

With the widget running, poke it from the terminal — just for fun:

```bash
bunx clauddy poke        # 💕 squish + hearts
bunx clauddy celebrate   # 🎉 jump + confetti
bunx clauddy fire        # 🔥 on fire
bunx clauddy sleeping    # 😴 blue zzz
bunx clauddy working     # 🍴 eats token coins
bunx clauddy tired       # 🥵 maxed out
bunx clauddy idle        # 🙂 calm
bunx clauddy auto        # ↩️ back to your real usage
```

Each state is written to the data dir the running widget watches, so it reacts
live. (Installed globally? Drop the `bunx`: `clauddy poke`. Working on the repo?
`./pet <state>` does the same.)

## How it works

- **`main.js`** — Electron main process: frameless, transparent, always-on-top window; polls usage; fires macOS notifications; watches `config.json` and `debug.json`.
- **`usage.js`** — reads `~/.claude/projects/**/*.jsonl`, sums tokens per model/day, detects the rolling 5-hour session window, the working/sleeping status, and which activity (reading/editing/running/…) Claude is on from its latest tool use.
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

### Releasing

Releases are **fully automated**. Every push to `main` runs
[semantic-release](https://semantic-release.gitbook.io) (`.github/workflows/release.yml`):
it reads the **Conventional Commits** and, when there's something to ship,
computes the version, builds the macOS app, publishes `clauddy` to npm, and
cuts a GitHub Release with the `.app` zip. Nothing to do by hand — just merge
your PRs.

- `feat:` → minor, `fix:` → patch, `feat!:`/`BREAKING CHANGE` → major.
- `docs:`/`chore:`/`ci:` etc. don't trigger a release.
