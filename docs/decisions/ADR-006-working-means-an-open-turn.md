# ADR-006: Working means an open turn, not a recent write

- **Status**: Accepted
- **Date**: 2026-09-25
- **Related**: [design/02-features.md](../design/02-features.md#moods), `usage.js`

## Context

Claude Code and Cursor write nothing to their logs while a command runs or the model thinks, sometimes for minutes. A pet that called "working" anything written in the last 20 seconds fell asleep in the middle of long tasks.

## Decision

A service is working while its turn is open. A Claude turn closes on a final answer (`end_turn`, then `system/turn_duration`) or `[Request interrupted by user`; a Cursor turn on `turn_ended`. Slash commands, `!` shell input, task notices and meta lines open nothing. A turn untouched for 30 minutes counts as abandoned. Only a log whose tail says nothing either way falls back to the recent-write window (`activeThresholdMs`).

## Alternatives considered

- **A longer recent-write window**: keeps the pet awake through long commands, but also long after the work ended.

## Consequences

- The pet keeps working through long commands and thinking.
- A crashed session can show working for up to 30 minutes.
