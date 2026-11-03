# ADR-011: Codex limits come from its app-server

- **Status**: Accepted
- **Date**: 2026-11-03
- **Related**: [design/04-usage.md](../design/04-usage.md#codex), [ADR-004](ADR-004-codex-and-cursor-are-opt-in.md), [ADR-007](ADR-007-a-deadline-is-not-a-new-budget.md)

## Context

Clauddy read the Codex % only from `token_count` events in the session logs. Other ChatGPT products, such as Dot, spend the same plan without writing those logs, so the % froze while the budget moved: a reading of 85% stayed on screen when the account was at 40%, in a newer window.

## Decision

While Codex is connected, Clauddy runs `codex app-server` every 2 minutes (5 after a failure), makes one `account/rateLimits/read` over stdio and exits it. The newer of that reading and the logs wins. Clauddy never reads Codex's token; the CLI uses its own login. A failed read keeps the last good reading, which ages into stale and never into zero. Tokens and history still come only from the logs.

## Alternatives considered

- **Call `/wham/usage` with the token in `~/.codex/auth.json`**: no process to start, but Clauddy would hold another product's credential, refresh it, and miss logins kept in the keychain.
- **A long-lived app-server**: no spawn per read, but one more process left running beside the widget.
- **Keep showing the logs and their age**: honest about the age, but wrong about the budget for anyone using those products.

## Consequences

- The % follows the whole plan, whichever product spent it.
- It depends on a CLI being found: the desktop app's bundled one, or `codex` on the PATH, which an app opened from the GUI often doesn't see. Without one, Clauddy behaves as before.
- Spending outside the logs adds to the % and not to the token counts.
