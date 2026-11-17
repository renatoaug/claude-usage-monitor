# ADR-016: A failing snapshot gallery commits its own renders

- **Status**: Accepted
- **Date**: 2026-11-17
- **Related**: [design/08-native.md](../design/08-native.md#tests), `.github/workflows/snapshots.yml`, #94

## Context

The snapshot gallery renders through wgpu, which only the macOS runner has; Windows and Linux machines don't render it, not even on `main`. A contributor without a Mac who changes something visible can't refresh the PNGs, so the macOS job fails, and the only way out was a maintainer regenerating them by hand and pushing to the contributor's branch.

## Decision

When a pull request's checks fail in the macOS job's tests, `snapshots.yml` (on `workflow_run`) renders the gallery from the pull request's code in a job with read access only and no cache, and hands over nothing but the PNGs. A second job, which never runs that code, accepts only PNG files with a snapshot's name and commits them to the branch with the `SNAPSHOTS_TOKEN` secret (a maintainer's token that can push to a fork allowing edits by maintainers). The push runs the checks again, and the PNGs are reviewed in the diff like any other change. It follows the checks, so a first-time contributor's pull request waits for the same approval. The `update-snapshots` label runs it by hand.

## Alternatives considered

- **Upload the renders as an artifact for the contributor to commit**: no secret needed, but every visual pull request from someone without a Mac turns into a manual download-and-copy.
- **Only on a maintainer's label**: one more step on every visual pull request, for no extra safety, since the run already waits for the checks' approval.
- **Render the gallery on Linux too**: contributors could run it locally, but pixels differ between platforms, so it needs a second set of baselines or a tolerance that hides real drift.

## Consequences

- A contributor without a Mac pushes the change as it is and gets a green macOS job without asking anyone; they pull the bot's commit before pushing again.
- A render that changes nothing commits nothing, so a failure elsewhere in the tests is left as it is.
- The token is a secret the workflow depends on; without it the commit job fails with a message saying what to add.
- The workflow file runs from `main`, so a change to it takes effect only after it is merged.
- If the branch moves between the failing checks and the commit, nothing is committed: the new push's checks run it again.
