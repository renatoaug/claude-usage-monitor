# Decision records

The project's decision log: short, dated, immutable records of structural decisions. They answer *"why is it like this?"* months later, and keep settled discussions from being reopened by someone who did not know they were settled.

Two kinds, same format:

- **ADR** (Architecture Decision Record) — technical choices: `ADR-NNN-slug.md`
- **PDR** (Product Decision Record) — product direction and scope: `PDR-NNN-slug.md`

The depth behind a record (diagrams, every setting, every case) lives in [`docs/design/`](../design/). Records are the memory; the design notes are the depth.

## Rules

1. **Records are immutable.** Reversing a decision means writing a *new* record and marking the old one `Superseded by ADR-NNN` in its Status line — never editing history.
2. **Write it in the same pull request.** A change that resolves a structural question ships with its record, written while the context is loaded.
3. **Keep it to half a page.** Four sections: Context, Decision, Alternatives considered, Consequences. If it needs more, link a design note or an issue.
4. **Number sequentially** within each prefix.
5. **State the decision as a rule someone can follow**, not as a summary of a discussion.

## Template

```markdown
# ADR-NNN: <Title>

- **Status**: Accepted | Superseded by ADR-NNN
- **Date**: YYYY-MM-DD
- **Related**: design notes, issues, other records

## Context
What was the situation and the forces at play? (2–5 sentences)

## Decision
What we decided, stated as a rule someone can follow.

## Alternatives considered
What else was on the table and why it lost. (bullets)

## Consequences
What this makes easier, what it makes harder, what to watch. (bullets)
```

## Index

The first records were written after the fact, from the code and the pull requests that made each decision; their date is the day the decision shipped.

| Record | Title | Status |
|---|---|---|
| [ADR-001](ADR-001-percent-from-the-account.md) | The % comes from the account, the tokens from the local logs | Accepted |
| [ADR-002](ADR-002-three-test-processes.md) | The tests run as three processes | Accepted |
| [ADR-003](ADR-003-coverage-counts-every-file.md) | The coverage gate counts every file that ships | Accepted |
| [ADR-004](ADR-004-codex-and-cursor-are-opt-in.md) | Codex and Cursor are opt-in and read-only | Accepted |
| [ADR-005](ADR-005-burn-rate-on-tokens.md) | The burn rate is fitted on tokens and anchored on the account % | Accepted |
| [ADR-006](ADR-006-working-means-an-open-turn.md) | Working means an open turn, not a recent write | Accepted |
| [ADR-007](ADR-007-a-deadline-is-not-a-new-budget.md) | A reset deadline alone does not prove a new budget | Accepted |
| [ADR-008](ADR-008-pets-drawn-in-code.md) | Pets are drawn in code and ship inside the app | Accepted |
| [ADR-009](ADR-009-holiday-pet-by-hash.md) | The holiday pet is picked by a hash of the year | Accepted |
| [ADR-010](ADR-010-build-on-each-platform.md) | Each platform builds on its own runner | Accepted |
