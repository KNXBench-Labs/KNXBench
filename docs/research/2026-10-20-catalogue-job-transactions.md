# Catalogue job transaction and cancellation research — 20 October 2026

Status: primary documentation inspected; **design candidate, not runtime
acceptance**. See the [proposed job contract](../contracts/catalogue-update-jobs.md).
No KNX format or manufacturer interpretation is introduced.

## Method and boundaries

The parent captured six complete official SQLite HTML bodies by Python HTTPS
and extracted readable text with `HTMLParser`, excluding script/style bodies.
The private retrieval manifest records status, final URL, content type and
HTML/text SHA-256. The configured page-extraction tool refused extraction because
its backend is search-only; that failed attempt is not evidence of reading.
No native-search citation handle is invented. Sources below are the actual
retrieved official URLs, registered in the task-owned citation ledger.

The two source inventories are bound to `f255f906`; the controller rehashed
all 110 repository/report entries without mismatch. This is source identity,
not execution. The inventoried rusqlite 0.40.2 archive matches Cargo.lock and
its six inspected source members; that does not establish runtime SQLite
version, compiled options or application behavior. The public documentation
is current documentation, not a claim that every detail was reproduced with
the pinned bundled engine. Subsequent source closure captures 19 official bodies
and checks 43 source members against six distinct lockfile-matching archives
across SQLite and reader reports. These populations overlap; they are not
product-test counts. Actual version/features and runtime behavior remain
unexecuted at that source-inspection checkpoint. A subsequent standalone
experiment is scoped separately below; it does not execute ProductDB.

## Scoped standalone mechanics observations

A private offline-locked Rust harness using checksum-matching rusqlite 0.40.2
and libsqlite3-sys 0.38.2 observed SQLite 3.53.2. Five synthetic DELETE-mode
mechanics cases were independently checked: the reserved writer used as backup
source returned Busy once; a separate pinned read-only/private-cache source
copied a fresh equal-page destination through 91 More plus one Done; a contender
process was excluded until writer release; abandoning More disposed only the
owned destination with unchanged live logical contents; retaining the reader
across writer COMMIT yielded Busy/active, then explicit rollback restored the
before-state. Reopened copy retained old rows while committed live held new rows.

The parent rehashed 1435 artifacts, eleven lock-bound archives and 1098 source
members, verified the native binary and independently regenerated fifteen exact
synthetic row-byte witnesses. The original harness compile failure and first
controller ownership/reap failure remain retained; later PID absence is not
proof that the failed owner correctly reaped its child. The fresh successful
cross-process attempt records both children exited zero and reaped.

This resolves only those dependency mechanics, not schema-28 ProductDB, full
witness/owner binding, cancel-versus-commit arbitration, native failure recovery,
resource enforcement, artifact crash durability, WAL or actual process lifetime.
The [consolidated contract](../contracts/catalogue-jobs/design-readiness.md)
requires new production-seam tests and independent readiness approval before
implementation. No product capability or completed WP-04.3 is inferred.

## Documented facts

### Stepped backup and completion [D]

The backup API requires distinct source/destination connections. Each positive
step copies up to the requested pages; only DONE establishes completion.
The official documentation labels a source write-operation refusal LOCKED, so
do not step from the IMMEDIATE writer handle. Other-connection source writes can restart backup.
The destination cannot be used through another API while the backup handle
exists. Finishing an incomplete operation rolls back its destination transaction;
a successful finish alone does not establish that copying reached DONE.[1]

**Pinned source distinction [SRC]:** the Cargo.lock-authenticated
`libsqlite3-sys 0.38.2` archive has SHA-256
`f1d20bef17f513b9b3004532233187769cd072d790971f4e4da0e346eb6401e8`.
Its `sqlite3/sqlite3.c` member has SHA-256
`0a409f1633283fa31a9126b11fbfd64a1991c5d30defad07e5745d4667f5e23d`
and declares SQLite 3.53.2. Lines 85056–85063 explicitly return BUSY for a source
in a write transaction. That is inspected source, **not** an executed result
or a claim the official label was observed. Either refusal forbids the design
from retrying that self-conflicting source forever.

