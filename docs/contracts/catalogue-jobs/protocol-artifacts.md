# Catalogue job protocol, resources, artifacts and production lifetime

Status: **final design candidate; not implemented or independently approved**.
This concretizes the [job contract](../catalogue-update-jobs.md), alongside
[managed binding and transaction ownership](managed-binding.md). The exact
field/record definitions and cross-field policy are adjacent static design
artifacts, not a callable API or an executable validator:

- [Wire schema](wp043-protocol.schema.json)
- [Raw/public record schema](wp043-artifact-records.schema.json)
- [Private artifact inventory schema](wp043-artifact-internal.schema.json)
- [Cross-field resource and lifecycle policy](wp043-protocol-policy.json)

All semantic and `x-*` annotations require explicit implementation checks;
ordinary JSON Schema alone cannot enforce them. The
[design readiness record](design-readiness.md) distinguishes proposal,
independent architecture approval, product runtime evidence and delivery.
No private source-binding paths or machine identifiers form part of this spec.

## P1. Scope and source-backed choices

The first lane is Linux, existing managed file-backed current-schema catalogue, default local VFS, private cache, normal locking, DELETE journaling. Unsupported OS/filesystem/journal/schema/recovery/identity conditions refuse `unsupportedCatalogue`; no journal conversion, new database, migration or alternate-profile fallback inside the job. A filesystem that does not implement required flock, no-replace rename or directory sync is unsupported for this lane. Hostile same-user mutation is excluded; ordinary own-process routes and competing server processes are not excluded.

No generic jobs service, custom SQLite VFS, quota daemon, new importer, durable outcome ledger or new authentication model is introduced. Exact managed database owner/generation/witness integration remains the other worker's contract; this package consumes its healthy reserved W and identity-bound S, never substitutes its own partial fingerprint.

Source facts versus decisions:

1. `apps/knx-server/src/main.rs:108–180` currently closes bus admission, gives requests five seconds, releases the bus and drains device writes; its forced process exit bypasses waiting for blocking work. Catalogue ownership is absent. P9 adds it without relabelling device-write safety.
2. `apps/knx-desktop/src-tauri/src/lib.rs:141–155,190–208,254` currently spawns an untracked server and exits on main-window destruction. `quit.ts:31–34` destroys rather than closes the window. P9 therefore intercepts **process exit**, not just a close-requested callback.
3. Workspace already declares rustix 1.1 with fs; lock pins **1.1.4**. No fs2 is in the lockfile. libc **0.2.189** is transitive, not a server direct dependency. The smallest dependency change is `rustix.workspace = true` in server (or the one artifact-owning existing crate), not a new crate/framework or direct libc/unsafe FFI. Inspected cached rustix `fs/fd.rs:317`, `fs/at.rs:296–313`, Linux backend types expose flock EX|NB and renameat2 NOREPLACE. Archives and exact members match Cargo.lock. std create_new/OpenOptionsExt/DirBuilderExt/File::sync_all and already-direct getrandom/sha2 suffice for the rest.
4. Cached Tauri **2.11.5** `src/app.rs:79–94,220–232,573–579,1357–1373` exposes `RunEvent::ExitRequested`, `prevent_exit`, build-then-run, and `AppHandle::exit`. Restart can bypass this; no controlled restart feature is introduced. The existing exit failure fallback remains crash-like, not clean-drain proof.
5. The parent accepted only the existing five-case synthetic SQLite mechanics spike: reserved W + distinct pinned RO/private S + disk D, observed Done after 91 More, S closed before W DML/COMMIT; held S caused COMMIT Busy with active transaction followed by explicit rollback. These observations do not test page quotas, artifacts, cancellation arbitration, production owner integration or durability. The failed initial child-owner/reap attempt remains a failure. No additional runtime mechanics work was performed here.

## P2. Strict primitives, bytes, counters and authority

All new protocol objects have **all listed keys required**, no additional keys, duplicate keys refused at every nesting level, explicit nulls, closed enum tokens and schemaVersion=1/policyVersion=`wp043-v1`. Reject invalid UTF-8, JSON BOM, nonfinite/numeric coercions, excessive nesting (>8), unknown fields and wrong nullability. New API clients and server ship together: no permissive older-server aliases. Header/query duplicate handling is strict too. Rust Deserialize structs require duplicate-field rejection plus deny_unknown_fields; TS must detect duplicate object members before JSON.parse loses them (small bounded strict tokenizer for these responses, or a reviewed existing strict parser, not JSON.parse plus a cast). This is a local protocol requirement, not permission for a generic application schema framework.

**U64:** ASCII decimal string only, grammar `0|[1-9][0-9]{0,19}`, parsed value <= `18446744073709551615`. No signs, whitespace, decimal/exponent, leading zero, Unicode digit or JSON numeric token. Rust validates grammar then checked u64 parse; all additions, subtraction, multiplication and usize conversion are checked. TS validates length/grammar, then BigInt and exact maximum comparison; never Number/parseInt for revision, serial, counts, offsets, ticks or byte sizes. Small JSON-number fields are exclusively schemaVersion, `limit` (1..200) and `scheme` (integer 0..4294967295); their schema numeric bounds are mandatory. U64 equality/range checks are semantic, not maxLength alone.

IDs are exactly 16 CSPRNG bytes rendered 32 lowercase hex, capability exactly 32 CSPRNG bytes rendered 64 lowercase hex; SHA256 exactly 64 lowercase hex. RNG failure refuses admission, never weakens entropy. `jobId`, `consentId` and `artifactId` are independent nonsecrets, never capabilities. The capability is compared in constant-time, kept only in bounded in-memory owner state, never echoed, in disk metadata, URL, logs, Debug, panic or analytics. Hashing the capability is not required for disk reclamation because no old capability survives restart.

Capability transport is exactly one `X-KNX-Catalogue-Capability` header, on **every job operation including create**. Non-create operations also require `X-KNX-Catalogue-Incarnation` and `X-KNX-Catalogue-Serial`; `{id}` is jobId only. Create incarnation/serial occur in the JSON body, not conflicting header copies. GET admission has neither capability nor allocation. Existing authentication guard runs first, before job lookup or body consumption. This stays the single-operator model; no cookie-derived principal, per-user ACL or logout-cancels-work promise is invented. Desktop/test auth-disabled entry keeps its current local trust boundary. Cookie expiry/logout stops authenticated transport, not workers. No new CORS grant; mutating browser calls remain same-origin; capability is not a substitute for auth or consent.

**Exhaustion:** next admission starts at `1`; MAX is the exhausted sentinel and is never allocated. Allocate only while next<MAX, then checked_add(1), all under existing short admission lock. No wrap, reset, saturation or automatic new incarnation on exhaustion. Revision starts `1`. At revision >= MAX−64, freeze ordinary progress updates, close new admission, and request `counterExhausted` settlement before a new apply/commit claim. Reserve these last 64 increments for settlement only. After this event there are at most eight externally published control snapshots: cancel request (if needed), settling entry, commit-outcome witness (if already claimed), handle-close result, cleanup result, terminal, disposal refusal/failure and a final retained-status update; intermediate cleanup/progress events are coalesced. Poll/replay/remaining-time passage never increments revision. Apply/commit claim is refused when it cannot preserve this reserve. Phase generations are checked and share the same stop reserve. Measured counters abort through typed Stop on checked overflow; they never saturate. A bug that exhausts the reserve closes/poisons the manager rather than emitting a reused revision or false clean terminal. Such a bug is not a supported successful run.

Tick milliseconds are from one incarnation-local monotonic origin, U64 checked; wall clock is not used for admission. `Lease.deadlineTickMs` is fixed for that phase. It is useful for display only after anchoring to GET admission's serverTickMs; HTTP transit is not trusted authority. Tick conversion overflow closes admission and requests settlement. Retention begins at actual terminal settlement, not cancel request or COMMIT.

## P3. Complete field/null/state contract

