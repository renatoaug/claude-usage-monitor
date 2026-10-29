# Alerts

The notifications Clauddy sends on its own, the reminder you can ask for at a session reset, and the rules that keep both from repeating or claiming more than they know.

## Notifications

Turned on, with their thresholds, in **⚙ Settings → Alerts**:

| Notification | When |
| --- | --- |
| _Session at 82%_ · `2h 39m left · resets 6:50 PM` | The session crosses a threshold (80% and 95% by default) |
| _Weekly usage at 84%_ · `resets Fri 7:00 AM` | The same, for the weekly limit |
| _Fable weekly at 84%_ · `resets Fri 7:00 AM` | The same, for a per-model weekly limit |
| _Codex session at 82%_, _Codex weekly at 84%_ | The same, for Codex, when connected |
| _Cursor usage at 82%_, _Cursor API models at 84%_ · `resets Oct 11` | The same, for Cursor's month, when connected |
| _Session window reset_ · `full budget again` | A session pushed past 80% rolls over |
| _Clauddy lost access to your usage_ | The OAuth token expired or was revoked |

- Only the highest threshold makes a sound; the others arrive silently.
- Each fires once and re-arms when usage drops back below it. What is armed is saved in `alerts.json`, so relaunching at 85% does not repeat the 80% alert.
- Saving an unrelated setting does not re-arm anything; changing the thresholds does.
- Only the active Claude account can notify ([usage](04-usage.md#several-subscriptions)).
- A click on a notification brings the widget to the front.
- The first one asks macOS for permission; after that Clauddy appears in **System Settings → Notifications**. macOS drops the notifications of an app that is not signed with its real app id, which is why the build ad-hoc signs the bundle (`scripts/adhoc-sign.js`).

## Remind me at the session reset

When the selected session crosses the first threshold, **Notify at reset** appears under the session details and in the compact monitor. A click arms it, **Reminder on** cancels it. It is a one-shot request, separate from the automatic alerts.

- Reminders are saved before they fire (`reset-reminders.json`), so they survive restarts; one that came due while the Mac slept or Clauddy was closed is delivered on reopening.
- Each belongs to the Claude account, Codex or Cursor connection that set it, and is canceled when that source is disconnected or removed.
- Clauddy must be running to notify.

A deadline alone does not prove a new budget ([ADR-007](../decisions/ADR-007-a-deadline-is-not-a-new-budget.md)). Only a fresh, lower reading after the reset time says _"budget is back"_, and the pet celebrates. Without one, which is common with an idle Codex, the reminder only says the reset time arrived and asks to check the usage.

## Still open

- [ ] **Alerts and the panel disagree without an account.** The panel shows a % only from the connected account. The alerts in `main.js` still fall back to a % worked out from local tokens, a budget guess that can be off by an order of magnitude, and a comment there still says the widget shows that estimate too. Either the alerts stop using the estimate, or the docs say they do.
