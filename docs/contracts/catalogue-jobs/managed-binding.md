# Managed catalogue ownership, witness and transaction composition

Status: **final design candidate; not implemented or independently approved**.
This is the concrete ownership/binding supplement to the
[job contract](../catalogue-update-jobs.md). The complementary
[protocol/resources/artifacts/lifetime contract](protocol-artifacts.md) and
[review record](design-readiness.md) must be read together; neither delegated
scope verdict independently approves production. All controls below are
required future executions, except the explicitly limited synthetic SQLite
mechanics observations. Existing public quiet API and schema remain unchanged.

## 1. Fixed support and compatibility decisions

1. The new controlled service accepts an **existing, healthy, managed, current schema-28 connection** only. Initial support is Linux/Unix local default SQLite VFS, private cache, normal locking, UTF-8 database encoding, journal mode **DELETE**, one `main` database, no application TEMP objects, no outstanding transactions/cursors/BLOB handles, no shared-cache/URI immutable/nolock/alternate-VFS configuration. Refuse unsupported conditions; never convert journal mode, migrate, create the live database, attach a database or run ANALYZE to obtain admission. Startup's existing open/create/migrate is independently committed setup, outside job cancellation.
2. Existing public ProductDB quiet signatures, exhaustive legacy errors, source-name semantics, v1 token bytes, and synchronous HTTP requests/responses remain unchanged. In particular, `/api/catalog/install` stays standalone-package-only; old update stays wrapper-capable STANDARD on the server; legacy/password import remains separate. New controlled code has its own errors/binding types, not flags bolted onto old endpoints. No old path is relabelled controlled or newly bounded.
3. Integrate the **server's ownership**, not the public quiet service contracts. Existing legacy path-based apply really opens its own SQLite connections. Holding the server owner lock and advancing its generation covers server admission; it does not turn that library function into same-W execution. Standalone calls outside this owner, including CLI and independently used public services, are external SQLite writers observed by W's persistent data_version and the full witness. Do not invent owner-generation events for them.
4. No new persisted table, column, user_version, trigger, change-log or catalogue UUID. Binding/generation/incarnation are volatile capabilities. Exact witness equality is an equality check, not a durable commit history, a substitute for rollback or an assertion that all schema-compatible data is semantically valid.
5. Ordinary **managed** restore/reopen is supported and invalidates before replacing/closing anything. Arbitrary external unlink/rename/replacement of an open database, alias writes/hard-link access, external recovery/format/configuration rewrites, and hostile same-user filesystem activity are **not supported**. A detected or uncertain identity fails closed and quarantines; a filesystem stat is not a pathname reservation. An offline external replacement requires closing the owner and opening a new incarnation before another preview.

## 2. One owner, one protected managed state

Replace the internal `Option<Mutex<Connection>>` with `Option<CatalogueOwner>`. Keep existing absent-database behavior. ProductDB owns the managed connection/type and transaction algorithm; server owns locking, worker admission and legacy-route adapters. No generic resource/task registry and no second parallel metadata mutex:

```rust
// PROPOSED, module names/types not implemented.
pub(crate) struct CatalogueOwner {
    state: std::sync::Mutex<knx_productdb::ManagedCatalogue>,
}

pub struct ManagedCatalogue {
    conn: Option<rusqlite::Connection>, // private; None while retired/reopening
    incarnation: CatalogueIncarnation, // fresh random 128-bit value, never reused
    generation: u64,                   // checked increment, never wraps
    health: WriterHealth,
    identity: Option<ManagedFileIdentity>,
    observation: Option<WriterObservation>,
}

pub enum WriterHealth {
    Healthy,
    Mutating, // lock is owned; guard has not yet proved safe settlement
    Retired(RetireReason), // deliberate close/restore/reopen in progress
    Quarantined(QuarantineReason), // no reads or writes through this handle
}

struct WriterObservation {
    data_version: i64, // last observation on THIS exact W, not S or a reopened W
    total_changes: u64, // conservative local diagnostic, not commit history
    file: FileStamp, // dev/ino/nlink/len/mtime+ns/ctime+ns; no atime
}

struct CatalogueStamp {
    incarnation: CatalogueIncarnation,
    generation: u64,
    data_version: i64,
    total_changes: u64,
    file: FileStamp,
    witness: CatalogueWitness,
}
```

`ManagedFileIdentity` retains the admitted absolute/canonical pathname privately, parent directory identity, device/inode/link count and the explicitly selected default-VFS configuration. No local path escapes into a public job result. Incarnation is not the server incarnation or a secret; all three of server/job/connection identity have independent purposes. `generation=0` on new adoption, with a fresh incarnation. Poisoned mutex is a refusal, not `expect`-panic/recovery by blindly using its inner state.

Only a read lease exposes `&Connection`, scoped to the lock; it is a **repository-internal** capability, not exported as arbitrary SQL authority. Audit the actual readers when changing the field type. No `DerefMut`, `into_inner`, cloning W, escaping guards, saved prepared statements, or public unguarded `&mut Connection`. A read lease first rejects non-Healthy/active transactions and never mutates observation counters as if a read were a known writer. Existing private readers can be mechanically adapted without changing their responses. Memory-only test catalogues remain usable by quiet APIs/readers; controlled jobs explicitly refuse them.

### Mutation admission and finish

