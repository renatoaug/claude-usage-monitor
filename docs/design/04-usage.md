# Usage

Where every number comes from: the Claude account and its local logs, several subscriptions, Codex, Cursor, how old a reading may be, and the burn-rate projection.

## Two sources, two jobs

| Number | From | Why |
|---|---|---|
| Session and weekly % | The account, through OAuth ([ADR-001](../decisions/ADR-001-percent-from-the-account.md)) | Only the account knows the real limit; a % worked out from tokens drifts from the official panel |
| Tokens, breakdowns, 30-day map, activity | The local logs, `~/.claude/projects/**/*.jsonl` | The account does not report them, and the logs have them for free |

Without a connected account the limits area shows _"Connect your account"_ instead of a %. The tokens still work.

## The Claude account

**⚙ Settings → Accounts → Claude → Connect** opens an Anthropic login in the browser (OAuth with PKCE, the same public client as Claude Code). Log in, copy the code it shows, paste it back and **Connect**. The token lands in `~/.claude-usage-monitor/auth.json` and refreshes on its own. When it expires or is revoked, a notification says so and the % goes back to the "connect" prompt.

### Several subscriptions

A personal Pro and a work Max, say. The **account chip** in the top-left corner switches between them: the list, the active one marked, and **+ Add another account**, which runs the same login into a new slot. The tray has the same list under **Account**. A login given up halfway leaves no empty slot.

- The widget follows **one account at a time**: its % is shown, its logs are counted, and only it can notify.
- Each account keeps its own token and its own armed alerts, so switching never replays a notification dismissed elsewhere. Position, mode, zoom and thresholds are shared.
- **×** on a row deletes that account's token. The active account and the last one left cannot be removed.

## Codex

[Codex](https://openai.com/codex) (the CLI and the desktop app) is opt-in ([ADR-004](../decisions/ADR-004-codex-and-cursor-are-opt-in.md)): **⚙ Settings → Accounts → Codex → Connect** looks for a Codex session on this computer and starts if it finds one. There is nothing to log into: Codex writes its tokens and rate limits to `~/.codex/sessions` (or `$CODEX_HOME`), and Clauddy only reads them.

With more than one service connected, **tabs under the pet** pick which one the panel shows; the other tab keeps its % and gets a **!** past an alert threshold. Minimized, the ring becomes one line per service.

- **The % moves only while Codex runs.** Its logs are written during a session, so an idle Codex shows its last reading with its age (`read 1h 30m ago`), and the usage bar under it goes dotted. A window that reset since then shows **—**, never a made-up 0%.
- **It can lag the account page**: the limits are shared with other ChatGPT products that do not write these logs.
- **No burn rate and no activity scenes**: the logs do not carry enough to draw them honestly.
- A plan with only a weekly window shows one weekly meter.

**×** on the Codex card stops monitoring; the Codex login and logs are untouched.

## Cursor

[Cursor](https://cursor.com) is opt-in too: **⚙ Settings → Accounts → Cursor → Connect** needs the Cursor app signed in on this computer. Clauddy reads the app's session token from its local storage and asks the endpoints the cursor.com dashboard uses, every 5 minutes.

- **Its budget is monthly.** The tab shows **this month · included** and **API models**, both resetting on the billing date; there is no 5-hour or weekly window.
- **Activity scenes work**: the agent writes a transcript of every tool call under `~/.cursor/projects`.
- **No tokens**: neither source reports them, so there are no breakdowns or 30-day map.
- **The endpoints are unofficial.** If Cursor changes them, the tab shows the last reading with its age until Clauddy is updated.

**×** on the Cursor card stops monitoring; the Cursor app and its login are untouched.

## Old readings

A Codex reading older than 15 minutes, or one whose window has expired, counts as stale: it is shown with its age and a dotted bar, and it never sets the pet's mood. An expired reading is never replaced with zero. The 15 minutes suit the 5-hour window; a weekly-only Codex plan moves slowly, so its reading is often right while marked stale.

## Burn rate

At 82% with 1h 12m left, the useful question is whether the session lasts. Clauddy fits the slope of recent usage and adds one line under the session bar:

- **`~35m left at this pace`** (coral): you run out before the window resets.
- **`resets before you run out`**: the reset gets there first.

The slope is fitted over **session tokens**, then converted to % with the account's number ([ADR-005](../decisions/ADR-005-burn-rate-on-tokens.md)). The % arrives as a whole number every few minutes, so over a short window it is a single `16 → 17` step; tokens step once per assistant turn, 10 to 20 times finer. The fit reads the **recent** pace, not the session average, so going quiet eases it off.

The line appears only once there is enough to say honestly, about 5 minutes into a session, and hides while idle, when the pace is flat or right after a reset. Only Claude gets it.
