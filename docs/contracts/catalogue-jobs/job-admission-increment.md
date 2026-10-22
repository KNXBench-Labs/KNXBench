# WP-04.3 stage D — owned job admission and replay kernel

Claimed 22 October 2026. The first increment of WP-04.3 stage D
([design-readiness.md](design-readiness.md) stage D: owned job/lifetime API)
under [ADR-0150](../../adr/0150-cancellable-catalogue-jobs.md). Stage D is the
largest stage; it is split into separately claimed increments in the staged
order. This increment delivers the admission/replay heart of the job manager
plus the strict wire primitives every later D increment builds on, as a
private unwired server module. No production route, upload, spool, status or
drain becomes reachable.

## Scope of this increment

A new private module `catalogue_jobs` in `knx-server` (declared with
`#[allow(dead_code)]`, like the existing `catalogue_job_control`) holding:

- **Strict wire primitives** (`strict` submodule), mirroring the P2 rules of
  [protocol-artifacts.md](protocol-artifacts.md):
  - `U64`: canonical ASCII-decimal `u64` newtype. Grammar `0|[1-9][0-9]{0,19}`,
    value at most `18446744073709551615`; rejects signs, whitespace, leading
    zero, decimal/exponent, Unicode digits, empty and overlong input; all
    arithmetic checked.
  - `JobId`, `Incarnation` (16 CSPRNG bytes) and `Sha256`, `Capability`
    (32 CSPRNG bytes), rendered as 32/64 lowercase hex. `FromStr` accepts only
    the exact lowercase-hex pattern (`^[0-9a-f]{32}$` / `^[0-9a-f]{64}$`).
  - `Capability` never has a `Display`, and its `Debug` is redacted; equality
    checks use a dependency-free constant-time byte comparison
    (`constant_time_eq`), so the secret is never echoed through any derived
    impl, log, status or error.
  - Minting uses the workspace `getrandom` (the OS CSPRNG), no new dependency.

- **Admission and replay manager** (`CatalogueJobManager`), implementing the
  P4.1/P4.2/R3 admission and bounded-replay semantics with one active slot, no
  queue, and 8+8 bounded terminal/tombstone rings:
  - One non-wrapping admission high-water serial per server incarnation.
    `next` starts at `1`; `u64::MAX` is the exhausted sentinel and is never
    allocated; allocation is `checked_add(1)` and exhaustion closes admission
    rather than wrapping.
  - `create(capability, body)` classifies in the contract order: prior
    incarnation, retained exact replay / conflicting metadata / tombstone,
    consumed-but-forgotten serial, future serial (`staleOffer`), then fresh
    admission. Retained replay is checked against exact incarnation, serial,
    capability (constant-time) and all create metadata **before** capacity;
    a capability mismatch is a generic `outcomeUnavailable`, never a
    conflicting-metadata oracle. A matching next serial is allocated exactly
    once; a capacity/stop/eviction refusal does not advance the serial.
  - Before a fresh admission with a full terminal ring, the oldest terminal is
    evicted first: if its cleanup is incomplete the offer is refused
    (`jobBusy`) without consuming the serial; otherwise it becomes a tombstone,
    evicting the oldest tombstone first. An active/unsettled record is never
    evicted.
  - `dispose(handle, capability)` models the DELETE outcome: quarantine
    (`artifactStoreUnavailable`), active/cleaning (`invalidPhase`), exact
    disposed tombstone (`disposed`), evicted-outcome tombstone and forgotten
    handle (`outcomeUnavailable`). An evicted tombstone never pretends DELETE
    succeeded.
  - Admission state transitions (`open`/`busy`/`stopping`/`exhausted`/
    `artifactStoreUnavailable`/`catalogueUnavailable`) and a read-only
    `admission()` projection with zero allocation/reservation.

The manager operates purely on owned in-memory state under one short ownership
domain (its mutating methods take `&mut self`); it performs no SQL, filesystem
I/O, await or parser work, so the sole writer->short-control nesting rule is
preserved by construction.

## Explicitly out of scope (later stage-D increments)

- The full serde wire DTO layer (strict `deny_unknown_fields` + duplicate-key
  rejection, closed-enum serialization and the complete `Status`/`Admission`/
  `Create`/`Error` field/null/state contract of P2/P3). This increment only
  supplies the strict primitives those DTOs consume.
- Upload/spool with sealing, status snapshots, progress/lease counters,
  retention deadlines and TTL, the ArtifactDir lease/inventory/reclamation and
  the real filesystem path fence (P4–P8).
- Standalone/desktop close/drain and the `EmbeddedServerHandle` lifetime (P9).
- Any production route wiring, Ready/apply, worker execution or UI.

## Done when

- The strict primitives reject every malformed grammar case and accept the
  exact boundary values; capability equality is constant-time and its value
  never appears in a derived `Debug`/`Display`.
- The manager reproduces the admission/replay/eviction/disposal decision table
  with focused tests: retained replay, conflict, tombstone, forgotten serial,
  stale offer, prior incarnation, single allocation under refusal, no-wrap
  exhaustion and bounded ring eviction.
- Focused tests and the owning `knx-server` suite pass; warning-denied Clippy,
  `cargo fmt` and header checks are clean; no production route becomes
  reachable and no quiet API, schema or response shape changes.

## References

- [ADR-0150](../../adr/0150-cancellable-catalogue-jobs.md)
- [design-readiness.md](design-readiness.md) (stage D ordering)
- [protocol-artifacts.md](protocol-artifacts.md) (P2 primitives, P4 operations)
- [managed-binding.md](managed-binding.md) (independent server/job/connection identity)
- [control-admission.md](control-admission.md) and the delivered stage-A
  primitive `crates/knx-productdb/src/operation.rs` (`PublicationGate`, `Stop`).
