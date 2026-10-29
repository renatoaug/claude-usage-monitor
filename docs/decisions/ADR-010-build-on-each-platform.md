# ADR-010: Each platform builds on its own runner

- **Status**: Accepted
- **Date**: 2026-08-15
- **Related**: [design/07-releases.md](../design/07-releases.md)

## Context

electron-builder cannot reliably build the Windows and Linux apps from macOS, and the release must carry all three with the same version in their file names.

## Decision

The release runs in three stages: **version** (a semantic-release dry run computes the next version), **build** (a matrix on macOS, Windows and Linux runners, each stamping that version into `package.json`) and **publish** (downloads every artifact and runs semantic-release for real, to GitHub and npm).

## Alternatives considered

- **Cross-building from one macOS runner**: electron-builder does not do it reliably for Windows and Linux.
- **Releasing each platform on its own**: versions and releases drift apart.

## Consequences

- One merge ships one version for every platform.
- Windows and Linux builds cannot be reproduced on a Mac.