- The sole server nesting direction is owner/W -> short job-control lock. No job manager/reaper/status/cancel path waits for the owner while holding job-control; no project lock is held with it. New blocking workers acquire the owner with try_lock + cancellation/deadline-aware bounded waits, not one uninterruptible `.lock()`.
- Every actual server mutator obtains a managed mutation guard. Before calling any possibly mutating quiet library body: verify health/autocommit, **checked-increment generation, set Mutating and arm quarantine-on-unwind**. Successful aliases, duplicates, skipped installs, zero removed rows and no-op retries still consume a generation. Refusals fully validated before entering the body may leave it unchanged; a conservative increment after admission is always allowed. Failure never rolls generation back.
- The guard is an implementation-local owner of `&mut ManagedCatalogue`, not another shared owner. Its ordinary finish validates no escaped native work, autocommit=true, same admitted path/handle where applicable, and records an updated observation. Success sets Healthy. It never proves a quiet helper's failed COMMIT by inspecting only autocommit.
- For quiet shared-W installers, non-SQL semantic errors proven by source control flow to precede the only COMMIT may be settled as no-commit **only** after autocommit=true/no outstanding statements. SQLite/IO errors, failed COMMIT, active transaction after return, panic, identity uncertainty or a legacy helper with an indeterminate partial outcome quarantine. The adapter may conservatively quarantine more failures; it must not fabricate clean rollback. Preserve the original endpoint's mapped error body/status; quarantine affects subsequent availability rather than changing success/failure wire shapes. Do not report a successful quiet result as committed job evidence.
- Quarantine blocks all reuse, invalidates extant bindings by advancing generation if possible, preserves recovery data, and is visible as catalogue unavailable. A health snapshot can be cheaply projected to job control after releasing W. Ready does not need an eager event to be safe: exact stamp validation is mandatory at apply. No automatic retry, silent fresh W or automatic restore. Explicit recovery closes/retire the old handle and adopts a new incarnation only after normal health/current-schema/identity checks. A fresh handle does not resolve an old unknown outcome.
- Generation overflow closes write/job admission; do not wrap or mint a same-incarnation zero. A deliberate re-adoption with a new incarnation is a separately observed operation, never an invisible retry.

### Actual mutation integrations (not just the new job)

| Actual server surface / source | Admission and integration | Shape and limitation preserved |
|---|---|---|
| Startup `AppState::with_product_db_at`, domain.rs:256-272 | Existing `open_and_migrate` completes **before** constructing healthy managed state/new incarnation and its first observation. Failed startup remains unavailable. | Existing create/migrate behavior is not job work; no cancellation rollback promise for it. |
| POST `/api/catalog/install`, routes.rs:839-864 -> domain.rs:3868-3885 | Managed guard on W around existing `install_package`; increment even when report.skipped. Quarantine uncertain SQL settlement. | Same standalone-only parser, `CatalogInstallReportDto`, refusal mapping and synchronous outcome. |
| POST `/api/catalog/install-legacy`, routes.rs:897-956 -> domain.rs:3918-3979 | Password/decryption stays outside W. Guard around the actual `publish_legacy` call at 3954; skipped adoption still increments. Password remembering remains after publish and outside W. | Same password/remember semantics and report fields. A remembered-password write/failure alone is not catalogue generation or a failed catalogue commit. |
| POST `/api/catalog/update/apply`, catalogue_update_routes.rs:210-245 | Guard spans old **path-based** apply/backup helper through return; invalidate before invocation; same-W data_version refreshed only after it ends. Any error after the path helper may have opened live state quarantines unless the adapter proves a pre-live-open refusal; W's autocommit alone cannot settle another connection. | Exact old request/token/v1/result mapping remains. Its separately opened writers remain outside controlled transaction guarantees. |
| POST `/api/catalog/cleanup/apply`, catalogue_update_routes.rs:577-599 | Guard on W inside existing blocking closure; existing cleanup owns its own IMMEDIATE transaction. Do not nest the new ProductDB transaction around it. Increment even on admitted no-op. | Confirmation/item selection/token/backup/result unchanged. A cleanup error without verified rollback is not healthy just because Drop ran. |
| POST `/api/catalog/restore`, catalogue_update_routes.rs:357-385 | Fresh-name validation unchanged; then checked generation increment, **Retired and new incarnation before** copy/close/reopen; invalidate all old stamps. Hold the owner for the whole sequence. Invoke existing restore with old W quiescent (it is an online backup into the same file, not an external rename); on success explicitly close old W and `open_and_migrate` the restored path. Adopt healthy reopened W with fresh identity/observation only on success. | `{restored:name}` only after reopened shared state is usable. Restore/open/migration failure remains original mapped error, but old W is quarantined/retired, never left silently usable. Do not claim restore is a cancellable install transaction. |
| POST `/api/catalog/backups/delete`, catalogue_update_routes.rs:395-432 | Keep owner serialization + confirmation + fresh listing/identity checks. Conservatively increment before the actual removal attempt; verify W still Healthy/autocommit after. Pure pre-validation refusal need not increment. File-removal failure alone does not make W uncertain; record it, keep Healthy if W is unchanged. | Exact name-only/identity-bound request and `{deleted:name}`; no deletion of recovery backup on job expiry. Generation false-stale is intentional, not a claim that backup deletion changed DB rows. |
| New controlled apply | Managed ProductDB owner + one actual same-W outer IMMEDIATE; increment before admitted mutation attempt, after matching the old stamp, and before copying/installing. | New binding only, shared noncommitting installer. No v1 substitution. |

GET backups, old update preview, cleanup preview and catalogue SELECT/enrich paths are not catalogue mutators. Rehearsal writes only private D. Legacy preview may allocate/migrate its in-memory copy but does not increment the live owner as a fake write. Backups created by old apply/cleanup are already inside the corresponding mutating admission. Filesystem routes that could reach the live DB are **not** another managed restore: arbitrary file replacement while open remains unsupported and must be refused/fenced at their resolver if it targets this managed catalogue; SQLite+stamp is not permission to support it. New workspace fencing and root leases belong to the artifact/lifecycle scope.

## 3. Persistent W observations and ordinary external ABA

A `CatalogueStamp` captured for Ready stores W's **actual value at that snapshot**, not a global latest value overwritten by a later read. `observation` is persistent bookkeeping on the one managed W. It does not replace the immutable stamp held by the job. On initial adoption take a real main read before observing. At preview and apply:

1. Hold owner, check Healthy/autocommit, admit exact source/policy and identity, acquire **W IMMEDIATE** with finite retry.
2. Finalize a real main-schema read on W, then sample `PRAGMA main.data_version` on W. Observe full logical state on **that W transaction** and sample again after scanning. No comparisons with S/D, another connection or values saved across reopening. An unexpected intra-reservation version/stamp change refuses/quarantines instead of resetting the baseline to make it match.
3. Bind the W data_version, own generation, incarnation, conservative total_changes and file stamp with the witness. At apply compare the old W value with a fresh W value under IMMEDIATE **before local DML**. Any difference stales, including logically equal catalogue content. Read/query helpers may update diagnostic observations but cannot rewrite the already issued stamp.
4. A normal CLI write or old public path service that commits between preview and apply is detected by same-W data_version even if it restores all rows to the same final values (ordinary ABA). A managed same-W mutation is detected by generation even though SQLite does not increment W's data_version for W's own commits. Local rollback/no-op attempts intentionally stale by conservative generation. An in-place restore through another SQLite connection can change data_version; managed restore additionally changes incarnation regardless of equal bytes. Uncommitted external transactions do not constitute a committed catalogue change; W's reservation excludes their publication during validation/install.
5. File metadata is sampled after reservation and around opening/copying S, before W DML; W/S each must report `SQLITE_FCNTL_HAS_MOVED` supported/success and `moved=0`. The proposed small audited wrapper uses pinned rusqlite `Connection::handle()` and `rusqlite::ffi::sqlite3_file_control` for `main`, not SQLite internal struct offsets or a guessed FD. Compare symlink_metadata regular-file/no-link, dev/ino/nlink==1/length/mtime+ns/ctime+ns with admitted identity; compare canonical parent identity. Reject no such primitive/VFS, unsupported platform, mismatch, vanished main, sidecar/recovery uncertainty or inconsistent pre/post sample. S's path string alone proves nothing. No immutable/nolock. Do not create/delete live sidecars as a check.
6. Check connection settings/`database_list`, schema/encoding/mode again under reservation. New-path settings must include W foreign_keys=ON, ignore_check_constraints=OFF, writable_schema=OFF, defer_foreign_keys=OFF and query_only=OFF; bind/check the observed trusted_schema/recursive_triggers values in the managed configuration receipt rather than silently changing legacy defaults. Unknown/unreadable configuration refuses. W has one main and no application TEMP state; private-cache S is distinct and READ_ONLY. Initial lane refuses WAL/SHM or rollback journal/recovery artifacts unexpectedly present at job admission rather than guessing cold/hot recovery safety. Recovery at earlier ordinary startup is outside the job.

**Finite-observer boundary, not a universal log:** SQLite data_version is connection-local and finite, not a durable monotone sequence. Equal values plus exact final witness cannot establish that no full native-counter cycle/reset occurred, that no unobserved same-content rename happened, or that arbitrary external tools never violated VFS rules. New binding does not support claims against unbounded counter-cycle writers, raw-file editors, external configuration/recovery writers, broken locking/network filesystems, malicious timestamp restoration or hostile same-user replacement. A deployment requiring those guarantees must refuse this initial job capability; do not add a partial hash or undocumented “watcher” to pretend otherwise. Lost/reopened/uncertain observation continuity is a concrete refusal/new-preview boundary, never cross-connection equality. Normal CLI SQLite commit interleavings within the ordinary same-handle continuity contract remain covered. This limitation is explicit initial support, not an open TODO to build a global epoch service.

## 4. Typed full-current-schema streaming witness v1

`CatalogueWitness { encoding_version: 1, sha256: [u8;32], table_count: u32, schema_object_count: u32, row_count: u64, encoded_bytes: u64 }`; all arithmetic checked, all errors typed. `table_count` counts scanned relations including sqlite_schema (47 in the source-derived base); `schema_object_count` counts stored sqlite_schema rows (120 in that base), not a fictitious self-row. The complete-object accounting is that value plus the explicit schema relation. Hash streaming directly; no `format!("{:?}")`, `group_concat`, JSON coercion, delimiter concatenation, first-N filtering, source-hash-only shortcuts or collection of all rows/values. This is a new binding field, **not** a rewrite of v1 preview token.

### Exact object/data inclusion and supported schema shape

