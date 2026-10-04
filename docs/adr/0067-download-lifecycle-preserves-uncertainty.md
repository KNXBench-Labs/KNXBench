# ADR-0067: Download lifecycle receipts preserve uncertainty and legacy history

Date: 2026-10-03
Status: Proposed (bounded offline implementation and integrated gates passed; broader owner admission pending)
Scope: SAFE-03 / AUDIT-01, offline server/storage lifecycle only

## Integrated scoped acceptance — 2026-10-04

The bounded implementation is published as `1c5dec07` after renewed merged
source32/public Rust-Cargo369/tracked Web acceptance: scoped125/0/0, required
ordinary3022/0 with176 ignored, Web typecheck/offline build and Vitest1739/98.
All26 actual command stages/dual leases/original private commitments are bound
in the [permanent receipt](../evidence/commission-download-lifecycle-offline-2026-10-04.json).
The initial stale inner receipt namespace safely refused before Cargo/input
selection; initial three successful stages and corrected17-phase continuation
are separately retained. No product/fixture change was needed. Five actual
semantic controls were executed on the pre-integration source, not on this
merged context. Review remains in-session. History stays partial metadata,
not recovery or device-effect proof. Broader callers/Web adoption, full owner
admission and crash/power-loss/hostile-race/hardware/vendor/ETS acceptance are
not established; therefore the ADR remains Proposed. Older pending notices
below describe their explicitly historical snapshots, not this current gate.

## Fault-fixture correction researched 2026-10-03

SQLite's primary file-format specification §1.3.3 identifies the one-byte write
and read versions at offsets18 and19:1/1 for rollback journaling,2/2 for WAL.
This is distinct from application-owned user_version/history format2.
Source: https://www.sqlite.org/fileformat2.html#file_format_version_numbers

The first compiled worker fault probe installed only an arbitrary nonempty -wal
marker beside a main file still declaring1/1. Our current opener checks the main
header, not arbitrary orphan-sidecar existence, so that fixture did not guarantee
the intended refusal. Its named worker-outcome failure is retained as a rejected
fixture premise, not proof of a broken keeper, real WAL recovery, or hardware write.

Correct the test-owned input to declare unsupported2/2 before returning its fake
tunnel, keep a synthetic marker, then snapshot those intentionally injected bytes
as the refusal baseline. Require no subsequent database/marker mutation or SHM
creation, retained device backup, no device-changing sends, cleanup and later
write refusal. This remains simulated admission failure, not authentic WAL/crash.
Orphan-sidecar presence policy with a rollback header remains unresolved coverage;
do not declare it accepted out of scope or silently universalize header-only checks.

The corrected admission-refusal test and default-disabled-control terminal test
now each passed1/0/0 with retained exact public named outcomes, unchanged original
inputs, both lease pairs and32 captured source/build-input paths. A separate
compiler-only stage built the actual integration executable. The refused main
bytes are the intentionally injected baseline, not an authentic WAL snapshot.

### Proposed orphan-sidecar admission policy (test-first, pending)

For this owned rollback/DELETE metadata store, an existing -wal, -shm or -journal
entry is unsupported/uncertain even if the main header declares1/1 or the main
file is absent. Refuse before sentinel creation or any SQLite open. Do not parse,
recover, delete, truncate or follow sidecar symlinks; preserve the input exactly.
This is our conservative admission policy, not a claim about every SQLite file's
semantics. Metadata errors must fail closed; it is not a hostile concurrent path-
replacement or power-loss guarantee. Public synthetic tests must first prove the
missing refusal against current code, then preserve each opener/main-presence/
suffix/empty-or-nonempty input on the corrected code. The public orphan test
actually compiled and failed on admitted sidecar; only afterward a minimum
no-follow entry check was added before creation/open. Whole-source delta is
verified; complete history11/0/0, OneShotLog17/0/0 and two named actual-worker
regressions1/0/0 each passed on that candidate.24 regular marker combinations
are one completed storage Rust leaf, not24 extra tests. Unix directory/link
coverage then passed12/0/0 with18 additional combinations in one new leaf,
preserving main/children/link/targets and not creating dangling targets.
Metadata-error and hostile-race acceptance is not inferred. Next actual keeper
backup-save-failure regression compiled, then failed without retained public
location; negative evidence preserved. Diagnostic-only keys located a wrong test
enum:available rather than configured. Only test literal corrected; full corrected
case and positive terminal-worker regression passed1/0/0 each after actual rebuild
on their own frozen32-path snapshot/dual leases/originals unchanged/raw discarded.
Test-owned backup directory blocker proves no mutations/no retained backup,
failed/no worker/one cleanup and actual durable failed/no/backup=false/intent=false/
cleanup row through successful history query, not universal keeper-fault handling.
Configured is not health proof; no production change or compatibility alias.
No retained-backup/intent success may survive a failed save. Full gates keep Proposed.

