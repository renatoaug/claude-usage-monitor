# The native app

How the app in `native/` is built: Rust with eframe/egui, no webview ([ADR-012](../decisions/ADR-012-the-native-app-replaces-electron.md)). It reads and writes the same files the Electron app did, so an upgrade keeps every setting and account. The notes on [usage](04-usage.md) and [alerts](05-alerts.md) say where the numbers come from and when it notifies; this note covers how the app is built.

## Overview

One window, drawn immediately by egui, and one background thread that reads everything. The window repaints only when something changes: a new reading, the pet's next frame, an animation, the pointer.

```mermaid
flowchart LR
    logs["~/.claude, ~/.codex, ~/.cursor"] --> poller
    api["Claude, Cursor APIs<br/>codex app-server"] --> poller
    poller["poller thread<br/>(poller.rs)"] -- "Snapshot" --> app["App<br/>(app.rs)"]
    app -- "Frame" --> screens["screens/"]
    screens -- "Action" --> app
    app -- "Command" --> poller
    poller --> notes["notifications"]
```

- **The poller** (`poller.rs`) reads the Claude logs (`usage.rs`), the account's % (`auth.rs`), Codex (`codex.rs`) and Cursor (`cursor.rs`), checks the alerts (`alerts.rs`) and the reset reminders (`reminders.rs`), and publishes a `Snapshot`. Commands from the window (switch account, log in, save settings, arm a reminder) reach it over a channel and are answered at once, followed by a fresh poll.
- **The app** (`app.rs`) holds what is on screen: the size, the open tab, the settings draft, the speech bubble. Each frame it builds a `Frame` from the latest snapshot and hands it to the screen.
- **The screens** (`screens/`) draw only from the `Frame` and return `Action`s for the app to apply. They never read the poller, the disk or the clock on their own, which is what lets the snapshot gallery render every state from fixtures.

## Code layout

| Where | What |
|---|---|
| `src/ui/` | Generic pieces that know nothing about Clauddy: the theme tokens, icons, controls, surfaces, bars, tags, text |
| `src/components/` | Clauddy's own pieces: the pet, logos, account chip, service rows, meters, status line, breakdowns, heat map, the glance and speech bubbles, the reminder button |
| `src/screens/` | Compact, expanded, pet-only and Settings, and the snapshot gallery |
| `src/platform/` | What the OS has to answer: the cursor watcher, the tray, login items, activation |
| `src/voice.rs`, `src/talk.rs` | The pet's remarks and blips, and when it says them |
| `src/updates.rs` | The release check and the installer |

The theme (`ui/theme.rs`) keeps the raw palette private; everything else uses semantic names, and a test fails on a raw colour anywhere else.

## The window on macOS

Floating, the window is a fixed transparent canvas and the card lives inside it: the window never resizes, because a resize shows one empty frame that reads as a blink. Changing size morphs the card within the canvas instead. Clicks off the card fall through to what is behind: a light thread (`platform/cursor.rs`) reads the cursor's screen position every 40 ms and turns click-through on only while the cursor is outside the card (or, in pet-only, outside the pet and its glance).

In the menu bar the window is an ordinary one the card's size, shown under the icon as a popover that closes when it loses focus. It is shown from a timer once the bar's menu has closed: shown from the menu's own handler, macOS hands the next click to the app that was in front.

A hidden window gets no frames, so anything that must happen while it is hidden (the menu's picks) puts it back on screen first.

## Pets

A pet is a pair of sprite sheets listed in `src/components/skins.rs`; the catalog, the holiday rule and how to add one are in [pets](06-pets.md).

## Notifications, login and updates

- **Notifications** go through `UNUserNotificationCenter`, from a signed bundle only; at launch the app comes to the front and asks for permission. The loose binary can't notify.
- **Open at login**: the released app registers itself (macOS `SMAppService`, the Windows Run key, a Linux autostart entry); development builds never do.
- **Updates**: see [releases](07-releases.md).

## Files

The same as the Electron app's, in `~/.claude-usage-monitor/`: `config.json` (only the keys the native app edits are rewritten), `accounts.json`, `auth.json` and `alerts.json` per account, `reset-reminders.json`. Its own: `native-ui.json` (size, open sections, the service shown), `native-voice.json` (once-a-day remarks), and `limits-cache.json` per account (the last reading and the polling schedule).

What only the Electron app used goes at launch (`platform/leftovers.rs`): its Chromium profile under the OS's app-data folder (`clauddy`, and the earlier `claude-usage-monitor` and `claude-usage-pet`; a folder by those names that isn't a Chromium profile stays), and `window.json`, `debug.json`, `icon.png` and `electron/` in the data dir. The released app does it; a development build leaves them, so the upgrade from Electron can still be tried.

## Tests

`cargo test --release` in `native/` runs the unit tests (each reader is a port of its JS module and of its tests) and the snapshot gallery: `src/screens/snapshots.rs` renders every screen state into `native/tests/snapshots/*.png` and fails when a render drifts. After an intended change, `UPDATE_SNAPSHOTS=1 cargo test --release` rewrites them; review the PNGs before committing.
