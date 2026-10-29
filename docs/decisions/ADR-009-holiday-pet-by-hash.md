# ADR-009: The holiday pet is picked by a hash of the year

- **Status**: Accepted
- **Date**: 2026-10-29
- **Related**: [design/06-pets.md](../design/06-pets.md#holidays)

## Context

On Halloween and Christmas a pet from the season's pack takes over, whatever the user picked. The pick should hold all day and across restarts, and change from year to year.

## Decision

`seasonalPet(date)` returns one pet of the pack whose days include the date, chosen by an FNV-1a hash of the year and the pack id. Nothing is stored. The saved pick (`config.pet`) is never changed and comes back the next day. There is no switch to turn the override off.

## Alternatives considered

- **A random pick saved on first launch of the day**: one more file, and a race between two windows.
- **A setting to opt out**: left out by choice; a holiday visit is the point.

## Consequences

- Every install sees the same holiday pet in a given year.
- Testing a holiday needs only a fake date.
