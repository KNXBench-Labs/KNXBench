# Cancellable catalogue update jobs — WP-04.3 design candidate

Status: **architecture approved for staged implementation; not implemented**
(20 October 2026), under [ADR-0150](../adr/0150-cancellable-catalogue-jobs.md). The first review requested
revision for Ready/apply expiry, real wizard ownership and bounded idempotency.
The scoped rereview accepts those corrections and the disclosed growth cliff
at design level, not full implementation readiness. Concrete
[managed binding](catalogue-jobs/managed-binding.md) and
[protocol/resources/artifacts/lifetime](catalogue-jobs/protocol-artifacts.md)
supplements plus their adjacent schemas now close the former decision briefs.
The [cross-scope readiness record](catalogue-jobs/design-readiness.md) fixes
integration, explicit support refusals and staged implementation order.
Both final independent halves and their scoped correction rereviews accept the
consolidated contract. The parent approved the architecture after authenticating
their exact inputs and artifacts. This document does not change the
[existing update contract](catalogue-update.md).
The current synchronous routes, CLI, schema 28, package parser and STANDARD/LARGE
admission rules remain authoritative. No job, latency, memory or rollback result
is credited by this design. Primary-source closure and independent design review
are complete; focused test-first implementation and independent runtime/delivery
acceptance still precede exposing the described capabilities.

## 1. Scope and established source baseline

The source inventory is bound to `f255f90646fc40dbfaa5a6e84fafa6e97c847192`;
its non-journal implementation is unchanged from `921e1abb`. The controller
verified all 110 inventory file/report hashes, without executing product tests.
The private inventories remain evidence, not a public dependency of this contract.

Current relevant seams are `knx-productdb/src/wrapper.rs`'s transaction-private
`install_admitted_product_file`, `update.rs`'s public preview/apply and
`cleanup.rs`'s existing IMMEDIATE-before-validation ordering. The server holds
one `Mutex<Connection>`; current preview/apply collect uploads and synchronously
execute under it. Neither a catalogue job owner nor catalogue shutdown drain
exists. `ProductInstallControl` is shared by catalogue and actual wizard parents.

**No second importer.** Controlled and existing quiet callers must use the same
wrapper/package/ingest/parser/report body. New orchestration belongs in
ProductDB and a catalogue-specific server owner, never React or KNX Core.
No generic task framework, domain schema, native-model migration, format
interpretation, executable procedure or hardware permission is introduced.

## 2. Proposed decisions and outstanding closure

| Decision | Proposed first implementation | Closure before implementation |
|---|---|---|
| Live migration | Jobs require an already open, file-backed, current-schema catalogue. Missing/older/future databases are explicitly refused without creating or migrating them. Ordinary startup migration stays a separate, disclosed operation. | Verify every actual server/CLI entry; test older/missing/future refusal and startup boundary. Do not claim cancellation rolls back an earlier startup migration. |
| Preview | Private file-backed SQLite rehearsal; never install-and-rollback against live data, never a whole-catalogue in-memory clone in the controlled path. | Verify stepped-copy snapshot, page admission and source journal behavior with pinned SQLite/rusqlite. |
| Apply | Same managed writer connection, IMMEDIATE before final binding validation through backup/install/commit; repeat disk rehearsal first. Initially admit local default-VFS, private-cache, normal-locking DELETE-mode databases only, with no journal conversion to obtain admission. | Prove writer exclusion, source/path/connection incarnation and separate-reader copy in that supported lane. WAL and other modes explicitly refuse; support can widen only after a separate proof. |
| Cancel | One atomic gate orders accepted cancellation versus commit claim; cooperative checkpoints only. | Both deterministic gate interleavings; transitive reader/parser/report control feasibility. No hard millisecond promise. |
| Reports | Complete immutable bounded artifacts, paged to clients; refuse over-policy results before live commit, never truncate unsupported evidence. | Exact record/byte accounting and old-result parity before materializing collections. |
| Resource claim | Bound named source/main-database/result allocations. Explicitly do **not** claim an aggregate filesystem quota or process RSS bound. | Verify each enforcement point. Journal/WAL/temp files and parser allocations are separate; free-space checks are advisory. |
| Compatibility | Preserve old public signatures, v1 token bytes, legacy/password flow and synchronous response shapes. | Exact compatibility regressions; no automatic fallback to direct install or a different profile. |

