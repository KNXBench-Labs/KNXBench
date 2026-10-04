# Commissioning activity history contract

[ADR-0064](adr/0064-durable-activity-history-is-not-recovery.md) separates durable
activity **metadata** from project data, product data, recovery images and bus
success. This backend contract is additive; Web adoption belongs to its owner.

## Local version-evolution/lifecycle candidate (delivery pending)

Latest local candidate, checked 2026-10-04: **125 distinct registered tests
passed, 0 failed, 0 ignored**: full Store94, OneShot18, ten actual offline
worker cases and three public HTTP history cases. History13 is included in
Store94; the new 25-case synthetic malformed-download matrix is one included
OneShot leaf, not 25 extra tests. All 16 chain phases used the same declared
32-path source/build-input scope, HEAD and actual outer-alpha/workspace leases.
Actual worker and history-API integration rebuilds and all-target server/store
Clippy passed. Five production-source controls each compiled and failed at
the intended named semantic assertion: repeated terminal outcome, late send
intent, swallowed migration COMMIT error, admitted unknown download fields and
admitted zero session identity. Original bytes were restored in finally before
lease release; the canonical Store94/OneShot18 bookend passed separately and
is not added to the final-chain count.
The reader-blocked migration case observes a real dirty metadata journal and
checks refusal plus retained main bytes/version/documents/keys/cursor counter.
API cases execute the format2/future3/bounds/no-path contract. Private original
inputs remained unchanged; no private raw output was retained. This is a local
offline candidate, not integrated delivery/full SAFE-03/AUDIT-01, hardware,
vendor/ETS compatibility or universal crash/power-loss recovery evidence.
Explicit current-source start-before-contact and backup-save-failure cases
passed with the unchanged original commitment, no skip and no raw retained.
The malformed-download matrix includes a valid control followed by an invalid
second row: no partially successful page, no overwrite, unavailable history
and refusal of a new download, with main bytes/rows/identity/cursor/sidecar
preservation. The test-only addition changes no production statements. The
declared frozen scope is not every repository file; reviewed Rust diffs and
owner-path inventory are checked separately. Review is in-session, not an
independent whole-goal approval. Integrated review/acceptance and delivery
remain pending; SAFE-03/AUDIT-01 remains PARTIAL_BACKEND.

Earlier local source43 and its widened full-storage source124 are historical
snapshots, not extra current tests. The later test-only addition required the
fresh current-source chain above. Earlier positives below likewise remain
their own evidence scopes rather than retroactively acquiring new coverage.

The SAFE-03/AUDIT-01 continuation introduces storage/wire format 2 without
changing the legacy row schema. Known format-1 files are admitted read-only
first and upgraded transactionally by changing the version marker only.
Documents, incarnation/operation keys, sequence gaps and the next automatic
sequence are preserved. A history read can perform this metadata-only upgrade;
it does not mutate a project or contact the bus. Older format-1-only readers
refuse the upgraded file; no automatic downgrade is provided.

The historical preliminary storage-library snapshot had **91 passed, 0 failed, 0 ignored** tests,
including an independently pinned nonempty v1 fixture, busy-write migration
failure and legacy foreign-schema refusal. The focused HTTP history target now
passes **3/0/0**: bounds reject before creation, empty history declares version
and partial coverage, and foreign/future files remain unchanged. This is not a
nonempty download-receipt HTTP test or worker integration on that preliminary
snapshot. The later lifecycle and mutation evidence is stated above; complete
integrated package acceptance remains pending.
Download journalling is not implemented by this
version marker alone. The publisher snapshot passed **15/0/0** focused
metadata/OneShotLog tests after a separate compiled RED, before later caller edits. Configured durable publishers
reuse the strict row validation before opening history; an internal download
without its required evidence refuses before any DB/sidecar creation and latches
unavailable. This is not the download start/worker guard. The reader accepts
download metadata with separate device/restart/cleanup outcomes and projects
previous-incarnation pending cleanup as unknown without erasing a witnessed
result or rewriting the stored document. These reader fixtures are synthetic.
The initial actual caller start regression separately passed one selected offline
HTTP simulator test after its compiled missing-start RED: a durable pending row
is visible before fake tunnel acquisition. This does not prove worker terminal,
intent-failure or cleanup behavior. Minimum start ownership and conservative drop preceded the actual compiled
terminal-receipt RED (0/1/0). Only afterward were minimal intent/result/cleanup
hooks added; its new terminal GREEN passes one selected offline simulator test,
with a real server rebuild, both leases and unchanged inputs. This proves the
positive joined-worker receipt, not intent-error/abort/crash/cleanup fault cases.
The latest scoped evidence above supersedes these earlier snapshots without
retroactively broadening them. Full branch review/closure/integrated acceptance
and publication remain pending. See proposed
[ADR-0067](adr/0067-download-lifecycle-preserves-uncertainty.md).

