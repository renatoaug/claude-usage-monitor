# ADR-018: The API value comes from a price list in the app, which a file can correct

- **Status**: Accepted
- **Date**: 2026-11-26
- **Related**: [design/04-usage.md](../design/04-usage.md#api-value), [ADR-001](ADR-001-percent-from-the-account.md)

## Context

On a subscription the tokens have no bill, so nothing said what a week of Claude Code is worth. The local logs record, for every answer, its model and its tokens by kind (input, output, cache read, cache write and how long the write is kept), which is all the API bills on. Codex's rollouts record their running totals by kind too. The account reports no dollar figure, and prices change with each model.

## Decision

The dollar value is worked out in the app from the local logs: each answer is priced once, when read, at its model's list price per kind of token. It is shown only when asked for (`apiValue`, off by default): tokens and percent stay the default, and the value always reads as what the tokens would cost at API prices, never as a bill.

The price list has three layers, first match wins: `~/.claude-usage-monitor/prices.json` (read once at launch, names matched as a part of the model id, longest first), then OpenAI's prices for Codex (exact model id only), then Anthropic's for Claude, both compiled into `native/src/pricing.rs`. A model in none of them adds no dollars, and the panel says how many tokens have no price. The plan's price, used only to compare, is the one set in Settings or a plan with a single price; it is never guessed.

## Alternatives considered

- **Fetching the prices at run time**: always current, but a network call, a page to parse, and a number that changes with no release or edit behind it.
- **One average price per token**: one line of code, but cache reads cost a fortieth of output on some models, so the total would be off by a large factor.
- **The Usage and Cost API**: the real bill, but only for API organizations with an admin key; a subscription has none.
- **Pricing an unknown model as its nearest name**: fewer gaps, but a wrong price looks as sure as a right one; a stated gap is better.
- **Leaving Codex in tokens**: nothing to maintain, but the dollar mode then means something different on each tab.

## Consequences

- The value works offline and on every plan, and the same logs always give the same total.
- A stale or missing price is fixed by editing a file, with no release; the built-in list still gets its row with each new model, and past tokens are then repriced at the new list.
- The file is trusted as written: a wrong price in it gives wrong dollars, and it takes a restart to be read.
- OpenAI's list is a second one to keep current, and its exact-id match leaves a model unpriced until it is listed.
- The Batch discount, fast mode's premium and long-prompt prices are not counted, so an API user's real bill can differ. Cursor reports no tokens, so it has no value.