The current-schema restriction removes migrations from a job's reachable live
transaction. It does not make migrations cancellable or authorize silently
moving them into preview/apply. A future controlled migration operation needs
its own transaction, transitive cancellation and evidence-history contract.
If an existing supported caller cannot meet this prerequisite, resolve that
caller explicitly rather than calling `open_and_migrate` inside the job.

## 3. ProductDB boundary

Add focused controlled siblings, retaining the existing quiet entrypoints.
Pinned-reader source/API feasibility is documented. The
[managed binding supplement](catalogue-jobs/managed-binding.md) defines proposed
controlled Rust boundaries, managed owner and typed witness. The
[protocol supplement](catalogue-jobs/protocol-artifacts.md) and adjacent schemas
define exact state/artifact/lifetime checks. These are design declarations,
not existing callable APIs. Their shared capabilities are:

- A sealed source is owned by the caller, has an exact immutable source name,
  byte length, SHA-256 and STANDARD/LARGE numeric limits, and has no caller-chosen
  filesystem path. Every consumed byte must be authenticated; client claims
  are not evidence. Admission of bytes and archive semantics remains shared.
- An explicit operation context carries a persistent cancellation/commit gate,
  typed operation-wide Stop, latest bounded real progress, policy and caller-owned
  private scratch. Retain existing slice XML readers initially, with before/after
  events, attributes, normalization and every skip-loop checkpoints. ZIP control
  covers both the sanitized central-directory view and original-byte overlay
  reader. Chunk reads and hashes through existing validation/decoder semantics.
  A separate control-aware BufRead path is not needed for this cooperative-only
  contract; introducing one later requires parity evidence. Whole token/value,
  dependency index-building, allocation/sort, decoder/native and filesystem calls
  remain disclosed unchecked spans. No
  HTTP, Tokio, React, authentication secret, ambient global or bus type leaks
  into ProductDB. Existing callers supply quiet control.
- Rehearsal produces existing preview semantics plus a separately versioned
  opaque binding. Preserve package order, source aliases, wrapper accounting,
  unknown/history diagnostics, conflict handling and exact retained bytes.
  Materialization must be admitted before allocation, including old catalogue
  program metadata and historical reports, not checked after a giant Vec exists.
- Apply borrows the actual mutable writer connection. Only its orchestration
  owns the live outer transaction; transaction-private installer bodies do not
  commit. Nested savepoint RELEASE and scratch commits never claim publication.
- Cancellation is a distinct operation-wide outcome, never an ordinary XML
  error converted into unavailable-history diagnostics and committed. Every
  reachable reread, hash, XML skip, classification/identity pass and final report
  stage observes it. Native SQL interruption is only a wakeup aid.

### Binding under reservation

Bind exact source name/length/hash, both numeric profile limits, policy version,
complete preview identity, current schema, catalogue identity/incarnation and
relevant full catalogue state. Include alias/placement/section and other changes
missed by the old package/source-set fingerprint. Bind the server/job lifetime
and one consumed consent separately at the server boundary.

Compare under the actual SQLite writer reservation, not only an HTTP mutex or
an earlier stat. `data_version` comparisons need the same long-lived connection;
a server counter alone cannot detect another process's writer. Reopen/restore,
same-content replacement, source/profile substitution and path versus existing
SQLite-handle identity all require explicit rejection. The
[managed binding and witness codec](catalogue-jobs/managed-binding.md) fixes
persistent same-W observation, conservative generation, restore incarnation,
exact typed/chunked schema/table data and explicit unsupported shapes. No
partial hash or unspecified watcher substitutes for that contract. The support boundary excludes
hostile same-user filesystem mutation; that exclusion does not waive ordinary
restore/replacement/CLI-writer races.

### Publication and failure order

1. Atomically admit Ready consent under section 4; this does not claim COMMIT.
2. Check current-schema/file/source/policy prerequisites without live creation,
   acquire managed writer W cancel-aware, then IMMEDIATE with finite retry,
   before any live DML. Revalidate exact binding; stale means no recovery backup
   or live install. A private rehearsal may already have existed from preview.
