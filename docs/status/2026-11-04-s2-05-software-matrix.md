# S2-05 software matrix consolidation

**Status: focused execution, independent source review and two discriminating
calibrations complete; integrated package acceptance pending.** This record
consolidates the finite E1–E17 software obligations without relabeling historical
caption-only, persistence, target/revision, failed-terminal or recovery evidence.
Authors changed source only; the controller performed the executions below.

Scope is the existing literal-caption settlement class. General equal-plan source
equivalence remains refused. `PlanRecord` does not capture all source ownership,
coverage, interpretation or hardware-policy premises; no exhaustive invariant
establishes that arbitrary omitted metadata is neutral. A wholly self-consistent
fabricated original cannot be distinguished from execution by hashing: authority
still belongs to the actual worker's paired original admission and terminal store.
No source comparison produces start or recovery authority.

## E1–E17 disposition

Paths in the table are relative to the existing App
`tests/it/system2_validation_preparation` and server
`device_system2/tests` modules. Existing tests are reused; the actual executed
populations and failed attempts are recorded separately below.

| Design obligation | Existing proof reused and executed extension |
| --- | --- |
| E1 strict baseline | `configured_hardware/caption_settlement`: strict-success P0/P1 and failed-original-pin controls retained; pretty/reordered original profile and distinct raw container hashes now cover namespaces 20 and 23. |
| E2 standalone | Existing namespace20/23 caption positives retain strict-start refusal, ALL mask and exact original bytes; mounted initial pending/programmed populations extended. |
| E3 retained | Existing independent-source namespace20/23 positives remain separate from standalone discovery; the same mounted belief extension applies. |
| E4 profile/pins | New `caption_settlement/adversaries` changes every profile field, selector family and namespace, plus unknown/duplicate JSON. Outer profile/source/hardware digests are refreshed deliberately; original identity and parser rules must still refuse. |
| E5 original integrity | New rehashed request, snapshot, source-original, coverage and normalized-operation adversaries complement existing simulator, device-id, serial and policy refusals. The changed snapshot has freshly measured snapshot and semantic hashes but stale request/plan, forcing reconstruction refusal. |
| E6 owners | Existing scanner/parse/occurrence/path controls remain; new missing winning rows, source blobs, source occurrences and corrupt originals assert refusal without writes or repair. Legitimate retained losers remain in the replacement fixture. |
| E7 packages | Existing ambiguous containers, bad second candidate, orphan local bytes, physical inventory and ZIP security tests are reused. New missing, repathed and aliased membership, missing package and corrupt measured blob cases reach the full App caller. |
| E8 literal class | Existing ProductDB decoy/comment/PI/qualified/duplicate/entity/DTD/budget tests and App lexical refusals remain authoritative; no XML normalization or wider caption policy is added. |
| E9 plan/policy | New rehashed load-control/task/allocation, memory byte/address/object, mask/manufacturer/hardware, source-index and tool-metadata adversaries. An explicitly newly pinned preview proves equal plan values/bytes with different parameter coverage; its full-caller refusal is lexical. New private owning unit tests isolate the post-preparation record guard with valid equal-plan records differing only in coverage after the four provenance allowances, plus a caption/provenance-positive neighbor. Newly pinned unsupported Options still fail full preparation. |
| E10 current request | Existing exact dependency-mask/ref controls retained; new unrelated-device control checks actual request equality, parameter edits check actual request inequality and selective mask, unreadable/empty flags fail request derivation and settlement. Both namespaces/families covered. |
| E11 authority | Existing mounted `recovery_authority` and `wrong_serial` keep original backup/phrases, actual restore and no-write serial refusal. New previews remain simulated; no admission conversion is added. |
| E12 terminal | Existing first-write barrier and joined hardware-kind worker reused. Both initial beliefs now cover all four family/namespace positives. The test observer is installed before worker start to retain its completion handle; only the actual terminal callback earns counts. Original start admission, observed bundle and recovery remain exact. |
| E13 source/current | Existing runner expanded across both namespaces/families for links, flags, nonclass and poisoned owner; new unsupported-source/unreadable-current-flag crosses include pending/programmed beliefs. Saved selective/refused history also covers namespace23. |
| E14 lifecycle | Existing command-bound address/revision and server-incarnation decision tests retained. New actual-worker project replacement with reused numeric id, real deletion, address reuse by another id, plus subsequent select/status/report/reopen assert no source reads or project/history mutation. |
| E15 persistence | Existing injected saved-history abort/control matrix reused unchanged. Saved success, parameter and reference cases extend to both namespaces; volatile receipts and working-history reopening remain distinct from download facts. |
| E16 ineligible ends | Existing actual failed/healthy terminal matrix now counts actual worker callbacks/leases/proofs: zero for failed, one for healthy. New actual cancellation preserves conservative MEMORY/history behavior and skips proof. Its full current and undo/redo snapshots now compare against exact before snapshots with only MEMORY claims cleared, including a retained nonempty download timestamp and address/medium/configuration. Existing recovery, pre-barrier, unexpected-end and typed-error controls remain separate. |
| E17 contention/limits | New `caption_mounted/contention` holds a real managed package mutation against the actual terminal lease while a real domain command contends on the project. It awaits a failed project-lock attempt inside the actual domain command, then asserts one lease/proof, the same candidate identity, selective mask or ambiguity refusal, exact configuration/history, complete captured recovery and unchanged read counts on GET/report/select. The executed intentional-assertion control requires owned cleanup before storage retirement. The changed-source 5,001-device measurement and retained-snapshot expansion refusal preserve all ceilings. |

