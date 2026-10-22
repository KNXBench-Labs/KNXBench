# WP-04.3 stage D — ArtifactDir root, lease and filesystem fence (P8)

Claimed 22 October 2026. The fourth increment of WP-04.3 stage D
([design-readiness.md](design-readiness.md) stage D: owned job/lifetime API)
under [ADR-0150](../../adr/0150-cancellable-catalogue-jobs.md). Stage D is split
into separately claimed increments in the staged order. This increment delivers
the ArtifactDir ownership primitive and the filesystem fence as private unwired
additions. No production route, worker execution or drain becomes reachable.

## Scope of this increment

A new private module `catalogue_jobs::artifacts` plus an extension of the
existing `paths.rs` fence:

- **Root and lease** (`ArtifactRoot`): the fixed `.knxbench-catalogue-jobs`
  child of the managed catalogue's canonical parent. Root creation is an atomic
  `mkdir 0700` with a no-follow validation of an existing root (actual
  directory, owned uid, no group/other access, no symlink). The stable `lease`
  file is atomically `create_new 0600`, then locked with
  `rustix::fs::flock(NonBlockingLockExclusive)`; an existing lease is reopened
  no-follow and its owner/mode/link-count and descriptor/path inode equality
  verified before locking. Lock refusal fails admission closed. The lock
  descriptor is retained for the root's lifetime; no unlink/replace.

- **Incarnation and job directories**: bounded `i-<32hex>` and `j-<32hex>`
  subdirectories created `0700` with at most eight random-name collisions
  before `ioFailure`.

- **Bounded inventory**: an exact known-basename inventory (role/identity/state
  per owned file), bounded to 16 384 bytes, with `inventory.json` /
  `inventory.next` (create_new 0600, sync, replace only the still-owned prior
  inventory). No capability or raw alias is stored.

- **Filesystem fence** (`paths.rs`): the reserved root joins `.knxbench-tls` in
  `refuse_reserved` and `is_reserved_entry`, covering relative reads/writes and
  the absolute pass-through branches (canonical existing target or canonical
  parent + basename), including symlink aliases, and rejecting on a missing or
  failed canonicalization inside a potentially reserved path instead of the
  current best-effort `if let`.

## Explicitly out of scope (later stage-D increments)

- The SQLite native-sidecar adoption exception, full cleanup matrix and
  restart reclamation sweep; those need the rehearsal/recovery SQLite handles
  and the worker/route increment to exercise.
- `DetailQuery`/`Page`/`PublicRecord` artifact records (P6) and the
  result/preview artifact publication.
- Standalone/desktop close/drain (P9) and any production route wiring.

## Done when

- `ArtifactRoot::acquire` refuses a missing parent, a non-directory or
  symlinked root, a lock already held by another process, and a replaced or
  widened lease; a fresh acquisition round-trips and a second acquisition of
  the same root refuses.
- Incarnation/job directories are created 0700 and bounded.
- The fence refuses `.knxbench-catalogue-jobs` (and symlink aliases into it)
  on relative and absolute reads/writes and hides it from the root listing,
  while neighbours stay allowed.
- Focused tests and the owning `knx-server` suite pass; warning-denied Clippy,
  `cargo fmt` and header checks are clean; no production route becomes
  reachable and no quiet API, schema or response shape changes.

## References

- [ADR-0150](../../adr/0150-cancellable-catalogue-jobs.md)
- [design-readiness.md](design-readiness.md) (stage D ordering)
- [protocol-artifacts.md](protocol-artifacts.md) (P8)
- [wp043-protocol-policy.json](wp043-protocol-policy.json) (ownership)