3. Open distinct read-only/private-cache source S on the same verified database.
   BEGIN DEFERRED plus a finalized real main-schema read pins its snapshot only
   after W is reserved. Never source a backup from W while W has its write
   transaction. Reserve does not itself prove pathname/handle identity.
4. Exclusively create each disk destination, with known page-size/settings and
   admitted caps before copying; use positive bounded page steps, finite retries,
   cancellation and deadline checks. Require observed Done, then drop the backup
   handle before any destination query. Do not use unbounded run-to-completion.
5. Rehearsal and pre-write recovery copies both derive from unchanged S, never
   from an already-installed rehearsal. Complete and validate the copies, end
   S's read transaction and close S before any W writes or COMMIT. Otherwise
   the owner's own DELETE-mode reader can block its commit. Preview-only copying
   releases the unchanged W reservation before offline rehearsal.
6. Repeat rehearsal under reservation and admit complete result artifacts; run
   only the shared transaction-private installer on W. Validate preview/report
   agreement and all fallible result admission before the final gate.
7. Win the commit gate and COMMIT once, or explicitly roll back if cancel won.
8. Preserve witnessed commit independently from later response/refresh/cleanup.

The official backup API labels write-source failure LOCKED; the authenticated
pinned SQLite 3.53.2 source returns BUSY at that branch. This is a documented
source distinction, not an executed witness here. Neither permits copying from
the reserved writer or infinitely retrying self-conflict. Done, not Drop/finish
alone, is the copy completion witness. Actual wrapper/version/fault behavior
requires disposable runtime proof before production acceptance.

A backup is allocated create-new, never exists-then-open; incomplete files must
not look like valid listed backups. Cleanup removes only verified owned files.
A committed or unknown-outcome backup is retained for recovery. For proved
rollback, remove only the still-owned backup, and record removal failure.
Drop attempting rollback is not verification: classify explicit rollback and
transaction state. On unresolved rollback/commit, retire or quarantine the
connection and block further writes; never report a clean cancellation.

## 4. Cancellation, settlement and worker ownership

The publication gate has mutually exclusive `Open -> CancelAccepted` and
`Open -> CommitClaimed` transitions. Cancellation returns accepted only after
winning that transition. Worker claim happens immediately before the only live
COMMIT, after all other fallible work. A checked boolean followed by COMMIT is
not acceptable. No callback, long work or database call runs while holding a
short snapshot/control lock.

### Ready/apply/expiry admission (R1)

Ready consent is single-use, governed by an exact monotonic deadline and a
phase generation. Apply takes the same short owner-control critical section used
for stop/expiry: check current phase and generation, exact handle/source/profile/
preview binding, unspent consent, open admission and `now < readyDeadline` using
a fresh monotonic clock sample **inside** that section. At `now >= deadline`,
refuse as expired immediately even if the reaper has not run. An expired phase
claims the persistent cancellation gate and moves to settlement; no job may be
revived by a stale status or delayed consent. No ProductDB call occurs there.

An accepted transition consumes consent, records its exact command identity and
switches to WaitingForWriter with a new phase generation and a new 2-hour
working deadline. It clears the Ready deadline; that deadline does not cancel
work admitted before expiry. Exact accepted-command retries return current
status while the record is retained, not a second transition. A reaper holding
a computed expiry must revalidate phase, generation, the exact deadline and
fresh time under this same section. After apply won, its old Ready expiry is
a no-op. Receiving/preview and apply each use the relevant phase deadline;
preview Working time is replaced by the Ready lease, not combined into a
fictional global progress clock. Stop closes admission under the same owner
control ordering; apply cannot pass a stop that won first. Working timeout and
stop still compete through Open->CancelAccepted versus CommitClaimed. A later
stop can cancel an admitted but not yet commit-claimed apply. Neither a deadline
nor timer firing proves the worker has stopped. Gate state, phase generation,
consent and snapshot must be changed in one short consistent ownership domain.

- Cancel wins: publication is forbidden. Worker rolls back, closes handles and
  settles owned artifacts; only then report terminal `cancelled/notCommitted`.
- Commit wins: cancel returns `tooLate`, not success. Actual COMMIT can still
  fail. Distinguish committed, proved not committed and unknown; unknown forbids
  automatic retry. Late cancellation cannot erase witnessed commit evidence.
