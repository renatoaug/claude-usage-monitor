# ADR-003: The coverage gate counts every file that ships

- **Status**: Accepted
- **Date**: 2026-08-15
- **Related**: `scripts/check-coverage.js`

## Context

Coverage tools measure only the files the tests load. A new module with no tests is invisible: it does not lower the percentage, because it is not in the total. Bun's own threshold also applies per `bun test` run, and the suite is three runs ([ADR-002](ADR-002-three-test-processes.md)).

## Decision

`bun run test:coverage` runs the three groups, merges their lcov reports and fails under 80% overall, under 60% for any single file, or when a `.js` that ships is missing from the report. The list of files that ship is read from electron-builder's `files` in `package.json`, never kept by hand. CI runs it on every pull request.

## Alternatives considered

- **`coverageThreshold` in `bunfig.toml`**: one floor per run, no per-file floor, and an untested file still passes.
- **A hand-kept list of sources**: forgets exactly the new file the check exists for.

## Consequences

- A new file in the app needs a test from its first commit.
- The gate takes longer than `bun run test`.
