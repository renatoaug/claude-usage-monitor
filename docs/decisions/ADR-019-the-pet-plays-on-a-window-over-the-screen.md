# ADR-019: Just the pet plays on a see-through window over the screen

- **Status**: Accepted
- **Date**: 2026-11-26
- **Related**: [design/02-features.md](../design/02-features.md#just-the-pet-at-play), [design/08-native.md](../design/08-native.md#the-stage), [design/06-pets.md](../design/06-pets.md#hats-and-play)

## Context

In pet-only size the pet stood still in its corner. People asked for a pet that does things on the screen on its own, each in its own way: the Getaway driving along the bottom of the screen, Santa's sleigh dropping presents, the Gingerbread running from the cursor, hopping onto windows, being thrown around. Clauddy's own window is a fixed canvas the size of the widget, so anything bigger than it (a hop onto another app's window, a throw across the screen) can't be drawn there, and moving that window frame by frame would fight the OS and every click on it.

## Decision

While something is happening, the app opens a second window: borderless, transparent, always on top, the size of the screen the pet is on, and clicks go through it. The play is drawn there and the window closes when it ends. The pet's own window shows nothing while the pet is out, and moves only once, to where a thrown pet lands. To land on other apps' windows the app reads where they are, and the cursor's position while something plays, (their frames, front to back) and never their titles or contents, so no permission is asked: `CGWindowListCopyWindowInfo` on macOS, `EnumWindows` on Windows. Play is on by default and only in pet-only size; **Settings → Appearance → Plays on the screen** turns it off.

## Alternatives considered

- **Move the widget's own window for each hop**: no second window, but a window moved every frame lags behind and flickers, and it can't show snow beside it.
- **Keep everything inside the widget's canvas**: nothing to read from the OS, but no hopping onto windows and no throw, which is the point.
- **Read window titles to aim at the terminal**: a better aim, but macOS asks for Screen Recording to read titles; the front window is usually the terminal anyway.

## Consequences

- The overlay is up only while something plays (seconds to a minute), so it costs nothing at rest; while up it repaints every frame.
- Other apps' windows are read about twice a second while something plays, and never stored.
- Linux reads no windows: the pet still snows, bursts and gets thrown, but lands only on the screen's floor.
- The throw needs the cursor from the OS while dragging, so it is macOS only; elsewhere the drag stays the OS's own and nothing is thrown.
