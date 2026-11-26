# ADR-017: On Linux, a Wayland session opens the window on XWayland

- **Status**: Accepted
- **Date**: 2026-11-17
- **Related**: [design/08-native.md](../design/08-native.md#the-window-on-linux), [design/03-setup.md](../design/03-setup.md#linux-x64)

## Context

The floating widget asks to stay above other windows. On Wayland a client can't: the protocol leaves stacking to the compositor, and winit's `set_window_level` does nothing there, as does placing the window. On KDE Plasma under Wayland (Bazzite's default) the widget sank under every other window, and users asked for an "always on top" option it already had everywhere else.

## Decision

On Linux, when the session is Wayland (`WAYLAND_DISPLAY` set) and XWayland is running (`DISPLAY` set), the app opens its window on X11 through XWayland, where the compositor honours always-on-top and positioning. `CLAUDDY_WAYLAND=1` keeps it on Wayland. Without XWayland it stays on Wayland rather than fail to start. Other systems are untouched.

## Alternatives considered

- **Stay on Wayland and document a window rule**: no code, but every user has to find and set a KWin or GNOME rule by hand, and GNOME has no such rule without an extension.
- **The `wlr-layer-shell` protocol**: an overlay layer that stays on top, but GNOME doesn't implement it and winit doesn't expose it.
- **A setting to pick the backend**: one more option to understand, for a choice almost nobody would make differently.

## Consequences

- The widget stays on top and keeps its place on KDE Plasma and GNOME under Wayland.
- Under fractional scaling, XWayland can render text a little softer than native Wayland.
- A session without XWayland keeps the old behaviour: no always-on-top.
