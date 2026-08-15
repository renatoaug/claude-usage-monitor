<!--
The PR title becomes the squashed commit message and drives the release, so it
must be a Conventional Commit: feat: / fix: / docs: / chore: / ci: / test: /
refactor: / perf: / build: / revert:
-->

## What and why

<!-- What changes, and what problem it solves. Link the issue if there is one. -->

Closes #

## How it was verified

<!-- What you actually ran or observed — not what should theoretically work. -->

## Checklist

- [ ] `bun run check` is clean
- [ ] `bun run test:coverage` passes (80% overall, 60% per file, nothing shipped untested)
- [ ] Tests cover the new behaviour
- [ ] `bun run pack` builds and the app still launches
- [ ] The title is a Conventional Commit with the right type
- [ ] Docs updated if behaviour changed (`README.md` / `CLAUDE.md`)
