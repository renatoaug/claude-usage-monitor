# ADR-004: Codex and Cursor are opt-in and read-only

- **Status**: Accepted
- **Date**: 2026-09-22
- **Related**: [design/04-usage.md](../design/04-usage.md#codex), [design/04-usage.md](../design/04-usage.md#cursor)

## Context

Clauddy can read Codex's session logs and the Cursor app's login on the same machine. Finding them is easy, but someone who installed Clauddy for Claude did not ask it to read another product's files, and Cursor's endpoints are unofficial.

## Decision

Codex and Cursor are monitored only after **Connect** in Settings (`config.codex.enabled`, `config.cursor.enabled`), never because their files exist. Clauddy only reads their data and never writes to it. Disconnecting stops the monitoring and leaves their login and logs untouched.

## Alternatives considered

- **On by default when the files are found**: less setup, but reads data nobody agreed to share.
- **A separate login for each service**: Codex has nothing to log into, and Cursor's login already lives in its app.

## Consequences

- A clean install shows only Claude.
- An unofficial Cursor endpoint that changes shows the last reading with its age until Clauddy is updated.
