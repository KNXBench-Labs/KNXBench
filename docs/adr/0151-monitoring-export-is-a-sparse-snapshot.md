# ADR 0151: Monitoring export is a sparse snapshot with a bound loss report

Date: 2026-10-20
Status: Accepted implementation decision after bounded independent design review;
required corrections recorded in the contract, runtime acceptance pending

## Context

Offline monitoring needs current names, group addresses, object relationships,
topology, buildings and Functions, without a native engineering backup or a bus
connection. Existing CSV/documentation exports do not provide this JSON projection
and do not define atomic publication of a document/report pair. A saved root can
differ from recoverable native working state; an open session can differ from both.

The independent Core distinguishes unavailable object flags from known false and
does not establish group-level security. A complete external-shaped boolean
record has no way to express those unknowns. Pretending they are false would be
misleading; silently discarding an entire normal project's addresses would make
an otherwise useful monitoring projection unusable.

## Decision

1. Keep external mapping in `knx-app`, not Core or UI. Add no model/native/product
   schema or mandatory runtime dependency. Read optional product context only.
2. Define an explicit reduced `monitoring-sparse-v1` profile: omit unavailable
   security and flag fields with diagnostics. Do not claim complete-schema
   conformance or receiving-application interoperability.
3. Export one installation, preserving current values and stable project-local
   relationships. Detect collisions, corrupt graphs and bounded-work exhaustion
   before publication. Addressless-device omissions and all other lossy mappings
   are counted and disclosed. Original source does not override current edits.
4. Package plain `project.json`, a versioned hash-bound `export-report.json` and
   `README.txt` in one deterministic ZIP. CLI publication is create-new/no-clobber;
   no two-file rename is called an atomic pair.
5. File readers use a consistent recovered native working snapshot, falling back
   to root only when no working history exists. HTTP reads the live snapshot;
   full incarnation/revision binding and a recomputed bundle digest prevent stale
   preview downloads, including changed catalogue-derived output.
6. CLI file admission requires quiescent standalone rollback-layout SQLite input
   and no WAL/SHM/journal entries, including resolved input-symlink targets. Refuse
   unknown layouts and dangling catalogue links before either opener; never recover,
   checkpoint or ignore live WAL. This is not concurrent-writer protection.
7. UI reviews counts, omissions, private data and not-backup scope before local
   download. Client code never resolves engineering fields itself; it verifies the
   received ZIP bytes against the reviewed bundle digest before saving.

## Consequences and limits

The projection is immediately useful to a separately implemented/tested sparse
reader, but is not a drop-in promise for a complete typed external format. It
cannot be imported back as a lossless engineering project. Manufacturer sources,
parameters, opaque bytes, attachments, keys, history and recovery remain outside
its payload. Native history loading still uses the existing validated editor
reader; constant-space reading is not claimed.

Source/shape checks, native/CLI/HTTP/UI tests and real receiver execution are
separate acceptance boundaries. No real receiver or hardware claim follows from
a green internal gate. See the [behavioral contract](../contracts/monitor-project-export.md).
