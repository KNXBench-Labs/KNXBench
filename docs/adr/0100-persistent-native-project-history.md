# ADR-0100: Persistent native undo and project versions

- Status: scoped local implementation verified; in-session self-review, no release claim
- Date: 2026-10-09
- Scope: HISTORY-01 and HISTORY-02

## Context and verified facts

The existing core CommandStack keeps inverse commands in memory; native open
resets it. Core types are not a versioned serialized command format. Native
store v10 writes the normalized model and passthrough in one transaction.
Full regression review found its flat line-vector order was coupled to area
traversal after reparenting; the v11 fix below closes that exactness gap.
Autosave uses the ordinary native Save route. Source evidence:
`knx-core/src/command.rs`, `knx-store/src/project.rs` and
`knx-server/src/domain.rs`; see [the contract](../PROJECT_HISTORY.md).

SQLite serialization of an in-memory database yields standalone database bytes;
deserialization into memory supports read-only access and excludes WAL images.
Sources inspected 2026-10-09:
https://sqlite.org/c3ref/serialize.html,
https://sqlite.org/c3ref/deserialize.html,
https://sqlite.org/lang_transaction.html.
The resolved rusqlite 0.40.2 source has safe `serialize` and
`deserialize_read_exact` wrappers behind its `serialize` feature. No custom
unsafe allocation or copying a live database file is necessary.

## Decision

Native v11 also stores `line.model_position` independently of the existing
per-area `line.position`. The migration backfills the preceding reader's
area/line traversal; new writes preserve the normalized flat vector and each
area's sibling list separately. A named RED/green regression and the existing
HTTP reparent/undo/reopen contract pin both orders. This is native storage
metadata, not a core-model or ETS-format change.

Add native schema v11 history tables and envelope version 1. Encode each history
state as a standalone native SQLite image produced through the existing store,
with empty history tables. Model images reference one immutable, content-addressed
retained-context image (opaque bytes and manufacturer references), so unchanged
source evidence is not duplicated per undo step. Context is validated and counted
inside the same transaction; unreferenced context is collected without trimming
any reachable history. This preserves current model semantics without inventing
a proprietary ETS revision parser or a command JSON wire format.

A private opt-in profiling run demonstrated repeated retained-context copying as
a real storage/serialization bottleneck. Only qualitative findings are public;
source names, item-level sizes, timings and identity digests are not published.
Core visits one inverse state at a time; a caller can stop admission before
materializing the remaining stack.

Baseline binding uses a versioned semantic digest of typed, ordered rows from a
fresh internal store, not a digest of physical SQLite pages. Image SHA-256 still
checks the exact stored image bytes. SQLite's header carries housekeeping and
writer-library-version fields (including offset 96), so physical image identity
is not semantic project identity. Primary sources inspected 2026-10-09:
https://sqlite.org/fileformat2.html and https://sqlite.org/datatype3.html.
Digest input includes the native model version, table and column names, explicit
value types and length-prefixed text/blob data; excludes history and internal
SQLite bookkeeping. Schema changes require migration, not a silent digest reset.

Core exports undo/redo states by replaying cloned inverse stacks. Reloaded stack
entries swap validated normalized snapshots; active entries still use inverse
commands. Restoring snapshots raises allocator counters to the current high-water
mark. The core remains unaware of SQLite, UI and snapshot envelopes.

The infrastructure layer owns snapshots, strict read/admission, transactions,
workspace generations and immutable versions. The application/service layer
coordinates native editor operations; the UI only requests/read-displays history
and obtains explicit restore/deletion consent. The independently saved root is
the clean baseline; the working journal retains unsaved native commands without
changing Save/autosave semantics.

New native mutations publish in memory only after durable commit. Generation
and saved-baseline binding reject concurrent editors. A normal writer that does
not supply editor history must preserve a differing recovered working state as
a version before invalidating its stack, rather than silently losing it.

Admission also compares native schema objects and column shapes with a fresh
store produced by the actual current migrations (including generated/hidden
columns). Unknown tables, views, persistent triggers or columns are refused
before editor load/write and during snapshot decoding, not partially captured.
The reserved `sqlite_` prefix is matched literally, not with SQL LIKE's `_`
wildcard. Extra indexes contain no project payload; they remain on the host
store, are not versioned, and any constraints they impose fail atomically.
Late rollback tests use an extra unique index so they still fail during writes
rather than merely exercising the earlier foreign-trigger admission refusal.
Ordinary same-connection root-write tests use `CREATE TEMP TRIGGER` on explicitly
qualified `main` tables and assert the injected message. TEMP triggers are
connection-local, not persistent native extensions (SQLite CREATE TRIGGER §7,
https://sqlite.org/lang_createtrigger.html, inspected 2026-10-09).

## Consequences and boundaries

- Full semantic native snapshots cost space/serialization work. Bounded admission
  refuses instead of silently trimming; no unmeasured large-project speed claim.
- History travels with `.knxdb`; it is not an independent disaster backup if that
  file/storage is lost. Copy the closed project to independent storage too.
- Versioned snapshots include retained project data and references, not the
  global manufacturer catalogue, passwords or actual bus/device state.
- New schema is intentionally refused by old builds; no downgrade is offered.
- Snapshot integrity hashes detect corruption, not malicious forgery or signatures.
- Changes before first Save As remain session-local; unsupported native extension
  tables and ETS restore-point semantics are not claimed as versioned data.
