# Clauddy — project guide

A cute pixel-art desktop pet (Electron) that tracks Claude Code usage:
real session/weekly % (via OAuth login) plus token counts from local logs.
macOS-first, with Windows (x64) support and Linux on the way.

## Stack

- **Electron** (frameless, transparent, always-on-top widget)
- **Bun** for install/scripts, **Node 24** (pinned in `.nvmrc`)
- **Biome** for format + lint
- Vanilla JS in `renderer/` (SVG sprite + CSS/WAAPI animations) — no framework

## Layout

- `main.js` — Electron main process (window, usage polling, notifications, file watchers)
- `preload.js` — `contextBridge` API exposed to the renderer
- `usage.js` — reads `~/.claude/projects/**/*.jsonl` (tokens, session window, activity)
- `auth.js` — OAuth (PKCE) login + authoritative usage % fetch
- `accounts.js` — the account list (several subscriptions, one active at a time)
- `codex.js` — reads Codex's `~/.codex/sessions/**/rollout-*.jsonl` (limits %, tokens, 30-day history).
  Opt-in from Settings (`config.codex.enabled`); with Claude too, tabs under the pet pick
  which service the whole panel shows
- `renderer/` — `index.html`, `pet.js`, `style.css` (the pet + UI)
- `reminders.js` — persistent, one-shot session reset reminders, keyed by provider and Claude account.
- `renderer/companion.js` — provider activity transitions, with a shared cooldown.
- `renderer/voice.js` — the pet's voice: remark lines for the speech bubble and the
  chiptune blips (Web Audio square waves, no assets). Talk is on by default, sound is
  opt-in (`config.sound`) and silent in menu-bar mode, collapsed, or muted
- `make-icon.js` — generates the macOS `.icns` from the pixel sprite
- `make-ico.js` — packs the Windows `.ico` (`build-icon.sh` drives both + the Linux `.png`)
- `make-tray.js` — generates the menu-bar (tray) template icon from the same sprite
- `scripts/adhoc-sign.js` — `afterPack` hook: ad-hoc signs the macOS bundle with the
  real appId, without which macOS silently drops the app's notifications
- `pet` — dev script to simulate pet states (writes `~/.claude-usage-monitor/debug.json`)

## Commands

```bash
bun start          # dev run
bun run pack       # build dist/mac-arm64/Clauddy.app
bun run dist       # macOS zip (used by the release pipeline)
bun run dist:win   # Windows zip — run on Windows/CI (needs a native runner)
bun run dist:linux # Linux tar.gz + AppImage (run on Linux/CI)
bun run icon       # regenerate build/icon.{icns,ico,png} from the sprite
bun run check      # Biome format + lint (autofix)
bun test           # the suite — but prefer `bun run test` (see below)
bun run test       # all three groups, each in its own process
bun run test:coverage
./pet <state>      # simulate a state, e.g. ./pet fire
```

`./pet <state>` states: `fire`, `sleeping`, `working`, `tired`, `idle`, `poke`,
`celebrate`, `say`, `auto`, plus the activity scenes `reading`, `editing`, `running`,
`planning`, `researching`, `delegating`, `waiting`.

> Windows/Linux artifacts must be built on their own OS (or CI runner) — electron-builder can't
> reliably cross-build them from macOS. `release.yml` handles that in three stages:
> `version` (semantic-release dry-run) → `build` (matrix on macOS/Windows/Linux, each stamping the
> computed version via `scripts/set-version.js`) → `publish` (downloads every artifact and runs
> semantic-release for real).

## Tests

`bun run test` — three processes, not plain `bun test`:

- `test/unit/` — `usage.js`, `auth.js`, `renderer/burn.js`, `renderer/voice.js` (no mocks)
- `test/main/` — `main.js` (mocks `electron`, `./usage`, `./auth`)
- `test/dom/` — `renderer/pet.js`, `preload.js` (happy-dom)