Actual worker cleanup error/panic/default regressions now passed1/0/0 each after
actual rebuild on a distinct frozen32-path snapshot/every dual lease/originals
unchanged/raw discarded. Witnessed finished/yes/acknowledged result and backup,
terminal timestamp/identity/sequence survive; cleanup independently returnedError/
unknown, reservation released, adapter once, history query no bus contact. Initial
test failed because document-only snapshot omitted row/API identity fields; actual
row fields were added, all preservation comparisons retained, production unchanged.
This is not pending-cleanup/midwrite abort or terminal/cleanup-storage-fault proof.

Subsequent cleanup-recording admission-refusal case and error/panic/default
regressions passed1/0/0 each on a new frozen32-path snapshot after rebuild/every
dual lease/originals unchanged/raw discarded. Synthetic unsupported header injected
after witnessed terminal result, before adapter OK, preserves device result/backup,
latches unavailable history/new-start refusal before contact and preserves refused
bytes/no sidecars. Not an authentic WAL/crash or repair guarantee.

Pending-cleanup interruption tests use complete drop of a test-owned current-thread
runtime, then a separate observer runtime. Pinned Tokio1.53.1 documents async-task
drop after yielding during shutdown and suspension of current-thread tasks after
block_on returns. This test mechanism adds no cancellation surface or hard-kill/
power-loss guarantee. Primary source:
https://docs.rs/tokio/1.53.1/tokio/runtime/struct.Runtime.html#shutdown

The actual pending-cleanup shutdown case plus storage/error/panic/default controls
passed1/0/0 each after rebuild/frozen32/every dual lease/originals unchanged/raw
discarded. Full owned runtime drop releases reservation and records unknown
cleanup while preserving witnessed result/backup/timestamp/identity/sequence;
adapter entered once, observer query adds no device frames. No production change,
public cancellation route or hard-kill/power-loss guarantee; broader gates open.

Subsequent actual midwrite future-drop and terminal-recording admission-refusal
cases plus cleanup controls/default worker/Store12/OneShot17 passed36/0/0 on one
new frozen32-path snapshot after integration rebuild/every dual lease/originals
unchanged/raw discarded. Midwrite shutdown preserves backup/intent and row
identity, live failed/partially versus durable unknown/null written/restart/unknown
cleanup/no added frames/no successful adapter return. Terminal-recording refusal
preserves witnessed worker result but latches unavailable history/new-start503
before contact/refused baseline unchanged; never infer durable terminal success.
These test-only extensions do not change production or establish hard-kill/
power-loss/rollback/restoration/hardware/ETS guarantees. Negative source mutants,
version/fault closure/full integrated review/delivery still pending; Proposed.

The following corrected snapshot passed all-target server/store Clippy, actual
integration rebuild, owning header-refusal plus seven worker controls, Store12
and OneShot17:37/0/0 with frozen32/every dual lease/same original commitment/raw
absent. Two earlier lint101 receipts stay negative. Only two scoped argument-count
annotations and lexical fixture snapshot-lock scope changed; no broad suppression
or executable production-statement changes.

### Reader-blocked migration finalization: next bounded test-only scope

The existing BEGIN IMMEDIATE failure fixture blocks writer acquisition, not a
dirty migration's final commit. SQLite's transaction primary documentation states
that another open read connection can make COMMIT return SQLITE_BUSY while the
transaction remains active. Source directly fetched HTTP200/read 2026-10-03:
https://www.sqlite.org/lang_transaction.html#implicit_versus_explicit_transactions
The local Cargo-pinned rusqlite0.40.2 transaction.rs uses default rollback behavior;
consuming commit executes COMMIT, and Drop invokes finish_ while ignoring rollback
errors. This source inspection does not prove rollback succeeded in our opener.

Add a synthetic nonempty-v1 fixture with an actual held read transaction. Require
a nonempty rollback-journal witness from the real upgrade opener before refusal,
then unchanged main bytes/version/documents/operation keys/cursor counter and no
retained sidecars. Only explicit test-reader release permits a later operator
retry; require retained cursor continuity. No production hook/repair/auto-retry,
private input, device mutation or hard-kill/power-loss inference. Compiler/focused
runtime/full public Store/OneShot/all-target lint on this new source PENDING.