The schema is the exact field-list source. Status has these keys and no others:

`schemaVersion, policyVersion, handle, revision, source, profile, state, phase, gate, stopCause, progress, lease, apply, preview, result, failure, commit, cleanup, backup`.

- Handle `{serverIncarnation,admissionSerial,jobId}` is immutable and must match the tracked selection exactly.
- Source `{name,declaredBytes,receivedBytes,sha256}`: immutable admitted name/length; server-observed **written** bytes only, not browser sent bytes. 0<=received<=declared<=1e9. SHA null until exact EOF/full declared length, then stable actual SHA. Empty length is syntactically allowed; package refusal later remains normal. Name is 1..512 UTF-8 bytes, basename only; reject `/`, `\`, `:`, NUL, Unicode Cc/Cf controls, `.`/`..`, trailing/leading ASCII whitespace; do not silently basename/rename it.
- Profile `{name,maxMemberBytes,maxExpandedBytes}` exactly standard/1e9/4294967296 or large/1e9/17179869184. No client chosen numeric limits. `allowLargePackage` is true iff large, false iff standard.
- Progress nullable `{stage,pass,unit,completed,total}`: real phase-local observations; total null if unavailable; else completed<=total. Pass is positive and increases at actual rereads, not repeated polling. Unit-stage mapping: receiving/hashing/decoding bytes; archiveScan entries; xmlParsing events; snapshotCopy/backupCopy pages; reporting/cleanup records; binding/rehearsal/install emit their actual bytes/events/records, not a synthetic percentage. No unbounded event history, member-name strings, ETA or global percent.
- Lease always `{kind,deadlineTickMs}`. Receiving one-hour overall deadline plus internal 60-second idle timer; preview working two hours; Ready 15 minutes; accepted apply new two-hour working lease; settling retains/creates the working lease but deadline only requests stop, never frees ownership; terminal retention deadline at most one hour. Polling cannot mutate the fixed snapshot merely to decrement a countdown.
- Apply always `{state,acceptedCommandId}`. Unavailable/available => command null. Accepted => nonnull command. Spent retains nonnull if apply was ever admitted, otherwise null. Status does not return the capability or old v1 token. Preview `{consentId,artifact,packageAlreadyInstalled,wrapper,packages,programs,productsAdded,hardware2ProgramsAdded,idConflicts,growthCliffPossible}` is complete bounded summary plus descriptor; growthCliffPossible always true for this policy. Result `{artifact,cataloguePageBytes,nextJobInputLimitExceeded}` only after witnessed commit; boolean exactly pageBytes>8GiB.
- Failure nullable `{kind,context}`; context always null in v1. Kind from closed policy. No exception string. Fixed English message mapping is in policy, localized by client, not interpolated raw server error data.
- Commit always `{outcome,evidence}`. Pending/notAttempted while unresolved, not a claim that a live transaction has no writes. Final evidence P7 below. Cleanup always `{disposition,failureKind}`: pending/null while handles/cleanup remain; complete/null; quarantined/non-null typed reason. Quarantined is **not clean**. Backup always `{state,name,identity,publication}`: none/staging have name+identity null and publication none; published/retained have safe name+identity and fileAndDirectorySyncAcknowledged; quarantine retains these only if publication was already witnessed, otherwise null/none. It contains no absolute path.

**Every phase and terminal is constrained by policy stateRules**, with these additional semantic requirements:

| State/phase | Exact authority and result rules |
|---|---|
| running/receiving | sha/preview/result/failure null; gate open; apply unavailable; commit pending; backup none; receiving lease. A cancel changes phase to settling atomically, not a receiving snapshot with a half-updated gate. |
| running/previewing | sealed source; preview/result/failure null; gate open; apply unavailable; commit pending; backup none; working lease. |
| running/ready | sealed, complete immutable preview, result/failure null; gate open; apply available; commit pending; backup none; Ready lease. Never expose Ready before the entire raw/public artifact is complete and admitted. |
| running/waitingForWriter | accepted command retained, consent spent; immutable preview; gate open; result/failure null; backup none; new working lease. |
| running/validating | same fields as waiting; reservation/binding/rehearsal may be running, still no public result or backup. |
| running/backingUp | accepted; preview retained; result/failure null; gate open; backup none, staging or complete published; pending commit. |
| running/installing | published backup required; accepted; preview retained; result/failure null; gate open; pending commit. |
| running/committing | accepted; preview retained; backup published; gate commitClaimed; pending commit until actual result; result/failure null. This is never cancellable success. |
| running/settling | apply spent, cleanup pending, progress null or actual cleanup records; source may be unsealed; preview nullable if it existed; result only if commitReturnedOk; failure nullable while outcome settles. Gate may be open (error), cancelAccepted (stop), commitClaimed. Backup current owned state, no source download. First stop/error transitions here without waiting for ProductDB. |
| succeeded | phase/progress/failure null; spent accepted command; sealed+preview+result required; commit committed/commitReturnedOk; backup retained or quarantined; cleanup complete **or explicitly quarantined**. Cleanup warning does not erase success. |
| cancelled | phase/progress/result null; failure cancelled or serverStop; gate cancelAccepted; stopCause userCancel or serverStop; commit notCommitted; cleanup complete; backup none. |
| stale | phase/progress/result null; sealed+preview retained; failure catalogueUpdateStale; gate open; stopCause null; commit notCommitted; cleanup complete; backup none. |
| expired | phase/progress/result null; gate cancelAccepted; failure matches exact deadline stopCause; commit notCommitted; cleanup complete; backup none. Ready expiration retains preview as evidence, never available consent. |
| failed | phase/progress null; failure required; result only if already committed; commit notCommitted/committed/unknown with matching evidence; cleanup complete or quarantined; backup none/retained/quarantined according to P7. Failure is never inferred as notCommitted. |

All terminals have retention lease, no worker/native handle and only the retained artifact inventory or explicit closed-handle quarantine. If worker is still alive or lock ownership is unresolved, state remains running/settling, regardless of a timeout. Any intended cancelled/stale/expired whose cleanup fails instead becomes **failed/notCommitted/cleanup.quarantined** with original stopCause preserved; cannot claim clean cancellation. A cleanup-only failure after witnessed commit may be succeeded/cleanup.quarantined; a panic or report-integrity defect after commit is failed/committed with result evidence retained. No automatic install retry in either case.

When gate=cancelAccepted stopCause is nonnull; otherwise it is null. Failure outside settling/terminal is null. Same revision must have byte-identical compact serialization. Known lower revision may be ignored only after complete strict validation; terminal can never regress. Matching job id alone does not admit a cross-incarnation/serial/source/profile response. Descriptor/public projection and nonnull digest never mutate under the same artifactId.

## P4. Operations, body admission, paging and bounded replay

All new responses use application/json, `Cache-Control: no-store`, no compression in v1 (byte caps are actual UTF-8 response bytes), no capability in logs/response. Authentication is before bounded JSON/body consumption. New JSON create max4096 bytes, control max1024, detail query max512 raw bytes; status/error <=16384. Content-Type exact application/json with optional charset=utf-8; no application/*+json aliases in v1. Header token lengths and allowed characters are validated before allocation. Body overflow stops processing, never truncates to a parseable prefix. No global limit widening or compatibility-route behavior change.

1. GET admission =>200 Admission. Exact fields in schema; zero allocation/reservation. Busy/stopping/exhausted/unavailable offer states are visible without creating a job. create uses prior offered serial; no automatic new-offer retry.
2. POST jobs =>201 Status on fresh create, 200 Status on retained exact replay. Body is exact Create. Compare retained incarnation/serial then capability before metadata (mismatched capability gets generic410, not a conflicting-metadata oracle); exact matching capability+all create metadata precedes capacity; reject conflicting metadata with commandConflict, consumed forgotten serial with410 outcomeUnavailable, future serial409 staleOffer, prior incarnation410 incarnationChanged. Capacity/stop/store failure does not advance serial. Reserve active record before filesystem setup; setup error creates a retained failed record with cleanup accounted, not a consumed invisible operation. One active slot includes Receiving/Ready/Settling, no queue. Full-record/tombstone 8+8 and one-hour upper TTL unchanged. Before admitting a fresh active job when eight terminal records are retained, settle eviction of the oldest terminal first (close new detail readers, wait current bounded reader, dispose exact transient result files, register tombstone). If I/O cleanup is needed, return jobBusy without consuming the offered serial until the owned reaper finishes; never hold the short lock during eviction. This reserves room for the new terminal and never briefly needs nine terminal records. Tombstone insertion evicts oldest tombstone first; no active/unsettled record is evicted. Retained exact replay precedes this capacity procedure.
3. PUT upload =>202 current Status after exact EOF/seal/preview handoff. application/octet-stream only, no multipart, Content-Encoding forbidden except absent/identity, no resume/range/trailer metadata. Content-Length if present must equal declaredBytes before first body read; actual chunked count and EOF are authoritative. Only one upload claim under short lock; second claim409 uploadNotReplayable without consuming its body, even if original reply was lost. Never retry upload automatically. Original request owns only a bounded receive task; a job-owned joinable supervisor owns the task and spool, so disconnect/HTTP grace cannot detach it. On body abort mark uploadAborted; idle/overall timeout marks exact expiry cause. Seal only after all writes plus flush/close readback identity/length/hash witness. Transition and spawn/handoff are tracked atomically; spawn failure settles failed, never a Running job with no owner.
4. GET status =>200 exact Status. Header ownership/short-state snapshot only, no ProductDB/file stat/SQL/backup listing/parser. Copy fixed-size bounded status after lock release; status active serialization max16384 checked at construction as described P5. One status builder at a time per manager, try-acquire/refuse409 jobBusy rather than queued unbounded copies.
5. GET details requires exactly `artifactId,publicSha256,offset,limit` query keys. offset canonical U64; limit canonical ASCII decimal 1..200 converted to bounded integer (no leading zeros); first offset0. No opaque cursor framework is needed. Immutable descriptor binds the digest/job and total. Unknown artifact/digest409 invalidRequest; forgotten job410. Pages serialize <=262144 including envelope, record JSON <=65536, records <=requested limit and200. Precomputed record offsets/lengths choose the maximal prefix fitting bytes; return at least one whenever offset<total, else fail artifact-integrity (not a silent short/empty page). `nextOffset=offset+len(records)` if <total, else null; offset==total permits empty final page; offset>total invalidRequest. All records and exact total cover the whole public projection; page digest is the descriptor's full public NDJSON hash, not hash of this page. Each record has ordinal=global offset+i. Same query gives identical bytes. No skipped records, first-N pseudo-completeness, source data/raw download or arbitrary section filter. Client may request any valid offset; server checked offset arithmetic/index bounds, no authority encoded in offsets.
6. POST apply =>202 Status on accepted fresh exact command, 200 Status for exact already-accepted replay. Body ApplyCommand binds commandId, current consentId, preview artifact id+public digest and confirmed:true; no source/profile/path fields accepted. Existing accepted-command replay checked before phase/expiry/stop so a lost response still reads its own current status, never creates work. Other command after consent consumed409 commandConflict, wrong preview409 catalogueUpdateStale, expired409 consentExpired, invalid phase409 invalidPhase. Existing atomic R1 admission remains unchanged. Even if response fails to serialize/deliver, admitted command stays spent.
7. POST cancel =>200 CancelReply. Body CancelCommand, echo commandId only; no per-request growing cancel ledger. Check terminal first =>alreadyTerminal; live commitClaimed=>tooLate; cancelAccepted=>alreadyRequested; else atomically claim cancellation and enter settling=>requested. Accepted means no later COMMIT claim, not worker ended. Repeating command sees truthful current gate/status; response outcome may advance, not frozen fiction. No cancellation before a known failure rewrites the original cause as a successful clean cancel.
8. DELETE job has no body =>200 DisposeReply only after terminal+worker/cleanup ended, retained artifact readers gone and cleanup complete. Active409 invalidPhase; quarantine503 artifactStoreUnavailable, no deletion amnesty. Exact disposed tombstone with correct capability/handle =>same disposed response. Tombstone after count/TTL forgetting =>410 outcomeUnavailable, never a new operation. Tombstones hold exact bounded owner identity and disposal bit only, no outcome reconstruction. An evicted outcome tombstone (not explicit disposed) cannot pretend DELETE succeeded.

Page reads pin only the artifact inventory while constructing a bounded response, with one try-acquired detail-reader permit per manager. They never pin a SQLite connection. Retention/disposal first close artifact read admission; existing reader finishes bounded construction or errors, then file cleanup. No file is unlinked under a live reader; an unfinished cleanup is not an evictable record. TTL/count eviction is allowed only after its cleanup attempt completes or closed-handle quarantine is registered. Failure closes new job admission rather than evicting a leaking record. Slow network response holds only bounded serialized bytes, not files/worker lease; no background streaming FD beyond page construction. Status/detail overload refusal never changes the job.

Errors have exactly `{schemaVersion:1,kind,context:null}`; policy maps every kind to fixed text/status. Existing auth middleware's current401 contract is preserved before this envelope. Capability mismatch/unknown well-formed handle returns the same generic410 outcomeUnavailable, without revealing existence or counts; malformed headers400 invalidHandle. Wrong incarnation410 incarnationChanged reveals only advertised incarnation change, not old outcomes. I/O/SQL/panic/path/member exceptions never escape through a string. Details show admitted raw archive facts only through P6, not error context. No Retry-After or retryable flag authorizes install retry.

## P5. Named resource pipeline — admission before owned materialization

The candidate caps remain unchanged: raw1e9, per-member1e9, expansion4/16GiB, input8GiB, main-growth64GiB, retained artifacts64MiB and100000 logical records, status16KiB, page256KiB/200 records, record64KiB. Additional bounds here are **local control/transport bounds**, not newly widened parser budgets. No aggregate disk/RSS, successful disk-full recovery, native-call duration or hard cancellation latency guarantee is made.

| Boundary | Required admission BEFORE allocation/copy | Failure/disclosure |
|---|---|---|
| HTTP create/control | guard; header grammar; Content-Length when available; bounded incremental JSON into at most4096/1024-byte buffer; duplicate/depth checks before String/model creation | bodyLimit/invalidRequest, no source spool or worker |
| upload | metadata already admitted; one fixed65536-byte owned I/O buffer; for each borrowed body frame checked total, consume/write slices <=65536; no collect/to_vec or spawned-write queue | A framework can allocate its incoming frame before application sees it: no hard hyper/network RSS/frame bound claimed. Oversize/short EOF refuses, never seals prefix. At most one receive task and one outstanding write. |
| source Vec for existing importer | sealed length<=1e9; checked usize/try_reserve_exact for one whole raw Vec; read in65536 chunks, hash all actual consumed bytes and compare sealed witness before parse | **First implementation still has a whole bounded raw Vec**, not streaming parser/RSS improvement. No redundant upload-body Vec or simultaneous raw duplicate by orchestration. Source spool is a disk copy in addition. |
| ZIP/member | existing metadata/path/count/declared/actual member+shared expansion checks before reserve/decode growth; actual output charge before extending buffer; same two ZIP views and parser pipeline | Member buffers can reach1e9 each. Sanitized ZIP overlay/indexes, wrapper inner bytes, whole XML token/value normalization and native reader allocations remain additional live allocations. Existing format guards remain, but no aggregate sum/RSS guarantee. |
| source catalogue | under reserved W and pinned S real snapshot, read scalar page_count/page_size; validate positive supported power-of-two size, checked multiplication<=8589934592 before destination creation/backup | No catalogue Vec, no sampling. Stat/free-space check only advisory. Disallow changing page size during the operation. |
| disk copy | fresh D, matching source page size, snapshot N known and pinned; positive128-page backup steps/checkpoints; finite cancel-aware30s total retry, <=50ms busy calls | Copy limit is **N times fixed pageSize**, from the pinned source and equal-page copy algorithm, not a destination PRAGMA alone. See below. |
| D growth | after observed Done and backup Drop, query/readback D page_size/count, reassert+readback max_page_count=floor(64GiB/pageSize), count<=cap; then shared install only | No ATTACH/growth of uncontrolled databases. Verify no reachable VACUUM, page-size/journal changes, backup-into-D or pragma that resets the cap during install. Any unexpected shape/mode refuses. |
| live W growth | require W synchronous already FULL or EXTRA **before** BEGIN; revalidate under managed reservation, save prior scoped max_page_count, set/readback same64GiB page cap on **actual W** before job DML | Rehearsal admission alone cannot cap W. Do not change synchronous inside an active transaction (SQLite forbids it); refuse lower/unknown settings instead. Retain main cap through commit/rollback; checked restoration afterward. Restoration uncertainty retires/quarantines managed writer; committed witness survives. This is job-scoped, not a new global catalogue quota for old callers. |
| SQL metadata/report rows | scalar count/length preflight in same snapshot; SELECT length(CAST(text AS BLOB)) for byte length (not text codepoints), length(blob) for BLOB; then row.get_ref borrowed view and exact length/field/type checks before to_owned | No query_map(...String...).collect then count. SQLite may materialize/decode a row internally before Rust sees ValueRef: **named Rust collection cap, not native SQL RSS bound**. Never fetch source/package BLOB for witness/report when scalar/streaming hash suffices. |
| old package source bytes needed for history | length(blob)<=1e9 and applicable format/member/source rules before row.get<Vec<u8>>/clone; one at a time; cancellation on reread | Existing/historical BLOB may itself be a whole large Vec; refusal if over raw policy, not hidden unavailable-history fallback. SQLITE_TRANSIENT parameter binding can make another SQLite-owned copy of raw/member bytes. |
| every report/program/version/conflict/unknown/diagnostic | charge raw UTF-8 byte sizes, exact compact JSON escaped lengths, logical-record count and projected copy/index bytes before String clone/map insert/Vec push/sort; use borrowed slice/token scans then counting writer; bounded try_reserve | All nested collections flattened into schema rows. UnknownCollector must gain controlled charging before its key/sample clones and checked occurrences; old quiet callers preserve behavior. Existing old installed versions/historical reports are included. No giant old-report Vec allocated first. |
| artifacts | combined reserved count+raw+public+index+manifest bytes <=64MiB before append/new allocation; per raw and public serialized row<=64KiB; logical rows<=100000 | Persistent counting sink checks before write/growth; reserve prospective public/index overhead too, not raw-only. Oversize is typed resultLimit/recordLimit and rolls back, never truncate/drop detail. |
| status | enum/fixed numeric types and bounded source name/descriptors; counting serialization before bounded buffer growth; cap16384, max two descriptors | Progress coalesces; failure no free text; public source name bounded512. Do not serialize unbounded core structs and check afterward. |
| detail page | index lengths first choose byte-fitting prefix; count bounded envelope and <=200 rows; reserve <=262144, read only chosen bounded slices | No full-artifact read/JSON parse into memory; one <=65536 record scratch buffer, one bounded page buffer, one detail builder; no cumulative client-side full artifact required for UI. |

**max_page_count / backup trap, closed:** cached pinned SQLite `sqlite3.c:84945–84995,85074–85128,85152–85258` copies direct pager pages, adjusts destination settings, truncates/commits directly. `62914–62919` can raise a pager maximum to allow an existing larger file to be read; ordinary page-growth paths enforce limits elsewhere. Therefore this contract expressly **does not claim max_page_count is a standalone cap on online backup or survives copy/reopen**. A post-copy observation alone would be too late as a bound. Instead initial copies are to fresh matching-page-size D from a pinned snapshot with checked N<=8GiB/pageSize; the direct page loop writes only those source pages and final truncation is N pages in this equal-page lane. Before copying destination has no data larger than N; unexpected source/destination page-size/count disagreement aborts and removes owned partial. No physical source modification/restart while copy runs is allowed. After Done/Drop validate N/page size, reassert and read back64GiB cap on the connection that will actually perform SQL allocation. Repeat on W before live DML. Disk/main-page extent is bounded in this lane by algorithm plus SQL cap; journals, native temp, filesystem allocation blocks/sparse behavior remain outside it. Future contrary source/runtime evidence must revise this assertion before dependent implementation, never call a PRAGMA sufficient by assertion.

**Artifact budget and repeat apply:** one job budget covers all retained Ready preview raw/public NDJSON, result raw/public NDJSON, bounded manifests and 16-byte-per-logical-row offsets (two u64 offsets/length metadata pairs may instead use32 bytes; charge the actual chosen32 bytes). Freeze the index representation as32 bytes/record: rawOffset/rawLength/publicOffset/publicLength, little-endian u64, all checked. Count one **logical raw record** once, not its public mirror twice; byte-charge both mirrors. Ownership inventory itself is a separately bounded16KiB control file; the result budget includes both complete artifact manifests but not source/rehearsal/recovery files. This is not an aggregate job-directory quota. Preview remains through terminal retention. Repeated apply rehearsal produces the same preview through a counting+digest sink and compares it without retaining a second preview; result artifact is completely admitted/sealed **before** live commit. Live install streams its evidence into the same canonical counting/hash comparator, not a second retained report. It must agree exactly with the admitted result/raw ordering. Any result/record/string overflow during live install still explicitly rolls back; no success with an omitted report. No unbounded sort: bounded old/new record vectors or existing index order only after their full element+string charges. Source equality checks do not create an extra raw Vec.

The64MiB policy is **named complete-result storage/materialization admission**, not all parser structures, source buffers, SQLite native cache, the1e9 raw source, snapshot, backup or journal disk. One active job and8 retained artifacts imply bounded named retained collections, not total filesystem usage. Quarantine closes admission rather than turning leaked bytes into an uncharged new allowance. User-managed published backups are deliberately not capped by job retention.

## P6. Complete internal raw versus complete redacted public artifacts

Artifacts are immutable compact UTF-8 NDJSON, one raw logical row and exactly one public row per ordinal; LF after each row included in digest/bytes. Deterministic field order is schema property order; UTF-8 non-ASCII emitted directly, JSON quotation/backslash/controls use standard escapes, control escapes lowercase `\\u00xx`, `/` not escaped, no whitespace outside strings. Numeric counts are canonical decimal strings. Raw schema preserves authoritative existing values; it is internal only, never routed. publicSha256/bytes refer **only to the public file**, never the raw hash (no raw-digest oracle for withheld aliases). ArtifactId random, no capability/name-derived filename. Preview consent binds public id+digest **and internal complete raw semantics/witness** through separately owned binding; redaction cannot weaken publication validation.

All record keys/types/nulls are frozen in the record schema. Records are flat so there is no massive installedVersions/member/program array hidden inside64KiB. Complete coverage:

- previewSummary once; program in existing preview order, installedVersion children keyed by programKey; wrapperPackage in original archive order; wrapperOtherMember in existing order;
- resultSummary once; packageReport per installed package in archive order; member rows per original report, installCount rows in existing category/disposition order; unknownConstruct and diagnostic rows in existing deterministic order; **idConflict rows retained and publicly projected**, not collapsed to only a count;
- provenance for **every InstallReport.source_names alias**, in existing sorted order, linked by packageKey and sourceKey; historyUnavailable explicitly marks missing historical report facts by packageKey/evidenceKind (installFacts or baggage) and reason (notRecorded or unavailable), not fabricated zeros. packageReport also includes exact baggage available/unavailable state; baggageSummary once/package, baggageDeclaration per existing index/document order, baggagePayload in existing member-path order. These carry the entire InstallReport.baggage, not merely the narrower old web DTO. packageReport facts unavailable =>unknownOccurrences null and no invented child evidence; facts available =>measured counts (including zero) and exact rows. No hash/order/public identifier is invented for unvisited history.

BaggageSummary has exact nullable measured declaration/payload counts: unavailable =>both null and no child rows, available =>exact counts including zero. baggageDeclaration retains indexPath, declarationOrdinal, all raw declaration lexemes (id/name/targetPath/installOnImport/timeInfo/fileVersion), resolution, nullable memberPath/detail. Resolved iff memberPath nonnull; missing/invalid =>null. baggagePayload retains memberPath/hash/size, exact mediaClass, declarations and NestedArchive state. Nested read =>all four counts present (entries/declaredExpandedSize/encryptedEntries/archiveNamedEntries); notArchive/unreadable =>all four null, never invented zeros. No payload execution or extraction is added. All baggage collection/map/sort/string sites are charged before allocation too.

programKey/packageKey/sourceKey are artifact-local canonical U64 indices, not durable catalogue row IDs or write authority; ordinal starts0 and increments one. Parent/reference existence and single-owner membership, record totals and matching counts are checked as semantic admission. A known zero count is `"0"`, unavailable is null plus its explicit state, never empty pretending measured.

**Public projection is deliberately deterministic, not heuristic host-path detection:**

| Raw field | Public decision |
|---|---|
| historical provenance.alias | Always null; one withheld entry if raw alias is present (including empty alias). Never basename/replace it and claim raw equality. Raw retained internally unchanged. |
| program.name | Always null; if raw nonnull, withheld field name with exact raw UTF-8 bytes. Program id/manufacturer/number/version and status remain visible for consent. |
| unknownConstruct.sample | Always null; if raw nonnull record withheld sample. No raw payload attribute values. |
| diagnostic.detail | Always null; record withheld detail. Public diagnosticKind, archivePath, xmlPath, occurrences retain structured meaning. |
| baggageDeclaration declarationId/name/targetPath/installOnImport/timeInfo/fileVersion/detail | All seven public fields always null; a withheld entry for each raw nonnull lexeme, even empty. Raw lexemes retained unchanged. Structured indexPath/declarationOrdinal/resolution/memberPath remain visible; no untrusted TargetPath mistaken for a host-safe path. baggageSummary/payload/nested measured facts remain exact. |
| program/installedVersion IDs, manufacturerId, applicationNumber/version | Must satisfy <=512-byte ASCII safe-id grammar `[A-Za-z0-9_][A-Za-z0-9_.-]*`; optional raw null remains null. If required identity cannot meet it, refuse publicProjectionRefused before publication, do not replace by a lossy hash or hide consent identity. |
| wrapper/member/diagnostic archive paths | Exact validated archive-relative path <=8192 UTF-8 bytes, `/` separators, no empty/`.`/`..` component, leading slash, drive colon, backslash, Cc/Cf; trailing slash only if original row explicitly denotes directory. Refuse before publication when unsafe/overlong; do not transform raw path to a host basename. |
| unknown xpath, diagnostic xmlPath, unknown name | Structural XML path/name, <=8192 bytes/path (including any generated selector) and512/name, no arbitrary predicates/attribute values/controls; names must satisfy existing XML-name parser then public ASCII safe XML-name grammar `[A-Za-z_][A-Za-z0-9_.:-]*`; path `/` or slash-separated structural names. The sole typed generated-selector exception for declaration diagnostics is specified below. Unprojectable required structured fact refuses, not withheld wholesale. |
| idConflict.table/id | Table must be an actual admitted current-schema table identifier, <=512 safe-id; id <=512 safe-id. Refuse if required conflict identity cannot be projected. Kept/other hashes+occurrence remain exact. |
| member.role | Exact core tokens `Master`, `Signature`, `Catalog`, `Hardware`, `ApplicationProgram`, `Baggages`, `Baggage`, `MasterData`, `Unrecognized`, from package.rs:1491–1503 and ingest.rs:18–26. `Signature` means stored **not verified**; UI must label that explicitly, not imply cryptographic verification. Do not use legacy display_role's expanded presentation string as raw transport role. Unknown stored role refuses unsupportedCatalogue, never passes arbitrary SQL text. |
| scalar counts, booleans, scheme, validated SHA | Exact typed values, never redacted. Newly admitted source basename is the one intentional caller-chosen display name, not a historical alias. |

**Generated master declaration identity (Alpha16 source correction):** the
producer `master_provenance::diagnostics` appends `[@master-ordinal=N]` to the
parser-observed structural path. `parse::master` numbers the five declaration
elements (`DatapointType`, `DatapointSubtype`, `FunctionType`, `FunctionPoint`,
`SpaceUsage`) in encounter order, starting at 1, capped by
`MAX_MASTER_DECLARATIONS=1000000`. This is KNXBench's generated internal identity,
not a statement about external XML/XPath semantics or an attribute read from XML.
The package validator binds it to the retained source's declaration and kind
before the adapter forwards it unchanged.

Choose **tightly typed generated-selector support**, not stripping the selector
or adding a second identity field: existing raw evidence already contains the
complete address and requires no persisted/schema-shape migration. Both public
writer and client must require exactly one terminal `[@master-ordinal=N]` for
`master-identity-collision` and `malformed-master-declaration`, with canonical
ASCII decimal N in 1..1000000, one of the five exact terminal element names and
`occurrences="1"`. Every preceding path component retains the strict structural
name/byte rules. No assumption about parent hierarchy is added: the parser
records actual ancestry, including malformed/orphan placements. Other diagnostic
kinds and unknown paths/names continue to forbid all selectors. Reject missing,
zero, signed, leading-zero, overflow/out-of-range, Unicode/whitespace/control,
quoted, repeated or nonterminal selectors and arbitrary predicates. Never
evaluate this string as XPath. `masterDeclarationIdentity` in policy and the
public diagnostic schema annotation identify these semantic checks; ordinary
schema validation alone is insufficient.

The raw path and raw detail remain byte-for-byte unchanged. Public path retains
the same declaration identity; detail remains null with exact withheld byte
metadata. `packageKey + archivePath + xmlPath` disambiguates declarations within
an artifact, not a new cross-source durable ID or write authority. Native
package/adapter/writer assertions and the faithful intercepted browser fixtures
are reserved for final-candidate execution; this correction is source-traced,
not a claimed runtime reproduction or release acceptance.

Each PublicRecord has `withheld` array (max seven for baggageDeclaration, one for the four single-field variants; zero otherwise), exact `{field,reason:"untrustedFreeText",rawUtf8Bytes}` entries in schema field order. Raw null => public null with **no** withheld entry; raw present-but-empty =>withheld with bytes0. No duplicate field entry. `withheldFields=sum(entries)`, `withheldRecords=count(rows with entries)`, `withheldOccurrences=sum(record occurrences or1 when no occurrence counter)` over rows with withholding (count each row once, not once/field). These exact descriptor counts disclose redaction; all counts themselves are checked U64. This projection emits every logical row; public completeness means complete **redacted projection**, not raw byte completeness or zero omissions. The UI labels withheld details/provenance before consent and on result. If structured identity/status/conflict/growth facts needed for consent cannot be shown, refuse the operation; names/free-text examples are not required consent facts in this v1 policy.

No raw JSON/debug/SQL/hostname alias is embedded in error/status. No free-form internal artifact path/name leak. Evidence fields are admitted before cloning/projecting; the public mirror is never serialized from the core report with a denylist afterward. Unknown-data source bytes remain preserved by the normal importer, not through a new downloadable artifact route.

## P7. Commit, rollback, settlement and failure evidence

Pending is a deliberate **intermediate-only** outcome. Final outcome/evidence allowed pairs:

- committed/commitReturnedOk: the only live COMMIT returned success; this witness is stored in the short owner state immediately, independently of result delivery, notification, handler lifetime, later configuration restoration and cleanup. Scratch COMMIT, SAVEPOINT RELEASE, commit gate claim, autocommit=true, backup Done and successful HTTP response are not this witness.
- notCommitted/noLiveMutation: complete owned trace says no live DML/COMMIT was attempted; any unchanged IMMEDIATE reservation explicitly rolled back/confirmed ended. Initial receive/preview/refusal can use this after worker ownership settles.
- notCommitted/explicitRollback: no successful COMMIT witness, controlled nonambiguous failure while explicit transaction active, statements reset/finalized, explicit ROLLBACK returned success and autocommit true. COMMIT Busy with active transaction followed by successful rollback is in this pair. There is no generic retry of COMMIT after claim in this v1 design: attempt once, then classify/unwind.
- notCommitted/automaticRollbackBeforeCommit: no COMMIT invocation yet; a known error ended the explicit transaction and autocommit true, complete trace rules out a nested publication. Keep original failure and automatic-rollback evidence. Autocommit alone after an attempted ambiguous COMMIT cannot establish this.
- unknown/ambiguousCommit: COMMIT result missing/ambiguous or COMMIT I/O outcome not safely classified by supported error path; do not infer rollback from reopened bytes/autocommit.
- unknown/unresolvedRollback: explicit rollback/transaction state remains unresolved after precommit error. Conservative even if no success was observed.
- unknown/workerLost: worker panic/end while live write/commit trace cannot prove an earlier pair; preserve any already witnessed committed pair instead of overwriting it.

Cleanup disables operation cancellation/progress interruption issuance, joins interrupt issuer if any, finalizes statements/cursors/BLOB/backup handles, ends/closes S, performs explicit W unwind and scoped setting restoration, then closes scratch SQLite/files. Neither a dropped transaction's rollback attempt nor Backup Drop is inspected success. If a connection cannot be cleanly handed back, separately owned managed owner is poisoned/retired and all writes fail closed; replacement is not silently reopened. Backup for committed/unknown stays recovery data. No new jobs after unknown until explicit managed-owner recovery/review, not just artifact cleanup.

Outcome/cleanup matrix:

| Observed outcome | Transient source/rehearsal | Recovery backup | Terminal/admission |
|---|---|---|---|
| no live writes or proved rollback + cleanup succeeded | remove exact owned handles/files/sidecars; keep admitted report artifacts until retention where present | remove exact still-owned staging/published backup, sync parent if published | intended cancelled/stale/expired/failed with notCommitted; slot releases only after join+settlement |
| notCommitted + cleanup failed | closed-handle ownership inventory kept/quarantined | never delete mismatched/replaced target | failed/notCommitted/quarantined; close new job admission; no false clean cancel |
| COMMIT OK + ordinary cleanup succeeded | remove transient, retain complete preview/result artifact | retained user-managed listed backup | succeeded/committed/complete; no reapply |
| COMMIT OK + cleanup/config/panic failure | preserve complete result witness/artifact; quarantine closed leftover | retained or explicit quarantined ownership if publication identity no longer verifiable | succeeded+cleanup warning for cleanup-only; failed/committed for integrity/panic; no retry, no rollback claim |
| unresolved commit/rollback | close/retire managed connection only when worker actually ended; quarantine transient for investigation | retain complete published recovery backup, never reclaim on TTL | failed/unknown only after worker ownership ended; otherwise running/settling. Admission remains closed. |
| forced exit while worker alive | no cleanup proof | published copy remains; incomplete files remain hidden | **No terminal invented**; client outcome unavailable/possibly committed after restart. |

The resource slot remains held while workers/receive tasks/readers/cleanup are live. Closed-handle quarantine can release the *worker* slot, but an owner health latch blocks fresh admission; cannot cycle arbitrarily many failed workspaces under one active-slot label. Retained terminal-ring eviction never forgets a quarantine latch or removes published recovery backup.

## P8. Exact ArtifactDir lease, allocation, publication and reclamation

### Root, lease and inventory

Private root is the fixed `.knxbench-catalogue-jobs` child of the managed catalogue's canonical parent, on the same filesystem as final recovery backups. This root is not caller chosen and is not derived from sourceName or capability. Root creation is mkdir with mode0700 atomically, no exists-then-create; EEXIST path is validated with no-follow metadata as actual directory, owned uid, no group/other access, no symlink/alias surprise. Existing root with wrong type/ownership/mode fails closed instead of chmodding arbitrary data. Hostile same-user filesystem modification is not defended against, but same-process API writes are fenced below.

Inside root, stable `lease` regular file is atomically create_new mode0600/O_NOFOLLOW. On EEXIST open O_NOFOLLOW and verify owner, mode, nlink=1 and descriptor/path inode equality. Acquire `rustix::fs::flock(fd, NonBlockingLockExclusive)`. Lock refusal means job admission unavailable, not permission to sweep. Retain the **same open file descriptor** until all workers/cleanup have ended or process exits; never unlink/replace lease, unlock for a short API pause, or use PID/mtime as ownership. No lock-file framework or fs2 dependency. A second compliant process using the same canonical catalogue parent/root cannot reclaim another process's workspace. This is the root lease, not a substitute for SQLite writer locking or the separately owned catalogue binding. Different roots/hardlink aliases remain refused by managed identity policy; a flock on one pathname cannot serialize arbitrary aliases.

Create per-incarnation directory `i-<32hex>` by atomic mkdir0700 (bounded eight random-name collisions then ioFailure). Each job directory `j-<jobId>` same. Fixed internal filenames: `source.part`, `source.sealed`, `rehearsal.sqlite`, `recovery.part.sqlite`, `preview.raw.part`, `preview.public.part`, `preview.index.part`, `result.raw.part`, `result.public.part`, `result.index.part`, `inventory.json` and complete manifest files. SQLite exact sidecars only `-journal`, `-wal`, `-shm` for its owned basename (WAL unexpected in lane, still quarantine/cleanup-owned, never source sidecars). Every **application-created primary/artifact file** uses create_new0600, no caller path; directories opened no-follow and identified. Record device/inode, nlink and current state from the returned application descriptor **before** handing the primary to SQLite or writing application data. This rule does not pretend that the application owns SQLite's internal journal descriptor before native I/O; the exact native-sidecar exception below governs it. SQLite opens an already-created empty owned file READ_WRITE **without CREATE**, private cache; identity before/after. Do not ask SQLite to allocate a merely guessed nonexistent target. In this nonhostile private directory the path reopen is acceptable; ordinary API clients cannot mutate it.

**Native sidecar exception (private D/B only):** default SQLite creates and
writes its rollback journal internally; no custom VFS, native journal-FD
extraction, precreated guessed journal or claimed application create-new is
introduced. Before each native-use interval verify the owned basename and all
exact sidecar paths with no-follow metadata. Initially all sidecars must be
absent. A surviving sidecar from an earlier **settled and inventoried** native
interval may reenter only after its identity is revalidated as still owned and
its native recovery/clean-state disposition is admitted; uncertainty refuses.
No unexpected preexisting journal/WAL/SHM is silently handed to SQLite. The
private 0700 directory is exclusively leased and fenced from same-process API
writes. Hostile same-user filesystem racing is outside this support boundary;
absence checking is not misrepresented as atomic exclusion against that threat.

While the connection/backup/statement may use its sidecars, do not adopt by a
racing snapshot, unlink, rename or expose them. SQLite alone creates/writes/
closes its journal. After that database's native handles are finalized/closed,
reconcile exactly its `-journal` path: absence is a recorded disposition;
a surviving file must be opened no-follow and checked regular, owned uid,
mode0600, nlink=1, and matching path/descriptor device+inode with bounded size
admission before adding the actual post-native identity to inventory and syncing
it. Do not infer the journal inode from the database inode or invent a returned
native descriptor. Unexpected names, WAL/SHM in the DELETE lane, wrong metadata,
missing/unreconciled expected inventory or identity disagreement quarantine the
workspace and close new admission. Preexisting W/source sidecars are never
owned, adopted or deleted by this exception. Source recovery uncertainty remains
an unsupported-catalogue boundary, not artifact cleanup authority.

Once adopted, a closed private journal can be disposed only via exact inventory
identity and sync rules; a missing journal after normal native deletion is
recorded absent, not an ownership error to repair by recreation. A surviving
active/hot or unreconciled journal prevents backup publication and automatic
reuse, even when its pathname is known. At restart, a native sidecar whose
identity was never successfully adopted and durably inventoried is **unknown**:
quarantine, do not newly adopt it by suffix or replay it to infer an old outcome.
Tests must cover ordinary native create/delete, closed-journal adoption and
exact disposal, unexpected preexisting journal, changed inode/symlink/link/mode,
WAL/SHM, crash before adoption and attempted source-sidecar cleanup. No such
native-sidecar ownership test has executed for this contract yet.

Inventory is bounded <=16384 bytes and uses exact known file basenames/identity/role/state; artifact count fixed by this list, no arbitrary file collection. Do not put capability or raw aliases in inventory. The two inventory slots themselves are fixed `inventory.json` and `inventory.next`, validated by owned descriptor and never recursively listed in files[]. Write a fresh create_new0600 inventory.next and sync file, replace only the prior inventory whose exact identity is still owned, then sync job directory. Ordinary rename may replace **only this explicitly owned inventory slot**, never a recovery/user file; directory lease+known identity distinguish it from safe-no-clobber publication. Missing/crashed inventory is conservative: known empty orphan dir may be removed; any unknown/partial set is quarantined, not swept by filename prefix. Do not claim this is a durable job-outcome journal; no consent/result resume.

### Seal, artifacts and recovery publication

Spool source remains private. On exact EOF and hash/length/identity validation, flush/sync then close writer and no-replace rename source.part->source.sealed using rustix renameat_with(NOREPLACE); no replacement/resume. All future consumers readonly; metadata source hash is recomputed over actual bytes consumed. Sealing is application immutability under owned root, not an OS immutable-file flag.

Raw/public/index artifacts are written with per-write budget admission, close/flush/sync, validate totals and streaming digest, then no-replace rename each part to its complete basename. Complete **manifest** is published last by no-replace rename only after all files verified and synced; sync directory. Status descriptor never points at parts or a partial manifest. Multi-file rename is not claimed atomic; the manifest-last rule is the visibility boundary. All fallible complete preview/result construction/admission occurs before exposing Ready/claiming COMMIT. Cleanup after commit cannot erase admitted artifact identity; retention disposal is separate.

Recovery copy follows canonical W/S sequence: both rehearsal and recovery derive from pre-DML pinned S. Observe Done, drop Backup before D queries and close/reopen the destination readonly. Validate its separate CopyContentWitness/1 against pre-DML S plus the recorded fresh-empty destination-cookie 0 -> 1 rule in [managed binding section 4](managed-binding.md#copy-validation-is-distinct-from-live-readyapply-equality); quick_check is supplemental. The full same-W CatalogueWitness/1 still includes exact schema_version for Ready/apply; do not compare its cookie unchanged to a copied destination or write a cookie to obtain equality. Close S before W DML. Recovery is staged under private root on same filesystem. Sync completed DB file, explicitly close DB and check no required active sidecar remains; an incomplete/active-journal file is never published as a backup.

Final recovery name uses the **existing** parser grammar `<database-basename>.backup-<unix-seconds>-<first12-upload-sha>[-<positive-attempt>].sqlite`. Seconds is wall clock only for display/sort (clock failure uses0 consistent with old naming); try attempt0..999, atomically `renameat_with(root-job-fd,"recovery.part.sqlite",catalogue-parent-fd,candidate,NOREPLACE)`, retry only EEXIST with next name. No exists check, no POSIX rename overwrite, no tempfile.persist overwrite, no cross-filesystem copy fallback. ENOSYS/EINVAL/EXDEV/other unsupported filesystem fails **before live DML**, leaves/quarantines owned staging. A malicious user replacement remains outside support; owned-directory path identity is revalidated at publication and deletion.

After successful no-replace rename, save final identity from actual target, sync target file (if not already closed/synced) and **sync catalogue parent and source job directory**. Directory sync failures prevent live install; retain/quarantine the exact target if deletion fails. Only after these acknowledgements backup.state=published and install may begin. The existing backup list accepts only final names; parts are hidden in excluded root. TTL/disposal/restart cleanup **never deletes final published backups** except the running job's narrowly proved-notCommitted rollback path with its exact current target identity. Once committed/unknown they are user-managed and existing ask-first restore/identity-bound delete remain authoritative. Avoid changing old name/parser/wire contracts.

**Durability decision:** volatile job/consent/status/replay are explicitly non-crash-durable. Complete artifact/recovery publication requires OS-acknowledged file+directory sync in this Linux lane; this is stronger than merely calling a rename and does not claim power-loss survival on every storage/filesystem. Existing update contract promises a complete before-state backup, not a durable cross-restart job result. Thus there is no need to silently widen that contract into a durable outcome ledger, nor to weaken its complete-backup promise. Raw backup Done alone cannot satisfy publication. If platform sync is unsupported, refuse; no non-durable fallback after consent. Existing synchronous backup routine's exists-then-open/run_to_completion is **not reused as the new primitive** and is not retroactively certified safe.

### Cleanup, restart reclamation and quarantine

Normal removal happens only after native/file handles and readers ended, against the exact owned inventory and no-follow file identity. Close errors remain errors; only exact regular files and exact known sidecars may unlink. Replaced inode/type/link-count surprise =>do not delete it; quarantine root health and close new admission. Delete known child files individually then empty directories; no recursive remove_dir_all over an unverified tree, no glob/prefix/mtime sweep, no following symlinks. Published backup uses existing strong identity (device/inode/length/mtime/ctime-derived opaque identity); content/hash isn't substituted for ownership. After unlink sync relevant directory before claiming the disposal acknowledgement. A removal failure keeps closed-handle inventory/quarantine and failureKind; it does not free named charges silently.

At restart acquire stable-root kernel lease first, create **new** incarnation, fence all private roots before listener, then enumerate only exact owned `i-<hex>` directories under that root. Decode bounded inventories without following links. Prior **known transient files only**, verified regular ownership and no unexpected entries, may be removed with exact sidecars. Completed final recovery backups already live outside the volatile tree and cannot be part of this sweep. If a prior copy has no complete valid inventory, unknown entry, retained-recovery designation, changed identity or failed cleanup, leave it in place, mark root quarantined and disable fresh jobs until explicit operator repair. Do not relocate arbitrary data with clobbering rename or infer abandonment from PID not alive. The exclusive stable-root lease is the compliant multi-process exclusion proof. No automatic resume/reapply, no persisted consent decode. Reclamation establishes only artifact cleanup, never old notCommitted outcome.

### Real API/static exclusion

Store canonical forbidden root and its literal reserved location centrally, independent of configurable data_dir; keep them available even when catalogue job admission is disabled. Extend actual common path resolver `paths.rs` for **relative reads, absolute reads, relative writes and absolute writes**, using canonical existing target or canonical existing parent+basename. Reject exact root/descendant and symlink aliases before returning a path. Do not retain the current absolute-read/write branch's best-effort `if let` as a fail-open on missing root/canonicalization failure. On failed canonicalization within a potentially reserved path, refuse. Directory listing omits reserved entry and aliases into it, but hiding listings alone is insufficient. FS upload create directories/destination, project import/open/save-as, CSV/export and any bypass path all use the same fence; audit direct fs::read/write route callsites before enabling jobs.

At startup `static_dir` canonicalization must refuse any root overlapping/containing/inside private root; no `KNX_STATIC_DIR=<data root>` exception. In addition replace naked ServeDir fallback with a containment guard: percent-decoded normalized request path, canonical actual candidate, refuse if it escapes static root or resolves into reserved root; forbid symlink traversal for static entries so descendants cannot introduce a protected-root alias after startup. No body/HTTP error reveals the forbidden path. This is necessary because source `lib.rs:260–263` currently serves a caller-supplied root without such checks. Verify exclusion before making worker routes reachable in either standalone or embedded router. User-managed final backup access stays through existing name-only list/confirmed restore/identity-delete, not a new job artifact download.

## P9. Concrete ownership, standalone stop and desktop/headless drain

**Owner API (Rust declarations are design signatures, not existing callable code):**

```rust
impl CatalogueUpdateJobs {
    fn close_admission_and_request_stop(&self); // sync, idempotent, short control lock only
    async fn drain(&self, limit: Duration,
                   second_signal: impl Future<Output = ()>) -> CatalogueDrain;
}
```

`CatalogueDrain` is the exact schema object; internal Rust enum/struct can encode it strongly. `nothingRunning` =>no active receive/worker/cleanup, workOutstanding=false; finished=>all joined, workOutstanding=false; timedOut/secondSignal=>truthfully sampled outstanding work, normally true, never a cancellation witness. cleanup quarantined remains distinct from work outstanding; a poisoned but handle-free owner can be finished with quarantined cleanup. `commit` summarizes the single active observed job (notApplicable if none); pending cannot be converted to notCommitted by timeout. Timeout callers retain owner and may await drain again; dropping the waiting future leaves borrowed join handles **inside the owner**. No AbortHandle/abort claim for blocking work; no Drop-spawn cleanup. Root lease FD lives through owner/worker clones, not only AppState's outer pointer.

At create owner registers receive task; before preview handoff registers a single blocking worker/supervisor join handle. The terminal publication boundary requires result trace plus closed native handles and joined task/cleanup witness. Worker completion/JoinError is inspected by the persistent owner supervisor even with zero HTTP requests. Panic retains any previously stored commit witness, closes/quarantines as possible; no success merely from join returning. The supervisor's own handle is retained by AppState/embedder shutdown handle and drained; don't make the supervisor itself an unowned detached task. Job state mutation, gate and snapshot revisions remain one short ownership domain; no lock held over await/SQL/filesystem/join. Writer->short-state only nesting; drain borrows handles outside short state lock and cannot detach work on cancellation.

### Standalone exact integration

- In the existing first-signal watcher (`main.rs:108–117`), close existing `stop_admission` **and** call jobs.close_admission_and_request_stop under the catalogue domain's short ordering. At listener/server error or normal service termination also call it before final drains, so an error path cannot leak admission. Ready/receiving/preview and open-gate apply request cancel; commitClaimed receives no false accepted cancel. Existing job status/cancel can answer during5s HTTP grace.
- Preserve5s HTTP grace,2s bus release, `KNX_STOP_WRITE_WAIT_SECS` parsing/default/max and **existing device drain behavior/report text**. No new catalogue stop invokes device cancellation. A device write already underway is still not safely cancellable by this catalogue contract.
- Catalogue cancellation starts at first signal, not after device waiting. Immediately after bus release, join **device drain and catalogue drain concurrently**, each with separate `stop.second_signal()` receivers. Existing device wait remains configured unchanged; catalogue has fixed30-second bound from its drain invocation. Concurrent waiting avoids adding30 seconds after a120-second device wait and avoids shortening device safety wait to30. Both see a second signal already latched, including one after HTTP grace.
- Final normal-return decision requires existing ServeEnd::Drained, device drain not TimedOut/SecondSignal, catalogue workOutstanding=false and all owned catalogue task/cleanup joins resolved. Clean closed-handle quarantine is logged as quarantine, not clean cancellation, but does not force waiting forever. Include catalogue outstanding status in existing forced-exit decision even when HTTP and devices drained normally. No dropped Tokio runtime used to stop blocking work.
- If request grace expired, a second signal, device wait ended early, or catalogue30s wait leaves work: preserve current explicit `process::exit(0)` escape semantics (not a new nonzero-exit/release policy), after bounded existing drain handling. Say “catalogue work remains; process exit can leave its outcome unavailable/possibly committed; retained backup may need inspection”, not cancelled/rolled back/cleaned. Preserve existing partly-written-device warnings verbatim. `stopped` means process stopping, not successful catalogue cancellation. SIGKILL/power loss remains crash-like and no terminal is fabricated.

### Desktop and API-only embedder

No GUI operation is needed to define the hook. Change desktop startup seam to return/retain `EmbeddedServerHandle` containing the HTTP shutdown sender, actual server future JoinHandle, Arc<AppState>/catalogue owner and a once-only shutdown task handle. Expose `request_close()` (sync idempotent close bus+catalogue admission and trigger HTTP graceful stop) plus `async drain_close(second_signal)` (HTTP5s bounded, bus release2s, device wait default120 unchanged semantics, catalogue30s concurrently; exact headless outcome). No environment-based policy invented for embedded runtime. This is a focused shutdown handle in the existing server/embedder code, not a generic lifecycle framework. Its result includes ServeEnd/BusRelease/existing WriteDrain/CatalogueDrain without collapsing them to bool.

Replace builder `.run(context)` convenience with `.build(context)?.run(|app,event|...)`;
retain frontend unsaved-check/dead-frontend close decisions. **Remove** the
current main-window Destroyed -> AppHandle.exit(0) enqueue before draining.
Destroyed calls the same idempotent `begin_close()` directly; ordinary
RunEvent::ExitRequested, including last-window implicit exit and explicit
AppHandle requests, first check exitAuthorized. Unless authorized they always
prevent_exit() and call that same begin function. Its atomic first transition
owns one shutdown task; repeated close/native notifications only observe it.
An ordinary close never supplies force authority by notification count or
Option<exit-code>. Thus one last-window destroy and any None/Some(0) duplicates
coalesce, regardless of ordering; they cannot cut device/catalogue drain short.

Only a separately identified explicit force action can request escape: a
confirmed desktop `force_quit` command with the current close generation (not
ordinary Quit or another native ExitRequested), or an explicit second-stop
request supplied to the headless handle. The ordinary close and force entry
points are distinct typed actions. Force is admitted only while already
draining with exact current generation; no first-request force or stale replay.
A repeated native exit is **never** upgraded into a user force request. Force
signals retained drains without reporting cancellation. If no force UI is
available after the window is destroyed, the documented bounded timeout remains
the escape; inventing a second user action from native events is forbidden.

When drains finish, set exitAuthorized before the sole final AppHandle.exit(0);
that authorized reentry proceeds without another prevent/restart or force signal.
Normal and force/timeout decisions preserve the separate device/catalogue
outcomes. If unresolved work remains after the specified bound or explicit
force, log truthful warnings and explicitly terminate as standalone does,
rather than returning into a blocking-runtime Drop. Native event-delivery
fallback is crash-like, not cleanup evidence. Restart is not exposed; OS kill/
session termination is not interceptable by this hook.

Required future source-bound/headless shell controls include one last-window
close producing Destroyed and ExitRequested(None), the retained old queued
Some(0) duplicate as a regression input, both callback orders, repeated direct
native requests, exactly one begin/drain, no early second-stop signal, authorized
final reentry, and a distinct matching/stale explicit force action. They must
show existing device safety wait is not shortened by a single Quit. These are
requirements for later tests, not executed Tauri or GUI behavior.

The headless entry calls **the same** EmbeddedServerHandle methods without a Tauri window or GUI main loop. A later noninteractive harness can drive request_close, cancel drain waiter, second stop and retained join ownership; a separate shell event mock/source-bound check must prove every real exit route reaches the shared hook. Existing current device desktop behavior is not retroactively certified; this adds explicit bounded lifecycle ownership before catalogue workers become reachable. This design alone executes none of those checks.
