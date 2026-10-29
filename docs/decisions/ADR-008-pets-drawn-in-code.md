# ADR-008: Pets are drawn in code and ship inside the app

- **Status**: Accepted
- **Date**: 2026-10-29
- **Related**: [design/06-pets.md](../design/06-pets.md), [#65](https://github.com/renatoaug/claude-usage-monitor/issues/65)

## Context

Picking a pet should not make the app heavier on disk, and the widget has no server to load assets from. The ten pets are vector drawings that weigh about 50 KB together, against an app of 273 MB.

## Decision

Every pet is an SVG builder function in `renderer/pets.js`, with its animations in `renderer/pets.css`, bundled with the app. Nothing is fetched at runtime. A pet is drawn inside `#claude`, so the mood animations keep applying, and a new pet is a data change ([adding a pet](../design/06-pets.md#adding-a-pet)).

## Alternatives considered

- **Downloading pets on demand**: saves 50 KB, needs a network to show the pet, and puts remote SVG into the widget's page.
- **Raster sprites or GIFs**: heavier, and they cannot react to moods or the cursor.

## Consequences

- Pets work offline and cost nothing in app size.
- Every pet is code, reviewed and tested like the rest.