Actual new source compiler/all-target server+store lint passed. Focused migration
leaf plus complete Store13/OneShot17 passed:30 distinct public tests, no failures;
the separately selected leaf is included, not added twice. Exact source32/every
dual lease/native gone accepted, real dirty-journal witness and subsequent state
preservation checked. This is not power-loss/hardware or current private-worker
coverage. Next three public production-source controls remove terminal monotonicity,
late-intent refusal or COMMIT-error propagation, one at a time, only after leases,
with exact named semantic RED and finally canonical restoration/public bookend.

All three controls actually compiled0 and ran one exact registered leaf101 at
the intended semantic assertion: repeat-result rejection, late-intent rejection,
and late-migration/finalization-refused. Restored every canonical source byte in
finally before actual alpha/workspace lease release; Store13/OneShot17 canonical
bookend30/0/0. Complete current-source lint/build/eight-real-worker/public-suite
chain PENDING (38 distinct expected), not historical scope borrowing or Proposed
ADR/full SAFE03 acceptance.

Restored complete current-source chain now passed all-target lint, actual HTTP
integration build, eight actual privacy-closed workers and Store13/OneShot17:
38 distinct/0 failed/0 ignored, source32/chain/every actual dual lease/originals
unchanged/raw absent. Same-source three production mutants killed and restored.
Full branch review and actual changed history API format2/future3 projection
runtime remain pending; Proposed and no full SAFE03/parent acceptance.

## Context

ADR-0064 implements four durable one-shot kinds. A download currently admits
history before `connect_tunnel`, but its worker result, backup association and
cleanup lifecycle remain volatile. `device_download.rs` already distinguishes
written extent, restart outcome and worker reservation during cleanup.
`memory_download.rs::run_steps` invokes the synchronous backup keeper after
original-value reads and before the first device-changing step. This is the
existing pre-write boundary; no KNX procedure or authorization is widened.

SQLite's official synchronous documentation distinguishes FULL from EXTRA in
rollback/DELETE mode: EXTRA also synchronizes the directory after journal
unlink. FULL does not necessarily retain the last committed transaction across
power loss on every filesystem. SQLite's user_version is application-owned;
the engine does not interpret our compatibility policy. These are storage
facts, not measured hardware or power-loss evidence.

Primary references inspected 2026-10-03:

- https://www.sqlite.org/pragma.html#pragma_synchronous
- https://www.sqlite.org/pragma.html#pragma_user_version

## Proposed decision

Use EXTRA synchronization on every admitted writable history connection,
including initialization and migration transactions. Keep the rollback-only,
raw-header-before-SQLite and read-only-before-writable admission boundaries.
Foreign, future, malformed, hot-journal and WAL databases remain fail-closed;
never checkpoint, repair, replace or downgrade them automatically.

Introduce history format 2 for the additional closed download vocabulary.
Upgrade only positively admitted format-1 history with its exact known schema.
The migration changes the version in one transaction, not the historical
metadata documents, operation keys, cursor sequences or AUTOINCREMENT state.
Legacy rows remain legacy observations; never invent their missing download
facts. An older format-1 opener refuses the upgraded database rather than
misinterpreting download receipts. Project and manufacturer databases are
unchanged. Migration must have preservation, refusal and failure regressions.

Record one download lifecycle row before tunnel acquisition. Its identity is
server incarnation plus activity ID, with the existing session ID as a separate
correlation field. Keep downloads out of the volatile one-shot ring; the current
live download session remains the reservation/status owner. Add only bounded,
closed metadata: backup-recorded and possible-send flags, nullable witnessed
written/restart outcomes, and an independent tunnel-cleanup outcome. Never
include recovery payloads, keys, serial numbers, host paths or exception text.

Persist the possible-send boundary only after the existing recovery keeper has
successfully retained the original values, and before that callback permits any
device-changing step. Start/intent storage failure refuses tunnel acquisition
or the write, respectively. History is not the recovery image. Preserve the
IAW-only recovery exception and the existing phrase, plan, scope and lock gates.

Record the witnessed device result independently of tunnel cleanup. A worker
panic/drop or previous-incarnation nonterminal receipt is unknown/interrupted,
not successful cancellation, no-change or safe rollback. Unknown written extent
is nullable, never fabricated as `no`. A cleanup interruption must not erase an
already witnessed device result. Cleanup metadata describes the adapter's
observed return, not a newly invented device-disconnect acknowledgment.
Terminal recording failure latches unavailable history without relabelling a
witnessed write as `notSent`. No automatic retry, restore or new cancellation
route follows from this decision.