**Design inference:** reserve W IMMEDIATE before writes; open distinct private-
cache read-only S after reservation, then BEGIN plus a finalized real read to
pin its committed snapshot. Copy to create-new destination D in positive steps
with finite retry/cancellation and observed Done. Drop backup handles before D
queries and end/close S before W writes/COMMIT; otherwise the owner's reader can
block DELETE-mode commit. Preview-only copying releases the unchanged writer
before offline rehearsal. Test this sequence and failure boundaries in the
proposed first DELETE-only lane; WAL remains explicitly unsupported until
separate proof, not technically impossible. The wrapper's ignored finish return
is not an independent durability witness.

### Writer reservation and explicit failure settlement [D]

SQLite permits multiple readers but one writer. IMMEDIATE starts the write
transaction immediately and can fail BUSY if another writer already exists;
unlike EXCLUSIVE in non-WAL mode it does not exclude other readers. BEGIN
transactions do not nest; nested work uses savepoints. COMMIT can return BUSY
while leaving the transaction active. Certain FULL/IOERR/INTERRUPT/NOMEM failures
may undo a statement or the transaction; transaction/autocommit state is needed
to distinguish that behavior, and explicit rollback gives a consistent cleanup
attempt. With journal mode OFF, rollback behavior is undefined.[2]

**Design inference:** reserve before final consent/binding validation; do not
invoke a separately committing migration or public installer inside that owner.
Classify rollback and uncertain outcome explicitly. Refuse unsafe journal
configuration rather than claiming atomic cancellation there.

### Connection-local observation and resource scope [D]

`data_version` changes when other connections commit, not when its own connection
commits; comparisons are meaningful only on the same connection. It is not a
global catalogue version. `max_page_count` controls maximum pages of a database
file and cannot be reduced below its current size; that statement alone is no
proof of a bound on a directory, journals, temp storage or parser heap.[3]

**Design inference:** combine a same-connection observer with deliberate
same-connection mutation/reopen identity and relevant content/incarnation checks.
Page and result caps are named allocations, not an aggregate disk/RSS promise.
The exact version-specific enforcement must be proved before adopting the
candidate policy numbers.

### SQL wakeup is not persistent cancellation [D]

Interrupt may be invoked from another thread while the connection remains live.
Near-complete operations might finish before interruption; interrupt when no
statement is running has no effect on future statements. Its effect lasts until
currently running statements complete, rather than forming an operation-wide
cancellation latch.[4]

Progress callbacks run periodically during SQLite work, use approximate VM
instruction intervals, and can interrupt by returning nonzero. They must not
modify the invoking connection. Only one handler exists per connection; newer
SQLite also calls it directly during some prepare work.[5]

**Design inference:** a persistent shared gate must independently forbid the
final COMMIT when cancellation wins. Scoped progress/interrupt support may
reduce native SQL latency but cannot control Rust XML/decode/report work or
replace commit arbitration. Remove/disable cancellation wakeups for required
rollback/cleanup and before reusing a connection.

### WAL snapshot scope [D]

A WAL reader keeps one end mark for its read transaction; readers and the single
writer may operate concurrently. A long-lived reader can delay checkpoint
progress. The WAL is separate from the main file, so main-file size/equality
alone does not describe committed catalogue contents.[6]

**Design inference:** snapshot, state-preservation and stale-binding tests must
include sidecars and normal checkpoint effects. Do not apply `immutable=1` to a
live changing catalogue or claim that a read-only open guarantees byte-identical
sidecar behavior. Detailed readonly/hot-journal support remains part of the
independent primary-source closure.

## Copy-header and native-lifetime review corrections

The final readiness review identified a separate backup transformation in the
same authenticated SQLite 3.53.2 member: backup records the destination's prior
schema cookie and updates it to prior+1 (`sqlite3.c:85084–85089,85131–85143`),
which need not equal the source's schema_version. This is inspected source,
not an executed schema-cookie test. The corrected contract preserves exact
schema_version in the live Ready/apply stamp and separately defines completed
copy-content equality plus fresh destination cookie 0 -> 1 validation. No cookie
is written to force equality; row/schema/source bytes remain fully covered.
Unequal-cookie pass and real content/wrong-cookie failure controls are future
production-seam tests, not credited to the earlier synthetic harness.

The same review distinguishes application-created private files from journals
created and written inside native SQLite, and ordinary duplicate Tauri close
notifications from a separately authorized force action. Their source-backed
contract corrections are design-approved by both independent final rereviews;
no application journal-FD ownership, GUI experiment or production shutdown result
is claimed. [ADR-0150](../adr/0150-cancellable-catalogue-jobs.md) authorizes the
staged implementation and its future tests, not those runtime guarantees.

