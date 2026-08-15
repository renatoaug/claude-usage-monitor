# Contributing to Clauddy

Thanks for helping out. This is short on purpose — it only covers the things
that will actually trip you up.

## Setup

```bash
bun install     # also wires up the pre-commit hook
bun start       # run the widget from source
```

Node 24 (pinned in `.nvmrc`). No other setup — the app reads your existing
`~/.claude` logs, and login is optional (without it you get token counts but no
percentages).

## Three things that will bite you

**1. `bun run test`, never bare `bun test`.** The suite runs as three separate
processes, because bun mocks are per-runtime and it loads every test file before
running any — so `main`'s fake `electron` would reach the suites testing the
real modules. Bare `bun test` runs all of them together and reports dozens of
failures that have nothing to do with your change.

```
test/unit/   usage.js, auth.js, burn.js   (no mocks)
test/main/   main.js                      (mocks electron, ./usage, ./auth)
test/dom/    pet.js, preload.js           (happy-dom)
```

**2. Coverage is a gate, not a report.** `bun run test:coverage` fails CI under
80% overall, under 60% for any single file, or if a file that ships in the app
has no coverage at all. That last rule is deliberate: coverage tools only
measure files the tests load, so a new module with no tests would otherwise
pass green. Add a file to the app, add a test for it.

**3. Your PR title becomes the commit message.** PRs are squash-merged and
release automation reads it, so it must follow
[Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add confetti on session reset      → minor release
fix(auth): use the platform token host   → patch release
docs: / chore: / ci: / test: / refactor: → no release
```

Get the type wrong and you either ship a version you didn't mean to, or ship
nothing at all.

## Before you open a PR

```bash
bun run check          # Biome format + lint; the pre-commit hook runs this too
bun run test:coverage  # the same gate CI runs
bun run pack           # confirm the app still builds and launches
```

## Landmines

- **`main.js`'s update path spawns `curl … | bash`.** Any test that reaches
  `do-update` must stub `child_process.spawn`, or it downloads the installer
  and replaces the app you're running.
- **Never commit `auth.json`.** It holds an OAuth token. It lives in
  `~/.claude-usage-monitor/` and is gitignored — keep it that way, and don't
  paste tokens into issues.
- **Windows and Linux artifacts can't be cross-built from macOS.** The release
  workflow builds each on its own runner; don't try to reproduce that locally.

## Playing with the pet

`./pet <state>` forces a state without waiting for real usage — handy for
anything visual:

```bash
./pet fire        # or: sleeping, working, tired, idle, poke, celebrate
./pet reading     # activity scenes: editing, running, planning, researching…
./pet auto        # back to your real usage
```

## Architecture

`CLAUDE.md` has the layout, the data flow, and the reasoning behind the less
obvious decisions. Worth a skim before a first change.
