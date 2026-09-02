# ADR 0009: UI boundary via generated projections

Date: 2026-09-02
Status: Accepted
Session: 1

## Context

`CLAUDE.md` requires that the KNX core does not depend on the UI, and that UI
workarounds do not substitute for fixes in the domain or application layer.

The data is large enough that shipping it wholesale into a browser is a design
error, not a performance detail: 514 group addresses and 907 communication
object instances in the reference project (RESEARCH §4.1), and that is one
mid-sized installation.

Two languages meet here (ADR-0001), so any hand-maintained correspondence
between them will drift.

## Decision

Tauri commands expose display-shaped projections — `ProjectTree`, `DeviceList`,
`GroupAddressTable`, `Inspector<T>` — rather than the domain types. The
projections are generated into TypeScript with `ts-rs`, so the two sides cannot
disagree without the build failing.

The UI sends `Command` values back. It never holds a mutable reference to the
model. Filtering, sorting and pagination happen in Rust, against indexed
storage.

## Alternatives considered

**Mirror domain types one-to-one into TypeScript.** Couples the UI to the
internal model, so every model refactor becomes a UI refactor, and it ships far
more data than any view needs.

**Hand-written bindings.** They drift, and they drift silently — the failure
shows up as a wrong value on screen rather than as a compile error.

## Consequences

A projection layer to design and maintain, which is also the place where "what
does this view need" gets answered explicitly.

The UI cannot invent state, so disagreements between what is shown and what is
stored are resolved in one place.

A build step generates the TypeScript bindings, and CI must fail if the
generated output is stale.