## Candidate boundary, not a format fact

The first job may explicitly require an already-open current-schema file-backed
catalogue; old/missing/future state refuses before job-driven creation/migration.
Server startup migration remains a separate action and is not retroactively
rolled back by job cancellation. This narrower boundary avoids silently changing
migration/replay semantics while keeping old CLI/public entrypoints unchanged.
Independent source/caller review confirms both normal production entrypoints
initialize the managed catalogue before serving: standalone and desktop. This
is source feasibility, not tested compatibility or job cancellation of earlier
startup migration. Initial DELETE-mode admission is a separate explicit support
restriction, never a request-driven live journal conversion.

The [contract candidate](../contracts/catalogue-update-jobs.md) records proposed
source/result/page/lifetime policy and single-owner worker behavior. Numbers
are design choices, not measured capacities. Pinned reader/source closure supports cooperative slice-parser event/attribute/
skip checkpoints, typed operation-wide Stop and instrumenting both ZIP archive
views without dependency or parser replacement. It does not make whole-token,
namespace/value, dependency index-building, allocation/decoder/native calls
preemptible. The report's quiet-wrapper and fallback parity obligations remain
unexecuted. Exact managed owner/binding representation, enforceable resource
policies, durable-backup disposition and concrete strict DTOs remain unresolved.
The initial architecture review requested Ready/apply expiry ordering, a root
UI owner through wizard unmount and bounded replay semantics; the contract
revision proposes those resolutions for independent rereview. Production
implementation is not approved by this document. No tests, product build, latency/memory measurement, rollback,
compatibility, browser or shutdown result is claimed.

## Sources

[1] https://www.sqlite.org/c3ref/backup_finish.html
[2] https://www.sqlite.org/lang_transaction.html
[3] https://www.sqlite.org/pragma.html
[4] https://www.sqlite.org/c3ref/interrupt.html
[5] https://www.sqlite.org/c3ref/progress_handler.html
[6] https://www.sqlite.org/wal.html

## Schema-admission implementation basis — 21 October 2026

The Stage B1 read-only inventory identifies a new exact-schema admission boundary,
not a defect in the existing quiet version-based openers. Its 178 input hashes
(174 immutable candidate blobs/modes) and 30 output hashes are independently
reconciled at `c012f714`. The proposed helper is read-only and private; no complete
witness, managed binding, production A2 constructor, installer or API is inferred.
The actual bundled-engine migration/introspection oracle is pending separately.
The 46-table / 120-stored-object source inventory remains a static expectation,
not a newly measured database or a count-only admission rule.

Two official primary bodies were retrieved directly over HTTPS and their relevant
passages inspected. The configured extractor again refused because its backend
is search-only; actual retrieval uses urllib and HTMLParser excluding script/style
bodies. Private receipts retain final URLs, status/content type, complete bodies,
text digests and a task-local citation ledger. The current documentation establishes:

- `table_list` exposes schema/name/type, column count including hidden/generated
  columns, WITHOUT ROWID and STRICT flags; `table_xinfo` includes hidden/generated
  fields omitted by table_info. `index_xinfo` describes both key and auxiliary
  columns, including expression/rowid markers, direction and collation.
  Source: SQLite, *PRAGMA Statements*,
  <https://www.sqlite.org/pragma.html#pragma_table_list>,
  <https://www.sqlite.org/pragma.html#pragma_table_xinfo>,
  <https://www.sqlite.org/pragma.html#pragma_index_xinfo>.
- The schema relation has no stored row describing itself. Its SQL is normalized
  CREATE text as subsequently modified by ALTER; automatic PK/UNIQUE indexes have
  NULL SQL. Rootpage is observed layout metadata, not a universal allowlist value.
  Source: SQLite, *The Schema Table*,
  <https://www.sqlite.org/schematab.html>.

These are documented facts, not observation of the locked bundled runtime. The
implementation must compare actual final migration-generated SQL/introspection,
keep all unfamiliar variants refused and retain independent source/fixture identity.
Neither a static CREATE parser nor Python's separate SQLite runtime can certify
that oracle. A transaction borrow limits lifetime but does not prove immutability
against other calls on the same connection; later orchestration still owns the
transaction and mutation/reservation discipline. No compatibility promise for
unmeasured historical SQL variants follows from a fresh-chain fixture.