- Stream the **complete main.sqlite_schema relation** (including its hidden rowids, type, name, tbl_name, rootpage, nullable sql), ordered by schema rowid, plus explicit descriptors described below. Do not filter `name NOT LIKE 'sqlite_%'`. Include metadata for all explicit and autoindexes, constraints via SQL and introspection, and any object encountered; an unsupported object causes whole-operation refusal, not omission followed by success. There are no supported views/triggers/virtual/shadow tables in the present application schema. Never execute a view or trigger to witness it.
- Scan every persisted column and every accessible hidden rowid of **every supported main table**, independently of package membership or source SHA relationships. Thus aliases, unowned/standalone sources, legacy originals/payloads, history/unavailable markers, winner/declaration/provenance rows, placements/sections and diagnostics cannot silently disappear from the witness. Index btree cells are derived state, not separate application rows; their full definitions/introspection/rootpage are included. No raw free pages, journal bytes, page-cache layout or deleted-cell slack claim. Integrity/corruption checking is separate.
- Current source-derived base has **46 ordinary STRICT rowid tables**. All 46 have declared PKs. `ingest_unknown.id INTEGER PRIMARY KEY` aliases rowid; the other 45 have TEXT or composite PKs and an independent hidden rowid. **None currently declares WITHOUT ROWID, AUTOINCREMENT, or columns named rowid/_rowid_/oid.** Do not conflate a composite PK with WITHOUT ROWID. The schema source companion enumerates the 46 PKs, source definitions and statically derived final column-name order; its separate creation-column lists omit ALTERs, and none of these lists is a runtime schema receipt.
- Count convention: static source yields **120 stored sqlite_schema objects = 46 tables + 24 explicit indexes + 50 inferred PK/UNIQUE autoindexes**; the complete catalogue object universe is **121 when sqlite_schema itself is included**, since SQLite does not store sqlite_schema's own CREATE TABLE as another sqlite_schema row. The two partial unknown-identity indexes come from report.rs, not migration.rs alone. The witness scans the schema relation explicitly, not a count-only guess. Do not assert that a runtime database with 121 *stored* entries was inspected here. Object-count variants cannot bypass shape checking. If the parent's “121 objects” means 121 stored entries instead of this convention, the extra object's actual definition must be admitted or refused, never silently dropped.
- Initial allowlist is exactly those 46 table names from [schema source inventory](schema28-source-inventory.json), their final migration-28 columns/PKs, the 24 named explicit indexes (including predicates) and the inferred autoindexes with expected index_xinfo. Validate `table_list` type/wr/strict, `table_xinfo` cid/name/type/notnull/default/pk/hidden, `index_list` origin/unique/partial, `index_xinfo` key/aux/cid/name/desc/collation, foreign_key_list and exact supported schema SQL variants. Freeze accepted SQL variants emitted by the supported migration chain as implementation fixtures; byte-different unfamiliar shapes refuse rather than reinterpreting CHECK/default/collation semantics. Runtime enumeration is bounded **before retaining descriptors** by new-path witness policy: at most 256 stored schema objects, 64 ordinary tables, 256 columns per table, 256 bytes per identifier, 256 KiB per schema SQL value and 2 MiB aggregate retained schema/introspection metadata. Exceeding any ceiling is `WitnessSchemaAdmissionLimit`, not a format maximum or a legacy refusal-policy change. Read lengths before bounded metadata fetch; stream the admitted schema relation after shape validation. The exact expected shape still overrides these generous upper ceilings; 256 unknown objects are not accepted merely because they fit.
- `sqlite_sequence` is **not excluded** by a prefix filter. Current schema has no AUTOINCREMENT, so any sequence table/AUTOINCREMENT shape is an initial unsupported-schema refusal. Its future support must hash all sequence rowids/name/seq as well as table rows; exact row equality without sequence would miss future insertion behavior. Similarly, initial `sqlite_stat1`/`sqlite_stat4` or other extra internal tables refuse, not silently ignore optimizer state. They are not found as creations in the current source chain; foreign ANALYZE databases can use quiet APIs, not silently gain controlled support.
- WITHOUT ROWID, generated/hidden columns, PK-less application tables, shadow rowid aliases, custom collations/virtual tables/triggers/views and unknown schema versions/shapes are initial **UnsupportedSchemaShape**, not an attempt to invent a hidden-rowid fallback. A future WITHOUT ROWID codec must order by the actual non-null composite PK in declared PK order with explicit collations and encode all its values; it cannot `SELECT rowid`. This future capability is not required for current-schema v1.
- Exclude only non-main schemas from the data stream **after rejecting** additional attached databases and application TEMP objects. SQLite's implicit empty temp schema is not catalogue data. Scratch before-state tables live only in D's private separate DB, never W or S. No application table is excluded by purpose, prefix or absence of package references.

### Canonical encoding, source bytes and ordering

1. Start with fixed ASCII domain `KNXBench/CatalogueWitness/1` plus a NUL and u32 big-endian encoding version. Emit explicitly tagged records with big-endian fixed integers and u64-length-framed byte strings. Include user_version=28, schema_version, application_id and UTF-8 encoding tag; bind mode/private-cache/locking policy separately. Include introspection descriptors above in deterministic table-name byte order, columns by cid, PK parts by ordinal, indexes by byte name and xinfo seqno, foreign keys by (id,seq). All field counts/record boundaries are explicit. Freeze record tags as header `0x10`, descriptor `0x11`, table-begin `0x12`, row-begin `0x13`, row-end `0x14`, table-end `0x15`, stream-end `0x16`; cell tags remain `0x00..0x04` below and appear only at a cell position. Descriptor tuples use the same typed cell codec and explicit tuple/field counts; a byte string is u64 byte length followed by exactly those bytes, never a terminator. Table-end records carry u64 row count; stream-end carries u32 scanned-relation count, u32 stored-schema-object count and u64 total row count, all before finalizing SHA-256. `encoded_bytes` is the checked external stream-byte count, not recursively included in itself.
2. For the schema relation and each table: begin-table tag, exact identifier bytes/descriptors and column count; iterate `SELECT _rowid_ FROM main."table" ORDER BY _rowid_ ASC` on the reserved W transaction, row begin tag + signed i64 two's-complement rowid in BE, followed by each stored column in cid order, row end; finish table with checked row count. `_rowid_` is chosen only **after** table_list.wr=0 and case-insensitive shadow-name validation; do not interpolate an unchecked client identifier. `ingest_unknown` emits both rowid and declared id (redundant intentionally). Rowid is witness data because existing queries and source insertion order can use it. Avoid PK sorting/key materialization and duplicate-row ambiguity. Schema names/SQL may be encoded through the same cell codec after their bounded shape admission. No collection/sort of all data rows in Rust.
3. Each cell begins with actual SQLite storage-class tag: NULL `0`, INTEGER `1` + i64 BE, REAL `2` + exact returned IEEE-754 f64 to_bits u64 BE, TEXT `3` + byte length, BLOB `4` + byte length. No numeric stringification, invalid-UTF8 replacement, Unicode normalization, NULL/empty collapse, INT/REAL conflation, lossy text or application “zero unavailable” projection. Hash the values actually in SQLite; stored FLOAT normalization is not a claim to preserve non-stored NaN payload bits. Typed enum/state SQL TEXT is still raw stored bytes, not reserialized Rust enum names.
4. Obtain `typeof(col)` and, only for TEXT/BLOB, `length(CAST(col AS BLOB))` as integer **before fetching contents**. Enforce checked length/offset and the new-path **1,000,000,000-byte per-cell** bound (schema SQL/identifier/aggregate metadata use their smaller limits above) before allocating. This cell ceiling is an explicit witness policy, not a claim that arbitrary legacy rows fit native encoded-row limits. Fetch BLOB as `substr(col, ?offset, ?take)`; TEXT as `substr(CAST(col AS BLOB), ?offset, ?take)` so byte lengths, embedded NUL and invalid UTF-8 do not use SQLite character counts. One-based offsets, positive `take=min(65536,remaining)`, require returned storage class BLOB and exact requested length; then hash borrowed bytes and finalize/reset each row view before the next chunk. Empty byte values need no data query. Never read `source_file.bytes` or `package.bytes` wholesale or trust their len/size/SHA columns instead of actual bytes. Include both metadata fields and full actual bytes, including standalone/outside-package sources and legacy files.
5. Use rowid predicates (`WHERE _rowid_=?1`) on the same transaction for cell/chunk queries; rowid iterator can remain as a bounded cursor on W, with all cursors finalized before settlement. Ordinary INTEGER/REAL cells use a fixed-size ValueRef; TEXT/BLOB never become whole Rust String/Vec or UTF-8 `row.get::<String>`. Dynamic SQL identifiers come exclusively from admitted source descriptors, are quoted by doubling quotes and size-bounded. Check cancellation/deadline between rows/cells/chunks and before/after SQL. Counts/lengths/offset overflow or any read/type inconsistency refuse, never truncate or omit the row.
6. Set/read back a connection-local native length limit as a **finite admission bound**, compatible with the existing 1,000,000,000 raw/member ceiling and backed by actual compiled SQLite limits; a value/row that cannot be represented refuses, not a silent lowered legacy policy. Restore settings after the controlled scope. SQL length/substr bounds returned/application-owned chunks, **not a promise that SQLite's CAST/substr or btree machinery never materializes an input value internally**. Such native work remains under the admitted cell/row limit and the 8 GiB catalogue-input policy, not an aggregate RSS guarantee. No whole-value Rust allocation is required by this algorithm. No copied Python accounting module or Python/second-SQLite implementation is the production witness. The production codec is small Rust next to ProductDB orchestration, using existing sha2.

