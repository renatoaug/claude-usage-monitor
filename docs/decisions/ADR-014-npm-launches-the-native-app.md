# ADR-014: The npm package launches the native app, and the Electron code is gone

- **Status**: Accepted
- **Date**: 2026-11-06
- **Related**: [design/03-setup.md](../design/03-setup.md), [design/07-releases.md](../design/07-releases.md), [ADR-012](ADR-012-the-native-app-replaces-electron.md)

## Context

After [ADR-012](ADR-012-the-native-app-replaces-electron.md) releases shipped the native app, but `bunx clauddy` still installed Electron (about 270 MB) and ran the old widget, and the repository kept the Electron app, its three test processes and its coverage gate for code that no longer shipped.

## Decision

The `clauddy` npm package is a small launcher with no dependencies (`bin/clauddy.js`). The first run of a version downloads that version's release build for the OS from GitHub, unpacks it into `~/.claude-usage-monitor/bin/<version>/`, deletes older ones and starts it; later runs start it straight away. The launcher exits once the app is up, so it costs nothing while the app runs.

The Electron app, its tests, its build scripts and its icon generators are removed. The repository's JavaScript is only the launcher, with its own test.

## Alternatives considered

- **One npm package per platform with the binary inside** (optional dependencies): no download at first run, but three more packages to publish on every release and a binary in npm for each.
- **Keep the Electron code until later**: two apps in one tree, with the docs describing the one that does not ship.

## Consequences

- `bunx clauddy` needs the release of the same version on GitHub; a version published to npm without its release assets cannot start.
- The first run downloads about 10 MB instead of Electron.
- [ADR-002](ADR-002-three-test-processes.md) and [ADR-003](ADR-003-coverage-counts-every-file.md) no longer apply: the tests they shaped are gone.
- The `clauddy <state>` simulator went with the Electron app; the native app takes `CLAUDDY_STATE` instead.
