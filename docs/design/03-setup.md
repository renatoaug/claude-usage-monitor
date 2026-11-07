# Setup

Installing Clauddy on each platform, running it from the code, every setting in `config.json`, and how to force the pet's states.

## Install

**macOS (Apple Silicon)** is the first-class build. **Windows (x64)** and **Linux (x64)** work too. `bunx clauddy` runs on all three.

### macOS

**As an app, opening at login (recommended).** One command downloads the latest release and puts it in `/Applications`:

```bash
curl -fsSL https://raw.githubusercontent.com/renatoaug/claude-usage-monitor/main/install.sh | bash
```

[`install.sh`](../../install.sh) only downloads the release zip from this repo, replaces the app and opens it. It exists because macOS blocks unsigned apps downloaded through a browser with a _"damaged, move to Trash"_ warning; only a paid Apple signature ($99 a year) silences it, and files fetched with `curl` are not flagged. The app then registers in **Login Items** and starts with the Mac.

**Without installing.** With [Bun](https://bun.sh), or `npx` with Node 18 or newer:

```bash
bunx clauddy
```

The first run of each version downloads that version's app from the release (about 10 MB) into `~/.claude-usage-monitor/bin/`; later runs open it at once ([ADR-014](../decisions/ADR-014-npm-launches-the-native-app.md)). The command returns as soon as the app is up. **×** quits it.

### Windows (x64)

```powershell
bunx clauddy   # or: npx clauddy
```

Or the standalone app: the portable zip (`Clauddy-<version>-win-x64.zip`) from the [latest release](https://github.com/renatoaug/claude-usage-monitor/releases); unzip it anywhere and run `Clauddy.exe`. The app is unsigned, so **SmartScreen** shows _"Windows protected your PC"_ the first time: **More info → Run anyway**. From then on it starts with Windows.

### Linux (x64)

```bash
bunx clauddy   # or: npx clauddy
```

Or the tarball (`Clauddy-<version>-linux-x64.tar.gz`) from the [latest release](https://github.com/renatoaug/claude-usage-monitor/releases):

```bash
tar -xzf Clauddy-*-linux-x64.tar.gz
./Clauddy/clauddy
```

The tray icon needs an indicator extension on vanilla GNOME (such as "AppIndicator and KStatusNotifier Item Support"); Cinnamon, KDE and XFCE show it as is. Starting at login goes through an XDG entry in `~/.config/autostart/`.

## Update

**⚙ Settings → Check for updates → Update now** runs the same installer and relaunches the new build. A check also runs 8 seconds after launch and every 6 hours.

## From the code

```bash
cd native
cargo run --release              # the app, from source
scripts/bundle.sh && open target/Clauddy.app   # as a macOS app, which can notify
```

Rust stable ([rustup](https://rustup.rs)); on Linux, `libgtk-3-dev`, `libxdo-dev` and `libayatana-appindicator3-dev` for the tray. `bun install` at the root is only for the launcher and turns on the pre-commit hook. The app reads the existing `~/.claude` logs; connecting an account is optional, and without it there are token counts but no percentages.

## Configure

Settings saved from the UI live in `~/.claude-usage-monitor/config.json`, so they can be edited without rebuilding:

```jsonc
{
  "mode": "floating", // "floating" pet in a corner, or "menubar" popover
  "alerts": true, // notifications on or off
  "alertThresholds": [80, 95], // notify when the session or week crosses these %
  "fireThreshold": 90, // session % at which the pet catches fire (maxed out stays at 100)
  "zoom": 100, // widget scale %, 100 to 200
  "pet": "clauddy", // the chosen pet (native/src/components/skins.rs); holidays override it on their days
  "pollIntervalMs": 4000, // how often the local logs are read
  "activeThresholdMs": 20000, // fallback "active" window, for a log whose tail does not say whether a turn is open
  "sleepThresholdMs": 300000, // "sleeping" after this much idle time (5 min)
  "codex": { "enabled": false }, // set by Settings → Accounts → Codex
  "cursor": { "enabled": false }, // set by Settings → Accounts → Cursor
  "talk": true, // speech bubbles
  "sound": false, // chiptune blips, opt-in
  "soundMutedUntil": 0 // muted until this epoch ms
}
```

`CLAUDE_CONFIG_DIR` isolates a whole instance (token, settings and logs), so two Clauddys can run side by side, one per account.

## Force a state

`CLAUDDY_STATE` pins the pet's mood, to see one without waiting for it:

```bash
cd native
CLAUDDY_STATE=fire cargo run --release   # also: idle, working, tired, sleeping
```

`--say` plays the chiptune voice once, to check the sound.