Store the witness digest/counts and source/preview digests only; do not persist a second catalogue image or expose source bytes. SHA-256's collision-resistance is the cryptographic equality assumption; it is not mathematical injectivity. The complete stream is typed/lossless before hashing and complete-or-refused. Complete preview identity has a separate versioned typed encoding covering every admitted preview field, all ordered programs/versions/counts/wrapper/package/other-member facts and explicit nulls, plus immutable artifact digest/count. Never use the weaker v1 token alone, partial summary counts or public redacted projection as raw equality.

### Copy validation is distinct from live Ready/apply equality

The live `CatalogueWitness/1` above remains **unchanged**, including the actual
source `schema_version`. Original Ready/apply compares this exact same-W witness
and stamp; no normalization or schema-cookie write can make a stale stamp pass.

A completed native backup has a separate `CopyContentWitness/1`: fixed ASCII
`KNXBench/CopyContentWitness/1` plus NUL, u32 BE version 1, followed by the same
record/cell codec, complete schema/introspection/rowid/stored-value stream and
counts as section 4. Its header includes the same user_version, application_id
and UTF-8 tag but **omits only schema_version**. There are no other omitted or
normalized fields: schema SQL, schema rowids, rootpages, descriptors, all table
rowids/cells and actual source/package bytes remain exact. Both witnesses can
be fed in one bounded traversal, with their distinct headers/domains and shared
payload, not by deriving one digest from the other. A copy witness is never
accepted as a `CatalogueStamp` or live consent binding.

Every D and B is fresh, exclusively owned, equal-page-size, never previously
used for a backup or DML, autocommit and empty main schema. Before creating
Backup, observe and retain its actual schema cookie **0** and empty schema,
finalize those read cursors and verify no transaction remains. Configuration
cannot create schema. If these prerequisites fail, dispose/quarantine and
allocate a new destination; do not edit its cookie or reuse an interrupted copy.
Record that pre-copy receipt outside the destination in the owned bounded
inventory/control state. Each destination gets exactly one backup operation;
More/Busy steps of that same handle do not constitute separate operations.

Pinned SQLite 3.53.2 records the destination's prior cookie at backup transaction
start and sets it to prior+1 on completed copy (`sqlite3.c:85084–85089,
85131–85143`), not to the source cookie. The narrow fresh-zero lane therefore
requires destination cookie **1** after observed Done, dropping Backup and
closing/reopening the destination read-only. No overflow arithmetic or arbitrary
nonfresh-cookie support is needed. This is an exact pinned behavior to verify,
not an application `PRAGMA schema_version=...` instruction. Any other observed
cookie refuses as copy-validation failure; neither quick_check nor equal rows
alone can override it. Source cookie may differ from 1 and stays in the original
live witness, never rewritten to match D/B.

Before rehearsal DML or recovery publication, compare the fully recomputed
reopened D/B `CopyContentWitness/1` with pinned S's original pre-DML copy witness
(and its exact W prestate), plus the explicit destination-cookie receipt/rule.
Require schema policy, user_version, application_id, encoding and page-size/count
admission independently too. No source S, private rehearsal D or backup B commit
can claim the live publication gate. This narrow comparison is only for the
completed **pre-install** copies; after rehearsal DML its old-copy identity is
not expected to remain equal. Raw file/header/free-page equality is not promised.

Future acceptance must include source cookie unequal to 1 while the untouched
copy passes content equality and destination-cookie=1, exact live Ready/apply
still detecting a changed cookie, real copied row/source-byte and schema/rowid
changes failing copy equality, wrong destination cookie failing despite equal
content, and nonempty/nonfresh destination refusal. These are required tests,
not executed observations of the earlier five-case mechanics harness. A future
SQLite version/backup transformation change requires explicit codec/rule review;
do not silently widen equality.

## 5. Exact proposed ProductDB signatures and staged capabilities

Declarations below are **proposed**, with conceptual lifetime names; they are not currently callable/compiled APIs. Concrete structs remain intentionally small. The exact internal server adapters are:

```rust
impl CatalogueOwner {
    // Startup passes the existing ordinary opener's completed result here.
    pub(crate) fn adopt_opened(conn: Connection, configured_path: PathBuf)
        -> Result<Self, OwnerError>;
    pub(crate) fn with_read<T, E>(&self,
        read: impl for<'c> FnOnce(&'c Connection) -> Result<T, E>,
    ) -> Result<Result<T, E>, OwnerError>;
    pub(crate) fn with_legacy_mutation<T, E>(&self, kind: LegacyMutationKind,
        work: impl for<'w> FnOnce(&'w mut Connection, &'w Path) -> Result<T, E>,
        classify: impl FnOnce(&Result<T, E>) -> LegacySettlementHint,
    ) -> Result<Result<T, E>, OwnerError>;
    pub(crate) fn restore_legacy(&self, listed_backup: &Path)
        -> Result<(), OwnerRestoreError>;
}
```

