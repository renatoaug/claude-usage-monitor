# 🟫 Clauddy

A cute pixel-art desktop pet for macOS that tracks your Claude Code usage — mirroring the official **Settings → Usage** panel (current session + weekly limits, in tokens & %), with animations.

<p align="center">

https://github.com/user-attachments/assets/dbe00d9a-b49c-48ea-941c-76c517dec358

<em>A little terracotta creature that lives in the corner of your screen, eats your tokens, and naps when you're idle.</em>
</p>

## What it shows

- **Current session** — real % used + **"resets in Xh Ym"** + session tokens, and a projection of where that pace is taking you (see [Burn rate](#burn-rate))
- **Weekly · all models** — real % used + tokens over the last 7 days
- **Status line** under the pet: `● working · 1.6M tok/min` (or today's tokens when idle)
- **By model · 7 days** — Opus / Sonnet / Haiku / Fable, in tokens
- **By project · 7 days** — which repo actually ate the week, ranked, with the tail folded into `other`
- **30-day map** — colored squares by daily tokens (green = light → red = heavy), with the monthly total

The **percentages are real**, pulled from your account (you log in once — see below). The token counts, the by-model and by-project breakdowns, activity status, and 30-day map come from your local logs (`~/.claude/projects/**/*.jsonl`). Everything is token-based — no dollars.

## Account & live usage

The session/weekly **%** comes straight from your Anthropic account, so it matches the official panel exactly. You connect once via a browser login:

1. Open **⚙ Settings → "Log in with browser"** — your browser opens an Anthropic auth page.
2. Log in, copy the **authentication code** shown, and paste it back into the app → **Connect**.

The token is saved locally (see [Data & privacy](#data--privacy)) and refreshed automatically. **Until you connect**, the limits area shows a _"Connect your account"_ prompt instead of percentages.

## Burn rate

Knowing you're at **82%** with **1h 12m** left on the window still leaves you doing arithmetic in your head. So Clauddy does it for you: it fits the slope of your recent usage and projects when you'd hit 100% — showing one extra line under the session bar:

- **`~35m left at this pace`** (in coral) — you'd run out before the window resets. Ease off, or wrap up.
- **`resets before you run out`** — the reset gets there first. Carry on.

The slope is fitted over your **session tokens** rather than the account %. The % is the number you care about, but it arrives as a whole number every ~5 minutes — over a short window the whole signal is a single `16 → 17` step, which throws the fitted pace off by multiples. Local-log tokens step too — one jump per assistant turn — but in increments some 10–20× finer, so the slope is far steadier; the account % then anchors it, converting tokens into % and re-calibrating on every poll.

It reads your **recent** pace, not the session average: go quiet for a few minutes and the projection eases off, which is the point.

It only appears once there's enough to say honestly — roughly 5 minutes into a session — and stays hidden while you're idle, when the pace is flat, or right after a reset. A projection is a projection: change your pace and it changes with you.

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

**macOS (Apple Silicon)** is the first-class build. **Windows (x64)** and **Linux (x64)** work too. The `bunx`/`npx` route below runs on all of them today.

### macOS

Two ways, depending on what you want:

#### 1. Install as an app — opens at login (recommended)

One command — it downloads the latest release and drops it in `/Applications`:

```bash
curl -fsSL https://raw.githubusercontent.com/renatoaug/claude-usage-monitor/main/install.sh | bash
```

**Clauddy is free and open source** — the command above just downloads the latest release from this repo and drops it in `/Applications`, nothing else (you can read [`install.sh`](install.sh) first if you'd like).

Once installed, you can update in-app: **⚙ Settings → Check for updates → Update now** runs the same installer and relaunches the new build.

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

Prefer a standalone app with no Node/Bun? Grab the **portable zip** (`Clauddy-<version>-win-x64.zip`) from the [latest release](https://github.com/renatoaug/claude-usage-monitor/releases), unzip it anywhere, and run `Clauddy.exe`. Because the app is unsigned, Windows **SmartScreen** shows a "Windows protected your PC" prompt the first time — click **More info → Run anyway**. From then on it starts with Windows.

### Linux (x64)

The quickest path works the same as macOS — with [Bun](https://bun.sh) or Node 24 installed:

```bash
bunx clauddy   # or: npx clauddy
```

Prefer a standalone app? Grab the **AppImage** or **tar.gz** (`Clauddy-<version>-linux-x86_64.AppImage` / `Clauddy-<version>-linux-x64.tar.gz`) from the [latest release](https://github.com/renatoaug/claude-usage-monitor/releases), then:

```bash
chmod +x Clauddy-*.AppImage
./Clauddy-*.AppImage
```

Want the pet icon to show up in your app menu / taskbar too, instead of a generic icon? Running the AppImage directly doesn't register it anywhere — GNOME (and most Wayland desktops) only pick up an app's icon from an installed `.desktop` entry. One-time setup, right next to the AppImage:

```bash
./Clauddy-*.AppImage --appimage-extract clauddy.desktop >/dev/null
./Clauddy-*.AppImage --appimage-extract usr/share/icons/hicolor/512x512/apps/clauddy.png >/dev/null
mkdir -p ~/.local/share/applications ~/.local/share/icons/hicolor/512x512/apps
cp squashfs-root/usr/share/icons/hicolor/512x512/apps/clauddy.png ~/.local/share/icons/hicolor/512x512/apps/clauddy.png
sed "s|^Exec=.*|Exec=$(readlink -f Clauddy-*.AppImage) --no-sandbox %U|" squashfs-root/clauddy.desktop > ~/.local/share/applications/clauddy.desktop
rm -rf squashfs-root
update-desktop-database ~/.local/share/applications 2>/dev/null
```

> The system tray icon needs an indicator extension on vanilla GNOME (e.g. "AppIndicator and KStatusNotifier Item Support") — it works out of the box on Cinnamon, KDE, and XFCE. Autostart-at-login is wired up via an XDG `.desktop` entry in `~/.config/autostart/`.

> The app keeps its data in `~/.claude-usage-monitor`, regardless of platform or how you run it.

## Controls

- **Drag** the widget anywhere on screen
- **–** minimizes to just the pet's face (showing the live session %); the **⤢** button or a double-click on the pet expands it back
- **⚙** opens settings (log in, toggle alerts, set thresholds, pick the display mode)
- **↗** opens the official Usage page
- **×** quits

### Floating or menu bar

Under **⚙ Settings → Display** you can pick where Clauddy lives:

- **Floating pet** — the always-on widget in the corner (default).
- **Menu bar** — a small pet icon in the macOS menu bar showing your live session **%** (it turns 🔥 near your limit). Click it to pop open the full pet + usage panel; click away to dismiss. Right-click for a quick menu.

Switching is instant — no restart. (On Windows/Linux the icon lives in the system tray; the live % shows in its tooltip.)

## Alerts

Optional **macOS notifications**, toggled (with their thresholds) in **⚙ Settings**:

| Notification | When |
| --- | --- |
| _Session at 82%_ — `2h 39m left · resets 6:50 PM` | Your session crosses a threshold (default **80%** and **95%**) |
| _Weekly usage at 84%_ — `resets Fri 7:00 AM` | Same, for the weekly limit |
| _Fable weekly at 84%_ — `resets Fri 7:00 AM` | Same, for a per-model weekly limit |
| _Session window reset_ — `full budget again` | A session you had pushed past 80% rolls over |
| _Clauddy lost access to your usage_ | The OAuth token expired or was revoked, so the % went back to being an estimate |

The last threshold you set is the only one that makes a sound; the earlier ones arrive silently. Clicking any of them brings the widget to the front. Each fires once and re-arms when usage drops back below — remembered across restarts, so relaunching at 85% doesn't repeat an alert you already dismissed.

Percentages come from your account when you're logged in, and fall back to the local token estimate when you're not.

The first alert asks macOS for permission; after that the app shows up in **System Settings > Notifications** like any other.

## Configure (`config.json`)

Settings saved from the UI live in `~/.claude-usage-monitor/config.json`, so you can tweak them without rebuilding:

```jsonc
{
  "mode": "floating", // "floating" pet in the corner, or "menubar" popover
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
- **`usage.js`** — reads `~/.claude/projects/**/*.jsonl`, sums tokens per model/project/day, detects the rolling 5-hour session window, the working/sleeping status, and which activity (reading/editing/running/…) Claude is on from its latest tool use.
- **`auth.js`** — OAuth login (PKCE, same public client as Claude Code) that fetches the authoritative usage %. Token stored locally, never committed.
- **`renderer/`** — the pet itself: an SVG pixel sprite, CSS animations, and the Web Animations API for particles.
- **`make-icon.js`** — generates the app icon from the pixel sprite (`build/icon.icns`).

## Data & privacy

Everything lives on your machine, in `~/.claude-usage-monitor/`:

- `auth.json` — your OAuth token (file mode `600`, never committed)
- `config.json` — your alert settings
- `debug.json` — scratch file for the `./pet` simulator

Nothing leaves your machine except the OAuth calls to Anthropic's own login and usage endpoints.

## Contributing

Bug reports and ideas are welcome — see **[CONTRIBUTING.md](CONTRIBUTING.md)**
for setup and the few gotchas worth knowing before a first PR.

## Dev tooling

- **Bun** for install/scripts, **Node 24** pinned in `.nvmrc`
- **Tests**: `bun run test` (never bare `bun test` — the groups under `test/`
  must each run in their own process). `bun run test:coverage` enforces the
  floor; every PR runs both.
- **Biome** for format + lint (`bun run check`); a versioned **pre-commit hook** (`.githooks/pre-commit`) auto-formats staged files and blocks on errors. It's wired up automatically on `bun install` (via the `prepare` script).

### Releasing

Releases are **fully automated**. Every push to `main` runs
[semantic-release](https://semantic-release.gitbook.io) (`.github/workflows/release.yml`):
it reads the **Conventional Commits** and, when there's something to ship,
computes the version, builds the app for **macOS, Windows and Linux** on their
own runners, publishes `clauddy` to npm, and cuts a GitHub Release with every
artifact attached. Nothing to do by hand — just merge your PRs.

The pipeline runs in three stages, because electron-builder can't cross-build
Windows/Linux from macOS: `version` (a semantic-release dry-run that computes
the next version) → `build` (a matrix that stamps that version into
`package.json` so the filenames are right) → `publish` (downloads every
artifact and runs semantic-release for real).

- `feat:` → minor, `fix:` → patch, `feat!:`/`BREAKING CHANGE` → major.
- `docs:`/`chore:`/`ci:` etc. don't trigger a release.
