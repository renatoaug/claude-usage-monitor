# ADR-013: Pets are generated sprite sheets

- **Status**: Accepted
- **Date**: 2026-11-06
- **Related**: [ADR-008](ADR-008-pets-drawn-in-code.md), [ADR-012](ADR-012-the-native-app-replaces-electron.md), [design/08-native.md](../design/08-native.md)

## Context

The Electron pets were vector art drawn in code and animated with CSS ([ADR-008](ADR-008-pets-drawn-in-code.md)). The native app has no CSS engine, and its Clauddy is a pixel-art sprite in a richer style (outline, shading, a desk scene for work) that hand-written vector art does not match.

## Decision

In the native app a pet is a pair of sprite sheets in Clauddy's layout: working (4 × 2 frames, at the desk) and moods (4 × 4: idle, asleep, on fire, maxed out). The art is generated in Clauddy's style from the existing sheets as reference, its flat background cleared, and each sheet measured for where the art sits in a frame. Pets live in `native/src/components/skins.rs`, embedded in the binary; a pet's sheets are decoded the first time it shows. The saved choice stays `config.json`'s `pet`, and the seasonal rule ([ADR-009](ADR-009-holiday-pet-by-hash.md)) is unchanged.

## Alternatives considered

- **Porting the vector pets**: they would need a vector renderer and their own animation system, and would not look like the rest of the widget.
- **Drawing pixel art in code**: tried for Clauddy itself; it did not reach the generated art's quality.

## Consequences

- A new pet costs two generated sheets and a review of the art, about 1 to 2 MB in the binary.
- Every pet animates through the same sequences as Clauddy, with no per-pet code.
- A pet id only the Electron app knows is kept in `config.json` and shows as Clauddy.
