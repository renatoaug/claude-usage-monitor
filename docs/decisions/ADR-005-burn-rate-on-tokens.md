# ADR-005: The burn rate is fitted on tokens and anchored on the account %

- **Status**: Accepted
- **Date**: 2026-08-15
- **Related**: [design/04-usage.md](../design/04-usage.md#burn-rate), `renderer/burn.js`

## Context

The useful number under the session bar is when the session runs out at the current pace. The account % is the number that matters, but it arrives as a whole number every few minutes: over a short window the whole signal is one `16 → 17` step, which throws a fitted slope off by multiples. Session tokens also step, once per assistant turn, but 10 to 20 times finer.

## Decision

The slope is fitted over recent session tokens and converted to % with the account's reading, recalibrated on every poll. The projection appears only after about 5 minutes of a session, and hides while idle, when the pace is flat and right after a reset. It covers Claude only.

## Alternatives considered

- **Fit the account % directly**: too coarse over the window that matters.
- **The session average instead of the recent pace**: never eases off after a quiet stretch.

## Consequences

- The projection is steady and reacts when the pace changes.
- Codex and Cursor get no projection: their data cannot support one honestly.