The higher-ranked borrows prevent an ordinary returned `T` from retaining W/path borrows; these trusted adapters remain crate-private, and the restore adapter is dedicated because replacement/retirement cannot be a generic mutator closure. `LegacyMutationKind` is a closed enum (`DirectPackage`, `LegacyPublish`, `PathUpdate`, `PlacementCleanup`, `BackupDelete`); `LegacySettlementHint` is `SuccessfulReturn`, `PreCommitSemanticRefusal` or `Uncertain`, a hint the guard independently checks against native/health state, not permission to declare a failed commit rolled back. `OwnerRestoreError` retains the old UpdateError/reopen error mapping and managed disposition separately. `with_read` has no arbitrary public SQL API promise; the source audit owns the read-only closure rule.

`ControlledSource` is a bounded borrow authenticated from a sealed source, not a client path; current raw-byte installer bodies may still need the already-admitted full source slice. This memo does not claim spool mmap/heap or artifact durability closure.

```rust
#[derive(Clone, Copy)]
pub struct ControlledSource<'a> {
    pub(crate) name: &'a str,
    pub(crate) bytes: &'a [u8],
    pub(crate) sha256: [u8; 32],
    pub(crate) limits: PackageLimits, // bind BOTH numeric fields
}
pub struct OperationContext<'a> {
    pub gate: &'a PublicationGate,
    pub checkpoints: &'a dyn OperationCheckpoints,
    pub policy: &'a ControlledPolicy,
}
pub struct ControlledPreview {
    pub preview: AdmittedPreviewArtifact, // full immutable raw identity, safe projection separate
    pub binding: PreviewBinding,          // fields private; server retains it
}
pub struct PreviewBinding {
    version: u32,
    catalogue: CatalogueStamp,
    source: SourceIdentity,  // exact name bytes + length + digest
    limits: EffectiveLimits, // max_member_size + max_expanded_size
    policy: PolicyIdentity, // version + effective policy values
    preview: PreviewIdentity,
}

impl ManagedCatalogue {
    pub fn preview_current_controlled<'m, 's, 'o, 'a>(
        &'m mut self, source: ControlledSource<'s>,
        op: &'o OperationContext<'o>, artifacts: &'a mut dyn CatalogueArtifacts,
    ) -> Result<ControlledPreview, ControlledError>;

    pub fn apply_current_controlled<'m, 's, 'b, 'o, 'a>(
        &'m mut self, source: ControlledSource<'s>, shown: &'b PreviewBinding,
        op: &'o OperationContext<'o>, artifacts: &'a mut dyn CatalogueArtifacts,
    ) -> ControlledApplySettlement;
}

// Private ProductDB helpers. Only orchestration constructs a reserved tx.
fn begin_reserved<'w>(w: &'w mut Connection)
    -> rusqlite::Result<rusqlite::Transaction<'w>>;
fn witness_current<'w, 'o>(tx: &'w rusqlite::Transaction<'_>, op: &'o OperationContext<'o>)
    -> Result<CatalogueWitness, ControlledError>;
fn copy_pinned_current<'s, 'd, 'o>(source: &'s Connection, destination: &'d mut Connection,
    op: &'o OperationContext<'o>) -> Result<CopyDone, ControlledError>;
fn install_admitted_product_file_controlled<'t, 's, 'o>(
    tx: &'t rusqlite::Transaction<'_>, source: ControlledSource<'s>,
    op: &'o OperationContext<'o>,
) -> Result<AdmittedInstallResult, ControlledError>;
```

`begin_reserved` sets Immediate and explicit DropBehavior below. Do not store a Transaction inside ManagedCatalogue (it would borrow its own connection). Destructure/borrow the conn, stamp/generation and health fields disjointly inside the orchestration scope; the armed outer guard outlives the local Transaction. Finalize/drop that Transaction before taking/replacing conn on retirement. This avoids a self-referential owner or unsafe lifetime extension. `copy_pinned_current` independently requires READ_ONLY main, active S read transaction and the recorded real-read/identity receipt; reject a write-capable W as its source before calling Backup::new. Transaction-private internal leaf functions can continue to accept `&Connection` where quiet composition needs it; the controlled **orchestrator's** entry takes a borrowed Transaction, never calls a public committing installer and never exports a commit-capable transaction to server routes. `AdmittedInstallResult`/`AdmittedPreviewArtifact` cannot be created by truncating existing reports; the shared core must preserve quiet semantics while receiving explicit quiet/controlled contexts. `ControlledApplySettlement` carries original failure, commit evidence (`NotCommitted|Committed|Unknown`), rollback disposition, writer health and owned-artifact disposition independently. Server job/server/consent binding is checked atomically at R1 admission; a caller-supplied ProductDB binding alone grants no HTTP authority.

**Implementation-friendly stages; no prematurely reachable production path:**

- **A: gate only.** Small non-SQL PublicationGate API (`try_cancel`, `try_claim_commit`, state snapshots), typed Stop and settlement enums. Test both deterministic interleavings, same lock/CAS as expiry/stop. Independent of owner/copy/parser; no claim of cancellable installer.
- **B: managed binding + disk-copy capability.** Owner/adoption/all-route generation/restore-health integration, source identity, full witness, persistent W baseline and W/S/D copy recipe. Can produce a validated private **snapshot capability** and stale/refusal decisions without an installer. It is not a Ready/apply user capability and cannot claim preview parity or actual cancellation. Tests can use schema-28 synthetic data and fault/barrier seams later; no fake “controlled install” stub returns success.
- **C: shared installer transitive controls.** Reuse accepted cancellation-reader decisions: explicit typed Stop through wrapper/package/ingest, both ZIP views, every reread/hash/skip/parser/classification/report/fallback and final admission. Quiet wrappers pass quiet control; old APIs remain public and unchanged. Only once C and the separate artifact/result policy are verified may ProductDB expose actual controlled preview/apply and the server admit Ready. No stage-B direct fallback to old apply, hidden schema creation or report materialization without prior admission.