Once a terminal device result is witnessed, another terminal update must refuse
without replacing its stored outcome, timestamp or operation identity. Cleanup
may advance independently; rejecting an invalid transition is not itself a
storage failure. The dedicated preservation regression compiled and failed on
accepting a second terminal result. A minimal prior-state refusal followed that
RED; complete OneShotLog GREEN passed16/0/0 and new-source terminal HTTP regression
passed1/0/0 with actual server rebuilds and unchanged inputs. This is bounded
enforcement evidence, not complete fault or release acceptance.

Once a device result is terminal, a late possible-send/backup-intent update must
also refuse before changing the row or permitting a new mutation. The dedicated
failed/no preservation probe compiled and failed on accepting late intent. Only
afterward a minimum prior-state refusal was added before intent mutation/persistence;
complete OneShotLog, synthetic worker and selected terminal HTTP GREEN/regressions
passed17/0/0,8/0/0 and1/0/0 on that candidate. The corrected actual-worker admission-
refusal test and default-control positive terminal test subsequently each passed
1/0/0 on their own32-path snapshot: refused synthetic main-header2/2 prevents
property/memory/restart sends despite retaining the backup, preserves injected
metadata bytes, completes cleanup and blocks later starts. This is neither
authentic WAL/crash evidence nor a production change. This ADR remains Proposed
pending orphan-presence disposition and remaining acceptance gates.

The first actual fault-scope attempt failed test-instrumentation compilation,
before any test execution. A second simulator-helper constructor was corrected;
compiler-only verification and separately receipted runtime retry are pending.
This failure is retained infrastructure evidence, not an expected semantic RED.

## Verified caller facts for the pending integration

The current HTTP start uses `body.plan_id` as the session correlation ID; it
does not allocate a separate download counter after connecting. It admits
history before the tunnel but does not yet record a download lifecycle row.
The worker's keeper records the backup path only after `write_backup` returns;
the pending intent hook belongs after that success, not in a progress observer.
The worker publishes its returned `DownloadStatus` before awaiting disconnect,
whose result is currently ignored. Journal integration must preserve this
result independently and observe the adapter return without promising a KNX
acknowledgment. The existing `Written::Partially` failure classification is
conservative, derived from started steps, not a count or proof of delivered
property writes. Metadata must not promote it to independently verified effects.
These are source observations, not new guard/worker implementation or approval.

## Initial start tracer bullet — 2026-10-03

The earlier caller observations above describe the pre-integration source.
The first actual missing-start RED subsequently compiled and failed; a separate
GREEN passed one selected offline HTTP simulator test with a fresh server build,
both shared leases and two unchanged original inputs. A row-owning guard records
start before acquisition and stays with the worker, outside the one-shot ring.
That one test does not prove intent, terminal, cleanup or fault transitions.
Unproved worker hooks from the first candidate were removed before the next
vertical slice; minimum start ownership/conservative drop remains. The nonempty
joined-worker terminal receipt regression subsequently compiled and failed on
the expected missing terminal record (0/1/0). Minimal worker hooks followed
only that RED; actual terminal GREEN passed one selected offline simulator HTTP
test with a server rebuild, both leases and unchanged inputs. This positive case
does not establish intent-error, abort, crash or cleanup-failure coverage; existing
regressions on that terminal source subsequently passed15 OneShotLog,8 worker
and1 start test. The later terminal monotonicity guard passed16 OneShotLog and1
terminal HTTP test on its own source; complete fault acceptance remains pending.
This ADR remains Proposed
and the API continues to disclose partial
coverage. No bus/hardware/ETS, power-loss or complete recovery acceptance follows.

## Alternatives considered

- Reuse the one-shot ring for downloads: blurs existing live-session ownership
  and bounded ring semantics; retain a separate guard sharing the history writer.
- Keep format 1 while extending its closed vocabulary: an old reader could
  misinterpret the new contract; choose explicit evolution with legacy retention.
- Rewrite old documents into invented complete receipts: loses provenance and
  fabricates observations; preserve their bytes instead.
- Enable WAL or infer cleanup from a finished result: neither follows from the
  admitted storage/protocol contract; keep these paths unsupported/unknown.

## Consequences and acceptance boundary

Implementation proceeds test-first: synchronization policy, exact migration,
strict metadata/guard transitions, worker integration, HTTP refusal/readback,
crash/interruption and mutation regressions, then full integration gates.
This proposed ADR alone supplies no implementation or acceptance evidence.

The history API remains partial and payload-free. Web adoption/manual client
matching belongs to the current Web owner; no generated/locked frontend files
are edited. Other uninstrumented callers, complete audit coverage, retention,
forensic guarantees, genuine power-loss recovery, hardware/vendor/ETS evidence
and release acceptance remain separate open work.
