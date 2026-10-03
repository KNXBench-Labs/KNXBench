# Commissioning activity history contract

[ADR-0062](adr/0062-durable-activity-history-is-not-recovery.md) separates durable
activity **metadata** from project data, product data, recovery images and bus
success. This backend contract is additive; Web adoption belongs to its owner.

## Read API

`GET /api/bus/history?after=0&limit=50` uses the existing API authentication.
`after` is a nonnegative storage sequence within SQLite's signed integer range;
`limit` is 1–100. Missing parameters use 0 and 50. Invalid/unknown query fields
return HTTP 400 without creating the database. Querying never opens a tunnel,
contacts the bus, retries, restores or mutates a project.

A successful page contains `format: 1`, `coverage: "partial"`,
`durability: "persistent"`, `entries`, `hasMore`, `nextCursor` and `untracked`.
Each entry has `sequence`, `serverIncarnation`, `interrupted`, `id`, `kind`,
nullable `address`, `state`, `startedAt`, nullable `finishedAt`, and optional
`writeEvidence`. A write receipt's only evidence fields are `backupRecorded`
and `sendPossible`; no payload, property bytes, serial number, key, gateway,
backup path or exception text is present.

Storage order is **operation-start order**, not timestamp or completion order.
Updating a row keeps its sequence; gaps are permitted. `nextCursor` pages older
operations, **not** updates to an already seen running operation. Refresh the
current page/live snapshot to observe those updates.

## Coverage and outcomes

Only `deviceCompare`, `serviceControlRead`, `serviceControlWrite` and
`serialLookup` have durable rows in version 1. `deviceIdentify`, `groupWrite`,
`serialAddress`, `deviceDownload`, `addressProgramming`, `busMonitor` and
`lineScan` are explicitly untracked in durable history. Existing long-session
snapshots remain volatile. Empty history means no recorded rows in this
particular database, not that no KNX action happened.

Read observations use `running`, `finished`, `failed`, `unknown`. Property
writes use `running`, `verified`, `noChange`, `notSent`, `effectUnverified`,
`unknown`. A write is `verified` only when its existing route witnesses its
readback. A send attempt, a durable intent and a transport acknowledgement are
not that readback. A record still `running` in a previous server incarnation is
projected as `unknown`, `interrupted: true`, with no fabricated finish time;
the stored row is not rewritten just because somebody read it.

`GET /api/bus/activity` retains its volatile session/ring contract and adds
`historyState: "disabled" | "configured" | "unavailable"`. `configured` means
configured, not independently healthy or complete. The factory used by the
server and desktop (`AppState::new`) configures history under the application's
data directory. The memory-only default is not permission to execute a public
write without history.

## Storage and refusal

The separate `activity-history.sqlite` has an application ID, format version,
exact schema and unique incarnation/operation key. New Unix files are 0600;
SQLite FULL synchronization and initial file/directory sync are storage policy,
not universal power-loss or manufacturer-recovery proof. Existing foreign,
future-version or schema-conflicting nonempty databases are refused without
replacement. Reads bound page count and metadata size, validate the closed
vocabulary, identities, timestamps and write-evidence consistency, and refuse
malformed rows instead of silently skipping them.

Version 1 supports rollback-journal SQLite files only, not WAL. Existing files
are header-checked before SQLite and identity/schema-checked read-only before
the writable opener; admission is repeated there. Hot-journal recovery needing
writes fails closed. Preserve the main file and all sidecars for an explicit
repair decision; no automatic journal rollback, WAL checkpoint or conversion
is authorized. This does not cover hostile concurrent filesystem replacement.

Once admitted during an AppState lifetime, missing/blank history is not silently
reinitialized. Observed read/record failures latch `unavailable`; later guards
retain only volatile observations and do not overwrite the invalid records.
No automatic repair/retry is performed. Preserve the database, investigate the
cause, and restart only after an explicit repair/recovery decision. Deletion
across a server restart cannot be reconstructed from metadata that no longer
exists; there is no claim of a complete forensic ledger.

Both reachable HTTP commissioning write starts (download and Debug property
change) check history admission before asking for a tunnel. The Debug action
also records its possible-send boundary **after** PID 8/PID 14 recovery and
**before** its first property write. A failure at either start returns HTTP 503;
a property pre-write callback/storage refusal uses the existing HTTP 507
backup-refusal contract. No property write follows that failed callback.
Terminal record failure does not turn a witnessed device write into `notSent`:
the device result remains its measured result and history becomes unavailable.
This does not add a durable download journal or guard another track's group
writes; those remain explicit coverage residue.

Rows are not automatically pruned with the bounded volatile ring.
Per-document/per-page bounds are not a total database-size quota. Disk growth,
local database backup/retention and long-session journalling remain explicit
operational/follow-up concerns; failure is visible and write admission fails
closed rather than silently deleting history.

Storage-policy references: SQLite's official `autoinc.html` documents increasing
but not necessarily consecutive IDs; `pragma.html#pragma_synchronous` documents
the synchronization policy. Neither implies universal power-loss recovery.

## Web-owner adoption

Do not regenerate/edit another owner's locked Web tree. Adopt the manual API
shape above; no generated binding change is required. Render partial coverage,
volatile versus durable data, unavailable storage, interrupted unknown and
unconfirmed device effects distinctly. Localize transport tokens; never use
`verified` history as a substitute for fresh device/readiness evidence or a new
hardware go. Test page refresh/pagination, malformed/unavailable responses,
restart identity and cancellation rendering before claiming UI/native support.

The full source-ID and dependency ledger is
[COMMISSIONING_ALPHA_LEDGER](COMMISSIONING_ALPHA_LEDGER.md).