## 6. W/S/D recipes

### Preview

Acquire owner cancel-aware, require Healthy/current lane, authenticate source and effective limits/policy. Keep the owner for stamp/snapshot acquisition; do **not** advance generation for this unchanged reservation. Take W IMMEDIATE, establish a real W read, sample data_version/metadata and stream the full witness on W. Only after reservation open distinct READ_ONLY/private-cache S to the verified same path. S begins DEFERRED, executes/finalizes a real main read (BEGIN alone does not pin), verifies schema/page shape and identity; W/S prestate must agree at admitted metadata/digest boundaries. S is backup **source**, never W. Compute original copy-content witness as
defined in section 4 alongside the unchanged exact live witness; they are not
interchangeable.

Exclusively allocate D using the separate artifact owner, configure admitted page size/cache/main-page limits before copying, require D autocommit. Use manual positive bounded steps (candidate 128 pages), finite busy/retry/deadline and persistent Stop checks. Require **observed Done**; drop backup before any D SQL/close. Incomplete or merely openable/quick_check-valid D is not a completed copy.
Validate reopened pre-install D against the exact section-4 copy-content stream
and fresh-zero-to-one destination-cookie receipt; do not require its full live
witness cookie to equal S. End S's read transaction explicitly, finalize and close S. Recheck W's baseline and file identity; explicitly roll back the unchanged W reservation, verify autocommit=true, disarm mutation quarantine and release owner. No source sidecar deletion and no live data DML. Offline rehearsal on the disposable D may now run the shared installer and derive exact complete preview identity, after its result allocations have been admitted. The saved catalogue stamp remains the **original acquisition** stamp even if another writer ran during offline rehearsal. Any mutation while parsing will stale apply. Releasing the owner during the offline part must be implemented by a snapshot/rehearsal split, not by retaining `&mut ManagedCatalogue` across unlock; the combined conceptual API above may initially retain the lock longer for simplicity. Longer lock duration is allowed; silently claiming an unlock while a borrow escapes is not.

### Apply

R1 atomically consumes Ready consent under job control/deadline/stop, then worker waits for owner with no control lock held. ProductDB verifies source name/length/actual bytes digest, both limits, policy and complete shown preview identity. Require Healthy, establish W IMMEDIATE before live DML, and compare the **unchanged original** incarnation/generation/data_version/total_changes/file/schema/full witness under reservation. A mismatch is stale: no recovery backup and no live install. The freshly sampled values must not replace the old expected values before comparison.

After matching, mark managed mutation admitted and checked-increment generation, arm settlement/quarantine. Open/pin verified S as above while W remains unchanged. Create D rehearsal and the pre-write recovery destination B from the same **unchanged S**, not from an installed rehearsal. Copy with observed Done/drop, separate section-4 copy-content equality and
fresh destination-cookie validation for each D/B, plus artifact-owner rules. Explicitly end/close S before any W DML or COMMIT. Explicitly keep W reserved through the second offline rehearsal on D, full result admission/comparison, recovery-backup completion and live install. D can commit its scratch installation without claiming the live publication gate. Any copy/preview/report/consent-relevant mismatch refuses and explicitly rolls W back.

Run only the shared noncommitting controlled installer body on W. Check exact preview/report agreement and finish all fallible result admission before publication claim. The gate's one Open->CommitClaimed winner then executes the **one live COMMIT**; if CancelAccepted won, no COMMIT is executed and W explicitly rolls back. No second retry COMMIT, migration, rename-D-over-W, late source reread, fallible report expansion or “reopen to decide success” after the claim. Record witnessed commit independently before returning response/refresh/cleanup. B remains recovery data for committed/unknown; proved rollback may dispose only verified owned artifacts according to the separate artifact contract.

Accepted narrow SQLite spike evidence supports this recipe, not its product implementation: same-W backup actually Busy; reserved W plus pinned distinct S copied with 91 More/1 Done and S closed before W writes/commit; a held S made W COMMIT Busy/autocommit=false and explicit ROLLBACK restored autocommit=true/old rows; a real contender got Busy while W reserved and acquired after release. It used synthetic DELETE-only tables, not schema-28/owner/ABA/cancellation. The original controller attempt failed to reap its writer/record its wait exit; later PID absence is not retrospectively a clean first attempt. Only the successful fresh cross-process attempt had both children reaped. Do not rerun or inflate that evidence here.

## 7. Explicit transaction ownership, Drop and outcome classification

Use pinned rusqlite `Transaction<'w>` created with `transaction_with_behavior(Immediate)`. Immediately set **DropBehavior::Ignore**; the managed mutation/reservation guard is armed first and **must quarantine on unwind/unfinished scope**. Default Drop rollback is not a verification mechanism, and `Transaction::commit(self)` on error otherwise consumes the owner and can hide a rollback attempt. Chosen implementation retains the Transaction and executes `tx.execute_batch("COMMIT")` exactly once after the gate, or `tx.execute_batch("ROLLBACK")` explicitly on unwind. It inspects `tx.is_autocommit()` while the owner is still available. Then drop the Ignore transaction and release the managed guard only with a recorded settlement. Ignore means no accidental second hidden action; it is safe only because unfinished execution forbids reuse and retires/closes W, never leaves an active W available to later requests.

Scoped cancellation/interrupt issuers and progress handlers are detached/drained before the final commit claim as well as before rollback, so a late native interrupt cannot manufacture a cancellation after CommitClaimed or interrupt cleanup SQL. The disable/drain transition and gate observation must be ordered; cancel after the claim returns tooLate without issuing a fresh interrupt. The persistent gate stays latched. Finalize statements/BLOBs first. No callback/SQLite under a short job gate. Optional native wakeup hooks are a separately reviewed feature addition, not required to make the publication gate correct.

