# ADR-007: A reset deadline alone does not prove a new budget

- **Status**: Accepted
- **Date**: 2026-09-24
- **Related**: [design/05-alerts.md](../design/05-alerts.md#remind-me-at-the-session-reset), `reminders.js`

## Context

A reset reminder fires at the time the session window is due to reset. Codex logs and Cursor fetches can be stale, so at that moment Clauddy may still hold the old reading. Saying "your budget is back" from the clock alone can be wrong, and a stale reading must not be replaced with zero.

## Decision

Only a fresh, lower reading after the deadline earns _"budget is back"_ and a celebration. Without one, the reminder says the reset time arrived and asks to check the usage. Reminders are saved before they fire, catch up after sleep or a restart, belong to the account or connection that set them and are canceled when it goes away.

## Consequences

- The reminder never claims more than the data shows.
- With an idle Codex the message is often the cautious one.