- Cleanup failure is explicit. Retain the reservation/resource charge until the
  worker and cleanup have ended or the leaked resources are quarantined. A
  terminal-looking snapshot alone does not release the active slot.
- Status and cancel never acquire ProductDB. The sole nesting direction is
  writer/reservation -> short job gate; no manager/API/reaper holds that gate
  while waiting for ProductDB, disk, parsing, serialization or join.
- Keep a tracked worker/join handle through settlement. HTTP disconnect, UI
  unmount, dropped join waiter, panic or `spawn_blocking` handle abort is not
  proof of stopped work. Preserve already witnessed committed evidence on panic.

Progress is phase-local measured work with nullable total: real received/hashed
bytes, archive entries, decoded bytes **per pass**, parser events or copied
pages. Coalesce latest state; no unbounded event stream, invented ETA or global
percentage. Large token/native-call latency is disclosed as cooperative until
its actual worst unchecked spans are independently measured.

## 5. Proposed resource and lifetime policy

These are product-policy candidates, not KNX/ZIP maxima, measurements or a
promise that a maximum-profile package fits. They must be explicitly tested at
inclusive/exclusive bounds before adoption. Refusal has its own typed reason.

| Item | Candidate policy |
|---|---|
| Active jobs / queue | One including Receiving/Ready/Settling; no queue |
| Source bytes | Existing 1,000,000,000-byte ceiling; declared and actual bytes agree |
| Expansion | Existing STANDARD 4 GiB / explicit LARGE 16 GiB; shared wrapper budget unchanged |
| Existing catalogue before copy | 8 GiB page bytes, checked in stable source snapshot before copy |
| Rehearsal main database | 64 GiB maximum page bytes, validated page-count enforcement after copy |
| Results | 64 MiB complete artifact, 100,000 logical records; refuse before oversize collection |
| Transport | 16 KiB status; 256 KiB detail page; at most 200 records; one record at most 64 KiB |
| Display source name | At most 512 UTF-8 bytes, basename only, no controls; refuse rather than silently rename |
| I/O / backup checkpoints | 64 KiB caller I/O chunks; 128 positive backup pages per step |
| SQLite busy / writer wait | At most 50 ms per native busy wait, 30 s total acquisition/retry; cancel-aware waits |
| Receiving | 60 s idle, 1 h overall; timeout requests settlement, does not prove it instant |
| Ready consent | 15 min monotonic deadline; polling does not renew it |
| Working deadline | 2 h triggers the same cancellation gate; no forced-thread-stop promise |
| Retention | At most 8 settled terminal records, each at most 1 h from settlement; at most 8 tombstones, each at most 1 h from eviction. Earlier count eviction and explicit disposal can end detailed lookup; the admission high-water mark never rewinds within an incarnation. |

**Catalogue-growth cliff (M1):** the 8 GiB input and 64 GiB rehearsal ceilings
are deliberately separate policies. A successful install can leave the live
catalogue above 8 GiB; its result must then warn that a later job can refuse
`catalogueAdmissionLimit` before copying. Both real UI surfaces present that
consequence before consent and on the result. Do not silently switch to an old
direct route, reduce profile, delete data or enlarge budgets to bypass refusal.
Manual CLI maintenance remains a separate explicit operation with its own
contract, not an automatic retry. Successful current-schema startup alone does
not promise that every growing catalogue remains eligible for the job path.

Only enforceable named caps may be reported as enforced. `cache_size`,
`temp_store`, `max_page_count`, periodic directory stat and free-space admission
alone are not an aggregate hard disk/RSS quota. Pin the actual journal mode,
cache/temp settings and resulting scope in implementation evidence. Account for
source, rehearsal, reports, backup and journal/temp separately; disk-full must
fail atomically. Do not invent a quota backend or enlarge format limits to make
tests pass. If a needed named cap cannot be enforced, revise this design before
implementing its dependent promise.

## 6. Catalogue-specific server protocol

Keep all operations under the existing auth guard. This is the current
single-operator model, not new per-user authentication. Each selection owns an
unlogged CSPRNG client capability; every operation binds it, job id and server
incarnation. Do not put capabilities in URL paths or artifact names. A random
job id is not consent or a replacement for authentication.

Proposed additive routes (strict request fields; versioned strict responses):

