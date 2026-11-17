# ADR-016: A failing snapshot gallery offers its renders, and commits them when asked

- **Status**: Accepted
- **Date**: 2026-11-17
- **Related**: [design/08-native.md](../design/08-native.md#tests), `.github/workflows/snapshots.yml`, `.github/PULL_REQUEST_TEMPLATE.md`, #94

## Context

The snapshot gallery renders through wgpu, which only the macOS runner has; Windows and Linux machines don't render it, not even on `main`. A contributor without a Mac who changes something visible can't refresh the PNGs, so the macOS check fails, and the only way out was the owner regenerating them by hand and pushing to the contributor's branch.

## Decision

When a pull request's checks fail in the macOS job's tests, `snapshots.yml` renders the gallery from the pull request's code in a job with read access only, no cache and no secrets, and hands over only the PNGs that changed. By default it comments on the pull request with what changed and a link to download them; nothing touches the branch. If the pull request asks for it, by ticking **Commit the snapshot gallery's renders to this branch** in its description or with the `update-snapshots` label, a job that never runs the pull request's code accepts only PNG files with a snapshot's name and commits them to the branch with the `SNAPSHOTS_TOKEN` secret. Ticking the box or adding the label later does it at once, once the checks have run. Asking is the author's call: nothing tells them to tick it.

## Alternatives considered

- **Always commit on a failure**: no step for anyone, but a bot changes someone's branch without being asked.
- **Only upload an artifact**: no secret and no bot comment, but nobody hears about it, and every such pull request turns into a manual download-and-copy.
- **Only on the owner's label**: a contributor from a fork can't add labels, so the call would never be theirs.
- **Render the gallery on Linux too**: contributors could run it locally, but pixels differ between platforms, so it needs a second set of baselines or a tolerance that hides real drift.

## Consequences

- A contributor without a Mac sees what changed without asking anyone, and gets a green macOS check with one tick if they want it; they pull the bot's commit before pushing again.
- It follows the checks, so a first-time contributor's code waits for the same approval; a tick before that approval does nothing.
- A render that changes nothing does nothing, so a failure elsewhere in the tests is left as it is.
- The commit needs the `SNAPSHOTS_TOKEN` secret; without it the commit job fails with a message saying what to add. The comment needs nothing.
- The workflow file runs from `main`, so a change to it takes effect only after it is merged.