## Causal observation and measurement contract

`device_system2/settlement_observation.rs` exists only under `cfg(test)`, separately
from the transport observation module. It records the real terminal closure's
callback, catalogue lease attempt/acquisition, App proof completion and duration.
It accepts no test callback, never pauses the worker, holds no database handle and
has no production field or behavior switch. It also retains the real child completion
handle for the cleanup oracle; cancellation stays cooperative and the existing
supervisor still joins that child. Counters describe **catalogue read
leases and App proofs**, not individual SQL statements or device reads.

The contention test first replaces the catalogue through the managed restore
adapter after original admission and the first data write. A managed package
writer then holds the owner mutex. The actual worker reaches its terminal lease
attempt while holding the project and command-stack candidate. The test observes
both locks as contended and starts a real revision-bound edit on another thread.
A `cfg(test)` branch at `domain::apply_bound_prepared` signals only after that
command's own `try_lock` returns `WouldBlock`, then follows the normal blocking
lock path. The test awaits this event and checks its exact binding/command before
releasing the writer. This proves a genuine lock miss inside the command; it does
not infer that the next blocking system call was already scheduled. Callback and
comparison observations record the same borrowed candidate identity. Releasing the writer admits either one changed-caption container or two actual
containers with identical members. Standalone must refuse the ambiguous case;
retained mode must continue using its own closure. The successful case credits
Address/Application only after a real parameter command. The waiting project
command must see a stale revision after settlement and publish nothing. Both
contention and lifecycle assertion futures catch unwind, release hardware/writer
barriers, join owned writer/editor threads off the runtime, and finish or cooperatively cancel
and join the real service worker before fixture retirement. Keeping the worker
future owned also keeps its blocking settlement job awaited; cleanup does not abort
an async parent and detach a pending blocking settlement. A single existing-size 180-second case deadline also bounds the cleanup wait;
failure does not restart a second timed grace period. A deadline failure
remains a failure, never permission to detach a handle; the first assertion panic
is propagated after cleanup, with secondary cleanup errors kept separate.

Both paths capture the complete original `RecoveryRecord` from the committed
journal chain at the real first-write pause, reusing the original-recovery proof's
reader. Complete record equality is checked after settlement, status/report and
select/reopen; conservation is not inferred from `original_bundle` alone.

All owner/project/stack work is synchronous; channel waits run on independent
threads, never across an await with these locks held. The managed lease encloses
original-source discovery/comparison and current preparation on the same locked
candidate. This does not promise isolation from unmanaged external SQLite
writers. The observation measures the callback's lease wait and proof duration;
it is not a general SQL or scheduling profiler.

`E17_CONTENTION` output records actual callback/attempt/acquired-read/proof counts,
genuine command lock-miss count, same-candidate check, lease wait, proof duration and contending command duration. `E17_LARGE` records
selector, namespace, 5,001-device input, actual retained snapshot/bundle sizes and
three samples of strict refusal versus changed-caption proof. Actual values
from the retained executable runs are recorded below, not inferred from source.
There is no latency threshold or raised timeout. The over-budget current project
uses a 13 MiB synthetic description, independently checks that the raw snapshot
is below 16 MiB but its canonical base64 expansion exceeds the retained-byte
ceiling, and requires the exact `retained byte limit` refusal. It checks that
neither input nor catalogue nor original evidence was trimmed. This is the
retained snapshot expansion boundary, not a later aggregate recovery-envelope
measurement. The existing 16 MiB ceilings remain unchanged.

## Independent review and corrections

I1 isolates the existing record/plan guard without changing original validation,
the classifier or the caller's Core-plan/derived-field checks. Owning unit controls
cover the valid coverage-only difference, every other record field, exact coverage
order/fields and plan values/order, with provenance-positive controls. This is
defense in depth, not a claim that the narrow caption class naturally changes
coverage. I2 moves contention observation inside the actual command's project-lock
attempt. I3 adds unconditional owned cleanup and an intentional assertion-failure
control. M1 corrects report formats 3/4 in the maintained contract; M2 strengthens
exact cancellation history/timestamp and full recovery-record conservation.
Independent whole-feature review first blocked I1–I3; finding-specific re-review
approved all three and M1–M2. Subsequent controller corrections are test-only:
require the exact retained-byte boundary with independent raw/base64 size checks,
and remove fixture parameter dependencies through validated undoable commands
before the real device deletion. No product deletion rule or ceiling changed.

