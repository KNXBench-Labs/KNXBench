# WP-04.3 stage D — upload spool sealing and status snapshots

Claimed 22 October 2026. The third increment of WP-04.3 stage D
([design-readiness.md](design-readiness.md) stage D: owned job/lifetime API)
under [ADR-0150](../../adr/0150-cancellable-catalogue-jobs.md). Stage D is
split into separately claimed increments in the staged order. This increment
delivers the sealed upload spool and the status-snapshot builder as private
unwired additions to `catalogue_jobs`. No production route, worker execution,
ArtifactDir lease or drain becomes reachable.

## Scope of this increment

New private modules under `catalogue_jobs` (declared from the existing
`#[allow(dead_code)]` module):

- **Sealed spool** (`spool`): writes the private `source.part` with one owned
  64 KiB I/O buffer (policy `caps.ioChunkBytes`), checked `u64` byte counting,
  SHA-256 over actual consumed bytes, and exact-EOF agreement
  (`written == declaredBytes`). Sealing flushes/syncs and closes the writer,
  then `rustix::fs::renameat_with(NOREPLACE)` renames `source.part` →
  `source.sealed`; no replacement/resume. The spool lives under a minimal
  private `0700` directory (`<root>/j-<jobId>`), created no-follow with the
  existing `std` primitives; the stable-root flock lease, inventory and
  reclamation are the separate P8 increment and are not duplicated here.

- **Status snapshots** (`status`): the complete P3 job-state representation
  (handle, source, profile, state/phase, gate, stopCause, progress, lease,
  apply, preview/result/failure/commit/cleanup/backup) with a checked revision
  counter (starts at 1, never wraps), monotonic-tick lease deadlines
  (receiving idle 60 s / overall 1 h, ready 15 min, working 2 h, retention
  1 h) and the 16 384-byte serialization bound. Building a snapshot reuses the
  delivered `dto` layer and emits only a contract-valid `Status`.

- **Manager integration (deferred)**: `CatalogueJobManager` records spool
  progress (received bytes, sealed digest) and produces status snapshots once
  it holds the full job state in the worker/route increment. The spool and
  status-assembly primitives delivered here touch no SQL, filesystem, await or
  parser work under the manager's short ownership domain.

## Explicitly out of scope (later stage-D increments)

- The ArtifactDir stable-root flock lease, inventory/reclamation and the real
  filesystem path fence (P8); the spool's minimal owned directory is hardened
  by that increment.
- Standalone/desktop close/drain and the `EmbeddedServerHandle` lifetime (P9).
- `DetailQuery`/`Page`/`PublicRecord` artifact records (P6) and the result/
  preview artifact admission.
- Any production route wiring, Ready/apply or worker execution.

## Done when

- The spool rejects short/oversize uploads before sealing, seals only exact
  EOF with the observed SHA and length, refuses resume/replacement, and
  round-trips the sealed identity.
- Status snapshots bump revision only on real transitions, enforce the lease/
  retention deadlines as checked monotonic ticks and never serialize beyond
  the 16 384-byte bound; every emitted `Status` passes `Status::validate`.
- Focused tests and the owning `knx-server` suite pass; warning-denied Clippy,
  `cargo fmt` and header checks are clean; no production route becomes
  reachable and no quiet API, schema or response shape changes.

## References

- [ADR-0150](../../adr/0150-cancellable-catalogue-jobs.md)
- [design-readiness.md](design-readiness.md) (stage D ordering)
- [protocol-artifacts.md](protocol-artifacts.md) (P4 upload, P5 status)
- [wp043-protocol-policy.json](wp043-protocol-policy.json) (durationsSeconds, caps)