Split because bun mocks are per-runtime and it loads every test file before
running any, so a mock in one file reaches the others. Each group must be run
by its own path — `bun test` (or `bun test test/`) globs all three into one
process and fails.

`bun run test:coverage` gates: 80% total, 60% per file, and no shipped `.js`
without coverage. Runs on every PR.

`pet.js`, `burn.js` and `voice.js` end with a `module.exports` guard so one file works as a
`<script>` in the widget and as an import in the tests.

> `main.js`'s update path spawns `curl … | bash` — always stub
> `child_process.spawn`, or it installs over the running app.

## Data & secrets

- All user data lives in `~/.claude-usage-monitor/` (NOT in the repo): `auth.json`
  (OAuth token, mode 600), `config.json` (settings), `debug.json` (simulator),
  `alerts.json` (which notifications are already armed, so restarts don't repeat them),
  `reset-reminders.json` (one-shot deadlines and recently delivered reminders),
  `accounts.json` (the account list + the active one).
- Extra accounts nest their own `auth.json`/`alerts.json` under `accounts/<id>/`;
  the first account keeps the top-level paths, so existing installs are untouched.
  Switching rebinds `auth.setDataDir`, `usage.setClaudeDir` and the alert state —
  only the active account is polled, so only it can notify.
- **Never commit** `auth.json` or any token/credential. It's gitignored — keep it that way.

## Code style

- Enforced by Biome: **single quotes**, **no semicolons**, **2-space** indent, LF.
- Run `bun run check` before committing. A pre-commit hook (`.githooks/pre-commit`)
  auto-formats staged files and blocks on lint errors; it's wired up on `bun install`.
- Match the surrounding style; keep comments short and in English.

## Commits — Conventional Commits

Every commit message MUST follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(optional scope): <short, imperative description>
```

Allowed types: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`,
`build`, `ci`, `chore`, `revert`.

Rules:

- Subject in the imperative mood, lower-case, no trailing period, ≤ ~72 chars.
- One logical change per commit; use the body to explain the "why" when useful.

Examples:

- `feat: add confetti burst on session reset`
- `fix(auth): use platform.claude.com token endpoint`
- `chore: bump Electron to 42.4.1`
- `docs: document the ./pet simulator`

## Before finishing a change

1. `bun run check` is clean.
2. `bun run pack` builds successfully (the app still launches).
3. Commit follows the convention above.

## Companion behavior

- Widget sizes: expanded, compact, and pet-only. `clauddy.size` in localStorage
  remembers the choice. Pet-only is floating-mode only; its hover/focus glance
  fades below the pet within fixed window bounds, without resizing or moving it.
- In pet-only mode, the sprite is a native Electron drag region. The separate
  expand button opens compact and appears with the glance on hover/focus. Enter on the
  focused pet also opens compact. Pointer clicks on the sprite do not change size.
  Main sends window-relative cursor coordinates only in this mode: native drag
  regions suppress DOM hover events. Keep logo descriptions accessible, without
  visible activity captions or native title tooltips.
- Session reset reminders are explicitly requested, separate from threshold
  notifications. They run while Clauddy is open and catch up after sleep/restart.
  They persist before firing, cancel on disconnect/removal, and are account-scoped.
- A deadline alone does not prove a new budget. Only a fresh, lower reading
  after that deadline earns “budget is back”; otherwise say the reset time arrived.
  Codex logs can be stale. Never substitute an expired reading with zero.
- Compact and pet-only moods follow activity across connected providers, independently
  of the selected usage tab. Logos identify the workers; simultaneous work uses a
  stable generic working animation. No active sources means rest, with quota warnings
  still shown in the meters. Stale activity and disconnected sources are ignored.
- Provider cues preserve manual selection and do not claim task completion from
  inactivity. First reads establish a baseline; cues share a 30-second cooldown.
- `AGENTS.md` is the single source of project instructions.