| Operation | Required behavior |
|---|---|
| GET `/api/catalog/update/jobs/admission` | Return current server incarnation and next canonical decimal admission serial; no allocation, reservation or authority to install. Read once for an explicit new selection; a conflicting concurrent offer may refuse rather than silently retry. |
| POST `/api/catalog/update/jobs` | Admit bounded JSON with the offered incarnation/serial, exact source/profile/declared bytes, LARGE acknowledgement and selection capability before upload. Allocate under the short control lock only if the serial is still next and capacity/admission permit; increment the high-water mark once. Retained exact replay returns the same job; consumed-but-forgotten serial refuses as outcomeUnavailable. |
| PUT `/api/catalog/update/jobs/{id}/upload` | One raw octet stream into private create-new spool, counted and hashed with backpressure/timeouts. No multipart buffering, resume or replacement. Exact EOF seals it and owns preview worker. |
| GET `/api/catalog/update/jobs/{id}` | Cheap bounded coherent snapshot independent of ProductDB; exact owner handle required. |
| GET `/api/catalog/update/jobs/{id}/details` | Complete immutable section pages bound by digest/count/cursor; no source download or host path. |
| POST `/api/catalog/update/jobs/{id}/apply` | Atomically admit exact Ready consent under section 4, with `confirmed:true` and stable command identity; no replacement bytes/profile. Exact retained accepted-command replay reads status; forgotten or disposed handle is outcomeUnavailable, never an implicit new install. |
| POST `/api/catalog/update/jobs/{id}/cancel` | `requested`, `alreadyRequested`, `alreadyTerminal` or `tooLate`; acceptance is not a terminal cancellation. |
| DELETE `/api/catalog/update/jobs/{id}` | Dispose only after worker and cleanup ended; never delete retained recovery backups. Active disposal refuses. |

### Bounded replay, not permanent idempotency (R3)

The manager retains one nonwrapping admission high-water serial for its server
incarnation, in addition to bounded records/tombstones. It is not secret, a job
id, a snapshot revision or user consent. On create, inspect exact retained
incarnation+serial+capability+metadata **before** considering fresh admission. A
retained exact command returns its job; conflicting reuse refuses without
consuming another serial. If no record remains and the requested serial is
below next, return 410 `outcomeUnavailable`; a request above next is invalid/
stale-offer. A matching next serial may allocate only once under the lock;
capacity/stop refusal does not advance it. Never treat an old consumed serial
as fresh, even after all corresponding records and tombstones are evicted.
On serial exhaustion close admission; do not wrap. Restart uses a new
incarnation, so old requests cannot become fresh again. These rules bound
replay memory without pretending an eight-entry ring remembers everything.

Detailed status/apply/cancel replay is guaranteed **only while its full record
is retained**, not for a minimum hour. Count pressure or explicit disposal can
end that horizon earlier; TTL is an upper limit, not a promised availability
window. A tombstone, when present, identifies an unavailable old handle without
reconstructing its outcome; after it too expires, unknown handle returns the
same generic 410 without allocating or assuming notCommitted. Unavailable
outcome is distinct from stale preview, known failed or cleanly cancelled.
Never expire an active or unsettled record to make space.

Lost create responses may retry the exact offered serial+selection capability+
metadata; they must not fetch a newer serial automatically. While retained this
finds the same job, even if it fills the sole active slot. Once unavailable, the
client shows historical outcome unknown/unavailable and requires an explicit
new selection, a fresh offer/capability and fresh preview/consent for a new
operation. Lost apply/cancel responses query the same exact job/command; 410 or
server restart never permits automatic recreate, reupload or reapply. A fresh
selection after an unknown outcome must visibly warn that the prior job may
have committed. StrictMode reuses the original pending offer/command; it is not
a reason to mint another. GET admission performs no work needing its own retry
ledger, and it does not claim a reservation against other clients.