## Read API

`GET /api/bus/history?after=0&limit=50` uses the existing API authentication.
`after` is a nonnegative storage sequence within SQLite's signed integer range;
`limit` is 1–100. Missing parameters use 0 and 50. Invalid/unknown query fields
return HTTP 400 without creating the database. Querying never opens a tunnel,
contacts the bus, retries, restores or mutates a project.

A successful page contains `format: 2`, `coverage: "partial"`,
`durability: "persistent"`, `entries`, `hasMore`, `nextCursor` and `untracked`.
Each entry has `sequence`, `serverIncarnation`, `interrupted`, `id`, `kind`,
nullable `address`, `state`, `startedAt`, nullable `finishedAt`, and optional
`writeEvidence`. A write receipt's only evidence fields are `backupRecorded`
and `sendPossible`; no payload, property bytes, serial number, key, gateway,
backup path or exception text is present.

The candidate reader also recognizes `kind: "deviceDownload"` with optional
`downloadEvidence` at the DTO level and mandatory evidence for that kind.
It contains a positive `sessionId`, nullable `written` (`yes`, `no`, `partially`),
nullable `restart` (`acknowledged`, `notInPlan`, `unconfirmed`) and `cleanup`
(`pending`, `returnedOk`, `returnedError`, `unknown`). Unknown enums/fields and
inconsistent evidence are refused, not skipped. A prior-incarnation pending
cleanup sets `interrupted: true`; an already witnessed device result and its
timestamp remain intact. The four-kind baseline coverage declaration deliberately continues to list
`deviceDownload` in `untracked`. This unpublished local candidate instruments
bounded download start/intent/result/cleanup; that is not permission to claim
complete download coverage or change the declaration without integrated owner
acceptance. `untracked` is conservative coverage metadata, not proof that no
local candidate download row exists.

Storage order is **operation-start order**, not timestamp or completion order.
Updating a row keeps its sequence; gaps are permitted. `nextCursor` pages older
operations, **not** updates to an already seen running operation. Refresh the
current page/live snapshot to observe those updates.

## Coverage and outcomes

The established baseline declares durable coverage only for `deviceCompare`,
`serviceControlRead`, `serviceControlWrite` and `serialLookup`; these kinds have
durable rows in version 1. `deviceIdentify`, `groupWrite`,
`serialAddress`, `deviceDownload`, `addressProgramming`, `busMonitor` and
`lineScan` are explicitly untracked in that baseline declaration. The local
bounded download candidate above does not close this wider coverage residue.
Existing long-session snapshots remain volatile. Empty history means no recorded rows in this
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
SQLite EXTRA synchronization and initial file/directory sync are storage policy,
not universal power-loss or manufacturer-recovery proof. Existing foreign,
future-version or schema-conflicting nonempty databases are refused without
replacement. Reads bound page count and metadata size, validate the closed
vocabulary, identities, timestamps and write-evidence consistency, and refuse
malformed rows instead of silently skipping them.

Supported legacy version 1 and current version 2 accept rollback-journal SQLite
files only, not WAL. Future versions are refused before migration. Existing files
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
The local candidate adds only the bounded download metadata lifecycle described
above, not a complete commissioning journal or recovery engine. It does not
guard another track's group writes; those remain explicit coverage residue.

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