| Observed trace | Decision |
|---|---|
| Before transaction/open refused; no live mutation | NotCommitted; unchanged reservation or no W; ordinary owned-artifact cleanup. |
| Preclaim body/cancel failure, active tx -> explicit ROLLBACK OK -> autocommit=true | NotCommitted. Cancellation is terminal clean only after worker/artifact settlement; cleanup failure is separate. |
| Preclaim SQLite error already ended transaction; no live COMMIT ever issued, complete control trace, autocommit=true | Can establish NotCommitted via automatic rollback. Record that disposition, not a fictional successful explicit rollback. Unreconciled IO/identity/native trace quarantines. |
| COMMIT returns OK and autocommit=true | Committed witness. Later cleanup/panic/response failure cannot erase it. Preserve B. |
| COMMIT returns Busy, still active; explicit ROLLBACK OK, then autocommit=true | Failed/NotCommitted, not a clean accepted cancellation after CommitClaimed. Preserve original Busy and rollback disposition. No retry COMMIT. |
| COMMIT error with active tx and a separately established no-publication trace, explicit rollback OK | Failed/NotCommitted only for the reviewed error category; initial implementation only whitelists the demonstrated Busy case. Other IO/FULL/NOMEM/fatal categories stay conservative Unknown until separately established. |
| Missing/ambiguous COMMIT return, panic across COMMIT, non-Busy commit error not established, rollback failure, contradictory autocommit, mismatched handle/path | Unknown; quarantine/retire W, invalidate bindings, retain B and original errors. Closing/reopening/autocommit=true/full witness equality alone cannot change this to NotCommitted. No automatic retry. |

A rollback error with autocommit=true is not blindly reinterpreted as clean; classify only from the complete trace. An unexpected panic before publication is conservatively quarantined even if closure may ultimately roll back. A witnessed commit survives a later panic. The server job slot remains held through worker and settlement ownership; transaction settlement does not pretend to close the independent lifecycle/artifact scope.

## 8. Required implementation controls (all unexecuted here)

Use the production owner/transaction helper barriers, not a toy bool or model-only gate. Register separate controls with exact fixture/source identities; test counts are not credited by this memo.

- **B01 all mutation routes:** establish Ready stamp, invoke each actual server mutator in the table (including skipped alias/no-op, cleanup no-op/refusal-after-admission, backup delete, legacy duplicate), then require stale before B/live DML. Preserve every legacy HTTP request/response and v1 token baseline. Pure readers must not invent a generation. Startup migration happens before new incarnation, never in job.
- **B02 managed ABA:** same-content restore/reopen changes incarnation **before** copy/close; block at that barrier, attempt another job/legacy writer, and prove serialization/refusal. Reopen failure/restore failure must leave owner quarantined, not old W reusable; successful restore queries use the new shared W. A read-only health/status projection never waits on W to request cancellation.
- **B03 actual cross-process external writes:** parent owns and reaps two explicit processes; Ready on persistent W, child commits alias-only/placement/source byte/report/schema-meta changes, and child commits A->B->A in separate transactions. Apply samples the original **same W** under IMMEDIATE and refuses. Include a negative discriminator that incorrectly samples a newly opened connection (must fail), and one that drops generation on same-W no-op (must fail).
- **B04 reservation barriers:** contender attempts IMMEDIATE before/after W reserve, after witness, after pinning S, after both Done copies, before gate and before COMMIT. It cannot commit until W releases. Terminate exact owned children in finally, capture exit/reap and barrier artifacts; no broad process cleanup. Accepted spike feasibility is not this schema-28 control.
- **B05 full witness discriminators:** independently enumerate expected 46 tables + complete schema relation/index metadata, every table's current columns and hidden rowid; mutate a value outside package/source sets, source aliases, standalone-source bytes with unchanged claimed SHA/len, legacy bytes, unavailable marker, provenance/winner, rowid-only and schema SQL/index/rootpage state. Each changes witness or explicitly refuses. No “all tables covered” assertion based only on table count. Record the 120 stored/121-including-schema convention exactly.
- **B06 typed/chunk boundaries:** NULL vs empty TEXT/BLOB; equal display INT/REAL; signed extrema/f64 bits; embedded NUL/invalid UTF-8 TEXT bytes; 0/65535/65536/65537 and maximum-admitted lengths, chunk short reads/overflow; assert no full String/Vec retrieval at BLOB/TEXT boundaries and exact whole source bytes through independent chunk oracle. Admission failure is complete-or-refused, never a partial digest. No hard native-call/RSS claim.
- **B07 schema admission:** old/missing/future, unknown extra object, stat/sequence table, autoindex/PK changes, WITHOUT ROWID, case-folded shadow aliases (all three or one), virtual/shadow/generated columns, attachments/TEMP/UTF-16 refuse without creating/migrating/converting/writing. If a future codec test uses WITHOUT ROWID/sequence, it must not accidentally widen current job admission.
- **B08 identity/mode:** normal same-handle metadata drift stales; pathname replacement/link/symlink/HAS_MOVED unsupported/error, WAL/recovery/journal artifact/config changes refuse/quarantine. Initial checks and a post-reservation check are independently discriminated. No test result may claim arbitrary hostile rename is supported by stat.
- **B09 W/S/D:** persistent W baseline/full witness bound to actual copied snapshot; no live DML before S closes; two copies from old S not installed D; self-source one-step refusal, incomplete copy never promoted, observed Done/drop before D query. Held S Busy rollback uses the explicit settlement path, not Drop. Faults at each copy/close/rollback seam keep owned resource disposition exact.
- **B10 gate/commit ownership:** deterministic cancel-wins and commit-wins barriers at production gate, including stop/deadline/expiry; kill check-then-COMMIT mutant. Scratch commits/savepoint RELEASE cannot consume live gate. COMMIT Busy, non-Busy/ambiguous return, rollback error, panic with Ignore must quarantine and preserve original failure/B; successful COMMIT plus response/cleanup panic must keep committed evidence.
- **B11 stage reachability:** A alone exposes no job; B alone yields no install/Ready consent; C missing any transitive typed Stop cannot be enabled by a server route. Quiet API/report/name/token parity and no new schema are mandatory before production capability admission. Artifact caps/complete raw-vs-safe preview identity remain separate release prerequisites.
