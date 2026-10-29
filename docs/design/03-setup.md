# Setup

Installing Clauddy on each platform, running it from the code, every setting in `config.json`, and the simulator that forces the pet's states.

## Install

**macOS (Apple Silicon)** is the first-class build. **Windows (x64)** and **Linux (x64)** work too. `bunx clauddy` runs on all three.

### macOS

**As an app, opening at login (recommended).** One command downloads the latest release and puts it in `/Applications`:

```bash
curl -fsSL https://raw.githubusercontent.com/renatoaug/claude-usage-monitor/main/install.sh | bash
```

[`install.sh`](../../install.sh) only downloads the release zip from this repo, replaces the app and opens it. It exists because macOS blocks unsigned apps downloaded through a browser with a _"damaged, move to Trash"_ warning; only a paid Apple signature ($99 a year) silences it, and files fetched with `curl` are not flagged. The app then registers in **Login Items** and starts with the Mac.

**Without installing.** With [Bun](https://bun.sh), or `npx` with Node 24:

```bash
bunx clauddy
```

The first run downloads Electron. It stays up only while that command is open and does not start at login. **×** quits it.

### Windows (x64)

```powershell
bunx clauddy   # or: npx clauddy
```

Or the standalone app: the portable zip (`Clauddy-<version>-win-x64.zip`) from the [latest release](https://github.com/renatoaug/claude-usage-monitor/releases); unzip it anywhere and run `Clauddy.exe`. The app is unsigned, so **SmartScreen** shows _"Windows protected your PC"_ the first time: **More info → Run anyway**. From then on it starts with Windows.

### Linux (x64)

```bash
bunx clauddy   # or: npx clauddy
```

Or the AppImage or tar.gz (`Clauddy-<version>-linux-x86_64.AppImage`, `Clauddy-<version>-linux-x64.tar.gz`) from the [latest release](https://github.com/renatoaug/claude-usage-monitor/releases):

```bash
chmod +x Clauddy-*.AppImage
./Clauddy-*.AppImage
```

GNOME and most Wayland desktops take an app's icon only from an installed `.desktop` entry, so a bare AppImage shows a generic icon. Once, next to the AppImage:

```bash
./Clauddy-*.AppImage --appimage-extract clauddy.desktop >/dev/null
./Clauddy-*.AppImage --appimage-extract usr/share/icons/hicolor/512x512/apps/clauddy.png >/dev/null
mkdir -p ~/.local/share/applications ~/.local/share/icons/hicolor/512x512/apps
cp squashfs-root/usr/share/icons/hicolor/512x512/apps/clauddy.png ~/.local/share/icons/hicolor/512x512/apps/clauddy.png
sed "s|^Exec=.*|Exec=$(readlink -f Clauddy-*.AppImage) --no-sandbox %U|" squashfs-root/clauddy.desktop > ~/.local/share/applications/clauddy.desktop
rm -rf squashfs-root
update-desktop-database ~/.local/share/applications 2>/dev/null
```

The tray icon needs an indicator extension on vanilla GNOME (such as "AppIndicator and KStatusNotifier Item Support"); Cinnamon, KDE and XFCE show it as is. Starting at login goes through an XDG entry in `~/.config/autostart/`.

## Update

**⚙ Settings → Check for updates → Update now** runs the same installer and relaunches the new build. A check also runs 8 seconds after launch and every 6 hours.

## From the code

```bash
bun install     # dependencies, and turns on the pre-commit hook
bun start       # the widget, from source
```

Node 24 (`.nvmrc`). The app reads the existing `~/.claude` logs; connecting an account is optional, and without it there are token counts but no percentages.

## Configure

Settings saved from the UI live in `~/.claude-usage-monitor/config.json`, so they can be edited without rebuilding:

```jsonc
{
  "mode": "floating", // "floating" pet in a corner, or "menubar" popover
  "alerts": true, // notifications on or off
  "alertThresholds": [80, 95], // notify when the session or week crosses these %
  "fireThreshold": 90, // session % at which the pet catches fire (maxed out stays at 100)
  "zoom": 100, // widget scale %, 100 to 200
  "pet": "clauddy", // the chosen pet (renderer/pets.js); holidays override it on their days
  "pollIntervalMs": 4000, // how often the local logs are read
  "activeThresholdMs": 20000, // fallback "active" window, for a log whose tail does not say whether a turn is open
  "sleepThresholdMs": 300000, // "sleeping" after this much idle time (5 min)
  "codex": { "enabled": false }, // set by Settings → Accounts → Codex
  "cursor": { "enabled": false }, // set by Settings → Accounts → Cursor
  "talk": true, // speech bubbles
  "sound": false, // chiptune blips, opt-in
  "soundMutedUntil": 0 // set by the 🔊 button: muted until this epoch ms
}
```

`CLAUDE_CONFIG_DIR` isolates a whole instance (token, settings and logs), so two Clauddys can run side by side, one per account.

## Play with the pet

With the widget running, force a state from the terminal:

```bash
bunx clauddy poke        # squish and hearts
bunx clauddy celebrate   # jump and confetti
bunx clauddy fire        # on fire
bunx clauddy sleeping    # blue zzz
bunx clauddy working     # eats token coins
bunx clauddy tired       # maxed out
bunx clauddy idle        # calm
bunx clauddy auto        # back to the real usage
bunx clauddy say         # a remark in the speech bubble
bunx clauddy say fire    # a given one: greeting, fire, reset, maxed, welcome, streak, record, codex, cursor
```

Each command writes `~/.claude-usage-monitor/debug.json`, which the running widget watches. Installed globally, drop the `bunx`. In the repo, `./pet <state>` does the same, and also takes the activity scenes: `reading`, `editing`, `running`, `planning`, `researching`, `delegating`, `waiting`.
