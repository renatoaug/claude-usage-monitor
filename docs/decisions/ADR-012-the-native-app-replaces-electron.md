# ADR-012: The native app replaces the Electron one

- **Status**: Accepted
- **Date**: 2026-11-06
- **Related**: [design/07-releases.md](../design/07-releases.md), [design/08-native.md](../design/08-native.md), [ADR-010](ADR-010-build-on-each-platform.md)

## Context

The Electron widget kept a browser engine running to draw a pet and a few meters: it used noticeable CPU at rest and the macOS app weighed 273 MB. A rewrite in Rust (`native/`, eframe/egui, no webview) reached parity with it: the same data sources and files in `~/.claude-usage-monitor/`, the same sizes, alerts, reminders and voice, at about 1% CPU at rest and a 13 MB app.

## Decision

Releases ship the native app on every platform, under the Electron app's identity: the bundle id `app.clauddy`, the same artifact names, the same `install.sh`. A user on the Electron app moves to the native one through the normal update, and their settings, accounts and alert state carry over because both read the same files. The Electron code stays in the repository and in its tests until it is removed on purpose; the release no longer builds it.

## Alternatives considered

- **Release the native app alongside, under its own id**: two apps to keep at parity, and users who never move over.
- **Keep Electron and tune it**: the cost is the engine itself, not the code on top of it.

## Consequences

- One app to build per platform, with a Rust toolchain on each runner; Linux needs GTK to build the tray.
- The AppImage is gone: Linux ships the tarball.
- The `clauddy` npm package still launches the Electron app until it is retired or repointed.
- A version must be newer than the last Electron release for the update to reach Electron users.