## Controller execution and failed attempts

All focused receipts have direct exits, nonempty compiled inventories, unchanged
consumed source fingerprints and checked/untouched real user-data snapshots.
Runs use owned targets, explicit scratch XDG homes with Cargo overrides,
offline locked dependencies, at most two build jobs, low priority, the shared
workspace lease, loopback-only network isolation and dropped DAC bypass.

- App owning comparison/source-phase units: **6 passed**.
- App default caption population: the first complete run had **28 passed / 1
  failed**. The failing test expected a later envelope diagnostic, but the actual
  serializer refused canonical retained-byte expansion first. The corrected
  exact-boundary test passed **1/1** for both selector families. This is not a
  relabeled all-green 29-test run; the final package gate must run the defaults.
- Mounted caption plus settlement-decision population: **53 passed / 2 failed**.
  Both failures were lifecycle fixture setup: `DeleteDevice` correctly refused
  dependent parameter data. After removing that data through validated undoable
  inverse commands, the two lifecycle leaves passed **2/2**, completing exactly
  **32 unique family/namespace/end/belief cells**. The earlier failed run remains
  failed. The complete default server suite is still required in final acceptance.
- The original mounted run completed all **8 unique contention cells**, including
  actual command lock misses, selective or ambiguous-source refusal masks,
  immutable metadata/full backup, exact history and read-only reattachment.
  The deliberate assertion-failure cleanup control passed after actual thread,
  supervisor and child completion checks. Expected caught panic-hook text is not
  confused with an uncaught test failure.
- ProductDB literal-caption and ZIP-inventory controls: **32 passed**.
- The opt-in changed-caption measurement: **1 passed**, six complete samples.
  These overlapping focused, matrix-cell and later package populations are not summed.

The first App compile failed on an incorrectly matched nested error type; it ran
zero tests. After correction, another compiled attempt was intentionally stopped
for review fixes and is unaccepted. Both failed compiled runs above and the
unstarted calibration prerequisite refusal are retained separately.

### Discriminating compiled controls

In an exact isolated source copy, omitting only the inner record equality made
`record_guard_refuses_valid_equal_plan_coverage_only_difference` fail at its
own refusal assertion; its caption/provenance-positive neighbor still passed.
Enabling the actual current-parts callback for an ineligible terminal download
made the final-version-mismatch worker fail its exact counter assertion:
`(1,1,1,1)` instead of `(0,0,0,0)`; its healthy worker neighbor still passed.
Both variants compiled successfully and exited 101 only at their intended tests.
Both two-test controls passed before mutation and again after exact restoration.
The candidate and mutation-copy fingerprints were read back equal to their
original frozen binding. No compile error or zero-test run is counted as a kill.

### Actual measurements

The changed-caption App measurement uses namespace 23 and **5,001 synthetic
devices**, with three samples per selector. Each snapshot is **868,352 bytes**.

| Selector | Original bundle bytes | Strict changed-pin refusal (seconds) | Complete changed-caption comparison (seconds) |
| --- | ---: | ---: | ---: |
| Standalone | 1,173,109 | 0.372730–0.377712 | 5.448010–5.538030 |
| Retained members | 1,171,676 | 0.369627–0.371955 | 5.413092–5.470053 |

In the eight mounted contention cells, measured managed-lease waits were
**15,004–21,148 microseconds**, proof duration **312,156–1,495,511 microseconds**,
and contending project-command duration **333,112–1,516,558 microseconds**.
Every cell recorded one callback, one lease attempt/acquisition, one proof,
one genuine in-command lock miss and the same borrowed candidate identity.
These are debug-build observations, not a latency SLA, speedup comparison,
individual SQL-statement count or physical-device measurement. Strict refusal
and successful complete comparison perform different contracts.

## Integrated acceptance and retained limits

The committed main-first candidate still requires the complete maintained
change-scoped package gate, including its selected default Rust and corpus
populations and real web-bundle prerequisite. No release/full-workspace or new
browser/native/hardware acceptance is inferred from the focused runs.
Evidence after retention: `<profile>/evidence/2026-11-04-s2-05-software/`.

Unreachable lifecycle combinations are not enabled by test switches: a running
server's incarnation and catalogue-owner option are immutable, and start requires
a present healthy owner. Historical server/absent-owner boundaries retain their
existing owner/decision tests. Process crashes promise no terminal settlement.
Field retry, exact firmware-clear time, physical recovery and broader source
metadata equivalence remain evidence-limited and unauthorised here. No bus,
hardware, simulator semantic change, timeout/budget increase, journal migration,
release or deployment is part of this work.

Related contracts: [settlement](../contracts/system2-download-result.md#settlement),
[preparation](../contracts/project-system2-offline-preparation.md),
[design and E-matrix](../design/system2-settlement-source-equivalence.md).
