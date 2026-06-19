# Claude Usage Monitor — project guide

A cute pixel-art macOS desktop pet (Electron) that tracks Claude Code usage:
real session/weekly % (via OAuth login) plus token counts from local logs.

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
- `make-icon.js` — generates the app icon from the pixel sprite
- `pet` — dev script to simulate pet states (writes `~/.claude-usage-monitor/debug.json`)

## Commands

```bash
bun start        # dev run
bun run pack     # build dist/mac-arm64/Claude Usage Monitor.app
bun run check    # Biome format + lint (autofix)
./pet <state>    # simulate: fire | sleeping | working | idle | poke | celebrate | auto
```

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
