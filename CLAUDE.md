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
- `renderer/` — `index.html`, `pet.js`, `style.css` (the pet + UI)
- `make-icon.js` — generates the macOS `.icns` from the pixel sprite
- `make-ico.js` — packs the Windows `.ico` (`build-icon.sh` drives both + the Linux `.png`)
- `make-tray.js` — generates the menu-bar (tray) template icon from the same sprite
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
`celebrate`, `auto`, plus the activity scenes `reading`, `editing`, `running`,
`planning`, `researching`, `delegating`, `waiting`.

> Windows/Linux artifacts must be built on their own OS (or CI runner) — electron-builder can't
> reliably cross-build them from macOS. `release.yml` handles that in three stages:
> `version` (semantic-release dry-run) → `build` (matrix on macOS/Windows/Linux, each stamping the
> computed version via `scripts/set-version.js`) → `publish` (downloads every artifact and runs
> semantic-release for real).

## Tests

`bun run test` — never bare `bun test`. The suite runs as **three separate
processes**, because mocking is global to a bun runtime and bun loads every
test file before running any of them:

- `test/` — the real `usage.js`, `auth.js` and `renderer/burn.js`, no mocks.
- `test-main/` — `main.js`, with `electron`, `./usage` and `./auth` replaced.
  Those mocks would otherwise reach the suites above.
- `test-dom/` — `renderer/pet.js` and `preload.js` under happy-dom, which
  installs a DOM on every global.

Line coverage sits around **87%**. `pet.js` and `burn.js` carry a
`module.exports` guard at the bottom so the same file works as a plain
`<script>` in the widget and as an import in the tests.

> `main.js`'s update path spawns `curl … | bash`. Any test touching it **must**
> stub `child_process.spawn`, or it downloads and installs over the running app.

## Data & secrets

- All user data lives in `~/.claude-usage-monitor/` (NOT in the repo): `auth.json`
  (OAuth token, mode 600), `config.json` (settings), `debug.json` (simulator).
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
