# ADR-002: The tests run as three processes

- **Status**: Accepted
- **Date**: 2026-08-15
- **Related**: [design/01-architecture.md](../design/01-architecture.md#tests)

## Context

`main.js` needs fake `electron`, `./usage` and `./auth` modules to be tested, and `renderer/pet.js` needs a DOM. Bun applies a module mock to the whole process, and it loads every test file before running any, so a mock declared in one file reaches the tests of the real module in another.

## Decision

The suite is three groups, each run by its own path in its own process: `test/unit/` (no mocks), `test/main/` (mocks `electron` and the data modules) and `test/dom/` (happy-dom). `bun run test` runs the three in turn. Bare `bun test` is never used.

## Alternatives considered

- **One process with mocks restored after each file**: bun has no reliable way to undo a module mock once a test file has imported it.
- **Dependency injection everywhere instead of mocks**: a large rewrite of `main.js` for the tests' sake.

## Consequences

- Each group sees only its own mocks.
- Bare `bun test` mixes the three and fails in ways unrelated to the change, which CONTRIBUTING warns about.
- Coverage has to be merged across three runs ([ADR-003](ADR-003-coverage-counts-every-file.md)).
