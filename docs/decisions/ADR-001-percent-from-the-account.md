# ADR-001: The % comes from the account, the tokens from the local logs

- **Status**: Accepted
- **Date**: 2026-06-18
- **Related**: [design/04-usage.md](../design/04-usage.md)

## Context

The limit Claude Code enforces is set by the account, and the official **Settings → Usage** panel is what people trust. A % worked out from token counts needs a guessed budget, which differs per plan and per model and can be off by an order of magnitude. The local logs, on the other hand, have every token, model and project for free.

## Decision

The session and weekly % shown in the widget come only from the account, through OAuth (PKCE, the same public client as Claude Code). Tokens, breakdowns, the 30-day map and activity come from `~/.claude/projects/**/*.jsonl`. Without a connected account the widget asks to connect instead of showing an estimated %.

## Alternatives considered

- **A % from tokens and a per-plan budget**: no login needed, but the number disagrees with the official panel and nobody can tell by how much.
- **Scraping the usage page**: fragile, and it would need the browser's session.

## Consequences

- The % matches the official panel exactly.
- Using the % needs a login; the token is kept in `auth.json` (mode `600`) and never committed.
- When the token dies the % disappears, so a notification says so.
- The alerts still fall back to the token estimate without an account, unlike the panel.
