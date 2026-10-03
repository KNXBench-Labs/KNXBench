# ADR-0062: Activity history is durable metadata, not recovery or bus proof

- Status: Accepted (2026-10-02)
- Scope: Offline commissioning application/storage contract; no new KNX behavior

## Context

ADR-0055/0056 deliberately expose only volatile observed activity. A bounded
ring loses completed operations and a server restart loses even attempted-write
metadata. A global UI must not invent either history or successful hardware
outcomes from that ring. The Web surface is owned and locked by another track.

## Decision

Use a dedicated SQLite activity database, independent of `.knxdb` projects and
manufacturer databases. `knx-store::activity_history` owns its schema and
format version; the server owns the closed, payload-free activity vocabulary.
Version 1 uses an application identifier, monotonic storage sequence and unique
server-incarnation/operation identity. Unsupported versions and foreign
nonempty databases are refused without replacing them. No project migration,
KNX domain dependency on UI, new library, or imported format controls this model.

Persist each observed operation start, its possible-send boundary, and its
witnessed terminal state. SQLite transactions and `synchronous=FULL` are the
storage policy, not a claim of universal power-loss recovery. New files are
owner-only. Keep persisted rows rather than pruning them with the 64-entry
volatile ring; expose bounded cursor pages in operation-start order. A cursor
is not a change feed: ongoing rows must be refreshed from the live snapshot.

A prior incarnation's nonterminal record is reported as `unknown`, with an
explicit interruption marker, never as succeeded, cancelled safely, or idle.
Reading history cannot contact the bus, retry, restore, or acquire a tunnel.
The immutable backup remains separately necessary for a device write. A Debug
property write must durably record its possible-send boundary after its backup
and before its first property write; history failure at that boundary refuses
the write. Terminal persistence failure is surfaced as unavailable history,
not a fabricated `notSent` device result. Read-only observations may continue
when recording fails, but the snapshot must disclose that failure.

History contains only the incarnation/operation IDs, validated target,
closed operation/state tokens, timestamps and coarse write-evidence flags.
No keys, gateway, serial numbers, payloads, memory, host paths or exception
strings appear in history. Unknown/malformed stored metadata fails the page
explicitly and remains on disk; it is not silently skipped.

Observed read/record failure latches unavailable history. Subsequent start,
intent and terminal observations do not overwrite the failed records. Once a
database has been admitted during the server lifetime, missing or blank storage
is not automatically recreated. Both public HTTP commissioning write starts
check storage admission before tunnelling; this is not a durable download
journal and does not imply that every operation kind has history coverage.

`GET /api/bus/history` is guarded like other application APIs and explicitly
reports `coverage: partial`, format, persistence mode and the uninstrumented
operation kinds. This contract never claims all external clients or arbitrary
KNX traffic are tracked. Integration proceeds per caller with exact offline
regressions; the Web owner consumes the additive API after its lock becomes
available. A real hardware action still needs its existing scope, complete
recovery and fresh device-specific go.

## Consequences

Durable metadata is useful evidence after a server restart, but cannot undo an
operation or establish whether the gateway/device received a frame. No generic
crash cleanup, automatic restore, new cancel route, native UI acceptance,
serial-address-write permission or release waiver follows from this ADR.
The readiness ledger retains every remaining hardware, vendor and UI boundary.

## Admission amendment — 2026-10-03

SQLite can recover a hot rollback journal while reading from a writable
connection, before an application has queried its format or schema. Synthetic
RED reproduced a refused foreign database being changed and its journal being
deleted. Existing files now pass read-only identity/schema admission before
the writable connection, with a second admission check there. A hot journal
needing writes is refused; neither it nor its database is automatically repaired.

Read-only WAL connections can still write/create shared-memory sidecars.
Version 1 therefore supports rollback mode only. A bounded raw header check
refuses other read/write format versions before SQLite is opened, rather than
converting/checkpointing the file. Synthetic RED/GREEN tests preserve the main
file and journal/WAL bytes and reject shared-memory creation, for both openers.
This restriction applies only to activity history, not project/vendor stores.
It is not protection against hostile concurrent replacement of the data path.

Primary references, inspected directly: SQLite `lockingv3.html` §4.1;
`fileformat.html` §1.3.3 (header offsets 18/19); `wal.html` §5. Preserving a
failed database and all its sidecars is necessary before any explicit repair.