Use `{state:"running",phase:...}` for Receiving, Previewing, Ready,
WaitingForWriter, Validating, BackingUp, Installing, Committing and Settling;
terminal states are Succeeded, Cancelled, Stale, Expired or Failed. The exact
[wire schema](catalogue-jobs/wp043-protocol.schema.json) and
[cross-field policy](catalogue-jobs/wp043-protocol-policy.json) fix closed
camelCase spelling, nullability, checked counters and all state combinations.
Rust and TS validators must implement both structure and semantic rules; static
schema validation does not establish those runtime validators. Common required fields include schema version, opaque identities,
canonical decimal-string monotone revision, immutable source/profile, real byte
counts, nullable sealed digest/progress/preview/result/failure, commit evidence
and cleanup disposition. Admit every intermediate state as strictly as terminals.
Reject impossible combinations, bounds, missing nulls, unexpected keys and
cross-job bindings. Same revision with different content is a protocol error.
A response can never authorize a different selection or re-enable spent consent.

Errors use fixed sanitized kinds/messages with only admitted archive-relative
context; no raw exception/Debug/SQL/panic, local path, credential or payload in
status. Raw authoritative provenance and safe public projection are distinct:
historical source aliases can be host paths even though a new upload name is
a basename. Retain raw bytes/provenance internally; do not silently rewrite them
or expose them in the new API. Complete paged public artifacts must explicitly
account for policy-withheld fields/counts, not claim byte-exact raw completeness
when redacted. If a record cannot be safely projected without hiding required
consent facts, refuse the job before publication. Details never use first-N
summaries posing as complete. No ProductDB work or unbounded serialization on
status/cancel.

## 7. Files, restart and shutdown

Spool storage is private, outside ordinary filesystem API/static-file reach,
with restrictive permissions and create-new names. Enforce that exclusion at
the real relative/absolute filesystem read/write containment choke points and
static-root validation, including configurable data roots; placement outside a
listing root or permissions alone do not fence routes in the same process.
Before startup reclamation,
prove single-owner root/incarnation through a kernel-held lease; never sweep by
prefix/mtime or delete another active process's workspace. Count failed cleanup
against admission or quarantine it; do not forget leaked bytes.

Restart changes server incarnation and invalidates volatile job/consent
handles. It does **not** mean a former operation was cancelled. No automatic
resume/reapply; committed catalogue data and user-managed backups remain.
Durable across-restart outcome lookup is outside this first volatile job API.

