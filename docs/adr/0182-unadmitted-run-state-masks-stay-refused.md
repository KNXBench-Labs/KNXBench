# ADR 0182: Unadmitted run-state masks stay refused until a source admits them

Date: 2026-11-21
Status: Proposed
Session: PRG-01-N4

## Context

PRG-01's Open list names other `070nh` masks, other object layouts and memory
run-state encoding. The [research note](../research/2026-11-21-run-state-remaining-masks-and-layouts.md)
finds no source for any of them; the simulator can verify only the refusal.

## Decision

Keep `UnsupportedMask` for every mask except the admitted set, and pin it with
tests for all remaining `070nh` nibbles and out-of-family masks. No code path
changes. See the [contract](../contracts/post-download-run-state-unadmitted-masks.md).

## Alternatives considered

- Admit more nibbles by family analogy: rejected — approval is per exact mask.
- Decode memory run state: rejected — encoding unspecified.

## Consequences

A future admission must come with source evidence and breaks these tests on
purpose. No hardware claim. Tests: unit/integration in `knx-net` simulator
(`unadmitted_masks`), including failed-load malformed input.