First stop closes job admission and competes at the same cancel/commit gate.
Drain tracks workers and cleanup even when no HTTP request is open; bounded
wait/second-signal escape is not proof of cancellation. Include catalogue work
in standalone exit decisions, and expose the same explicit close/drain API to
the desktop embedder. Never rely on Tokio runtime Drop to stop blocking work.
The [concrete lifetime contract](catalogue-jobs/protocol-artifacts.md#p9-concrete-ownership-standalone-stop-and-desktopheadless-drain)
defines standalone/embedded retained joins, first/second stop, existing device
drain preservation and the Tauri process-exit boundary. Review and actual
headless/source-bound shell controls must pass before workers become reachable
from either production server lifetime.

## 8. Shared UI and compatibility

The catalogue and actual device wizard use one **App-owned** catalogue job
controller/provider, above both `CatalogBrowser` and the conditional whole
`DeviceWizard`. A hook inside `ProductInstallControl`, `DeviceWizardProduct`,
the product step or wizard instance is not the job owner. The root controller
retains the capability, command/offer identities, worker-facing poll lifecycle,
origin wizard-run id and admitted commit evidence while those leaves unmount.
This is one injected application controller, not ambient module-global state.

### Navigation and committed invalidation (R2)

Product -> placement -> product detaches and reattaches a view to the same
root-owned job, without another create/upload/apply. Wizard dismissal while a
job is active explicitly offers continue-in-background or request-cancellation;
neither silently disposes the handle. Continue leaves a visible root-level job
indicator. Cancellation still needs settlement; committing/too-late is shown
as such. Close must not imply server stop. A dismissed wizard run is not updated
as an unmounted React instance; later reopening a genuinely new wizard run
uses a new run id, while any pending job stays discoverable at root.

A witnessed commit is processed at the root by exact incarnation/admission/job
identity independently of the current file selection generation. It increments
one monotone catalogue-install revision and records the bounded outcome once.
Both the already-mounted hidden catalogue and its opened backups observe this
revision and invalidate once without resetting filters. Any live wizard
product list observes it too, and a retained origin wizard run records that an
installation committed even while its product step is detached. On return its
installed-package discard warning is true and its list refreshes from the dirty
revision. If the whole origin wizard is gone, the root outcome notice reports
that catalogue change; no callback is sent to a dead instance or to an unrelated
new wizard as if it were the old run. A newly mounted surface reads the latest
revision rather than needing an unbounded historical event queue.

At most one active job and bounded retained outcome records hold explicit
per-job notification markers; do not grow an eternal Set of delivered ids or
use a high-water mark to suppress a previously unseen still-retained commit.
Root processing accepts a response only for its exact tracked job; after that
record is evicted or explicitly disposed, delayed replies cannot reattach it
or reconstruct a forgotten outcome. A catalogue revision is a freshness cursor,
not a substitute for per-job origin evidence. Keep view rendering generation
guards separate from committed invalidation. They drop stale local paint, never
the catalogue-wide fact of a tracked commit. A refresh failure is a visible dirty/retry
warning, not install failure; retry refresh does not reapply. Closing/reloading
the whole App or process can still lose the volatile owner: warn while active,
never claim server stop, recover only exact retained handles when available and
show unknown historical outcome after restart/forgetting. No across-restart
durable outcome lookup is invented.

Explicit LARGE opt-in is per selection, independent from final apply consent.
New File objects get new generations even with identical name/size/mtime.
StrictMode reuses the exact root-owned admission offer and commands. Selection
replacement cannot overwrite an unsettled job: reattach or explicitly settle
that job first. Every render-local callback checks generation and handle; root
status/commit processing stays bound to its own persistent handle. Leaf unmount
stops only leaf subscriptions, not root polling; application-owner unmount ends
transport without proving worker stop.

Render cancel-requested, settling, too-late, expiry, stale and restart honestly.
Lost apply/cancel responses poll the same job within the bounded replay horizon;
never create another apply automatically. Legacy/password fallback runs once
only after typed refusal and worker/cleanup settlement; unknown/unavailable
outcome, unsafe resource refusal and the growth cliff are not fallback triggers.
Keep backup ask-first restore, identity-bound delete, filters and root
install-revision refresh behavior.

Existing synchronous endpoints retain their wire behavior. Their expensive
work cannot remain on an async runtime thread if making a server-wide
responsiveness claim; review scheduling/admission separately without silently
changing compatibility responses or limiting input after buffering it.

## 9. Required acceptance (none executed for this candidate)

| Proof | Required discriminator |
|---|---|
| Shared semantics | Plain/wrapper, known alias, order, source bytes, reports/unknown/history and STANDARD/LARGE exact-limit parity |
| Binding | Other-process writer, alias-only/placement change, same-content restore/reopen and replacement, source/profile/policy substitution |
| Cancellation | Deterministic cancel-wins and commit-wins races at the production gate; later wrapper member rollback; every reachable read/decode/XML/identity/report stage |
| Failure | Backup copy/finalization/cleanup, disk full, SQL interrupt, COMMIT failure and rollback failure; no cancelled claim on unknown outcome |
| Resources | Before-copy page admission, growth and result-record/byte bounds, complete pagination, no whole-memory catalogue copy; no claimed aggregate quota |
| Ownership | Status/cancel responsive with real ProductDB mutex held; dropped HTTP/join waiter, panic, retained cleanup slot; apply before/at/after Ready expiry, delayed reaper across accepted apply, stop versus apply and working timeout |
| Lifetime | Restart invalidation, foreign spool protection, actual standalone stop/drain and headless desktop embedder drain |
| Admission | All routes authenticated before body consumption, strict every-state DTO, source/capability/profile/consent/revision binding; lost create/apply, count/TTL/tombstone eviction, consumed-serial replay, conflicting metadata and concurrent offers with no automatic reapplication |
| UI | App-owned controller, real wizard step/dismissal and hidden-catalogue/backups, commit while detached, once-only/deferred invalidation and filters; growth-cliff refusal; StrictMode, same-metadata new File, unknown outcome, legacy/password; built browser EN/DE narrow/light/dark |
| Delivery | Independent whole-change review, compiled critical guard discrimination, maintained scoped branch/integrated gates and verified publication |

Dependency order: close primary questions and policy -> independent design
review -> gate/service/rollback tests -> disk rehearsal/binding/report admission
-> transitive controls -> owned server lifecycle/strict protocol -> shared UI
-> cross-layer failure/compatibility proofs -> reviewed maintained delivery.
No later layer may paper over an unresolved earlier invariant.
