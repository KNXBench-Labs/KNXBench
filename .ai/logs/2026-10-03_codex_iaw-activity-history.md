# Commissioning activity metadata / Alpha source-ID reconciliation

## Candidate checkpoint — 2026-10-03 00:52 CEST

Runtime: gpt-6.1-sol / openai-codex; implementation and separate review in-session.
Base: c9f77d7b (published recovery package). Current upstream: be88e7b9.

- Separate version-1 SQLite activity metadata, never project/vendor/recovery data.
- Four one-shot kinds; seven untracked kinds explicitly disclosed.
- Strict identities/vocabulary/bounds and no silent replacement after admission.
- Observed storage/read/record failure latches unavailable and refuses reachable
  commissioning write starts before tunnelling; property intent follows recovery.
- Concurrency regression: first initialization serialized under a mutex.
- Final mutation sweep: all 19 rerun on the current mutex-fixed source, each
  compiled and failed behaviorally; two separate initialization/download guards
  also failed as expected. Exact source restoration checked.
- Private HTTP download: 14 registered/14 passed/0 failed/0 ignored; all 108
  original inputs unchanged. In-process SimTunnel only; no raw output retained.
- Earlier review's zero findings were overtaken by late storage review:
  Important 1, SQLite writable reads recovered a foreign hot journal before
  refusal (synthetic main-file change and journal deletion reproduced).
  Read-only admission plus pre-SQLite rollback-mode header guard close this
  mechanism, including WAL sidecar creation; two explicit RED/GREEN regressions
  cover both openers. Corrected-source separate review found no remaining
  blocking finding. No Web/native/hardware/ETS approval inferred.
- All 21 current-source compiled behavioral mutants failed as expected;
  exact restoration and post-mutation store 6/0/0, log 12/0/0, HTTP 22/0/0
  independently reconciled. Focused strict all-target Clippy also passed.
- Earlier full candidate: 18 steps, workspace 2948/0/165, Web1559,
  download14/0/0, Dynamic6/0/0, 108 originals unchanged. Initial wrong target
  orchestration failure was preserved and only missing registered scope run.
  This receipt describes the older source, not current integrated acceptance.
- All 42 ledger IDs match current readiness routing, zero duplicates/missing/
  extras. Direct Profile audit complete; unsupported mappings remain refused.
- Corrected candidate accepted 18/18 below; integration and publication remain PENDING.

## Active corrected candidate — 2026-10-03 01:46 CEST

Process `proc_9d5dc749e4f4`, still running; no acceptance inferred.
Recorded completed steps: 15. Workspace 2950/0/165,
Web 1559, private HTTP download 14/0/0; all completed
step exits 0. Private Dynamic, final bindings/frozen checks and integrated
publication remain pending. Current source includes the hot-journal/WAL fix.

## Delayed notifications reconciled — 2026-10-03 01:49 CEST

Batch: proc_92931979d66b exit 0 is the earlier 19-mutant sweep; actual
compiled behavioral failures and exact restoration independently confirmed.
proc_4f9e7e129e3f exit 1 remains a recorded orchestration failure:
http_parameter_panel had no registered ignored tests. The original traceback
is archived; missing actual Dynamic scope was separately executed and passed,
not relabelled. Corrected-source 21-mutant sweep supersedes the older source.
Current full corrected gate proc_9d5dc749e4f4 is still running, not accepted;
integration and publication remain pending. No duplicate gate was launched.

## Integration preflight — 2026-10-03 01:57 CEST

Fetched/read-back origin/main `be88e7b90cd622e921e675d4d58d01029ed22258`; four upstream commits
introduce the Float guard and receipts. Overlap includes domain.rs and shared
handover/status/limitations; HTTP parameter tests are part of integrated scope.
The read-only merge-tree used committed base c9f77d7b, not the uncommitted
runtime candidate, and supplies no candidate conflict-free approval. Shared
root and all foreign worktrees stayed untouched. Full corrected gate remains
active; Dynamic executable was CPU-active, not assumed hung from log silence.

## Corrected candidate acceptance and notification batch — 2026-10-03 02:02 CEST

proc_9d5dc749e4f4 exit0 and independent18/18 acceptance: workspace2950/0/165,
Web1559 in89 files, private download14/0/0 and Dynamic6/0/0, no unknown skips,
108 originals unchanged,608 frozen source/config inputs and17 shadow bindings
equal. Runtime diff exactly matches the separate corrected-source review.
Repository gates name the actual owned worktree and nonempty coverage.
Integration with published Float guard and publication are still PENDING.

Delayed batch: proc_35c99f3513cc historical gate reports13 steps and gate_exit0;
proc_9eccb7b570d8 offline download reports14 registered/passed, no skips and
108 originals unchanged. proc_c38e6e987bc9 exit1 failed at exclusive flock
before any integrated tests. This failed attempt stays recorded and is not
integration approval; no competing gate is bypassed or force-unlocked.

## Current documentation-gate lease refusal — 2026-10-03 02:08 CEST

Final documentation recheck did not start: exclusive lock unavailable.
/proc/locks identifies PID1408878, ui-theme-storage/integrated_gate.py, holding
both common leases. No bypass, unlock, competing build or foreign edit was
attempted. Corrected candidate18/18 stays accepted for its frozen source;
local source checkpoint is not final doc/integrated acceptance or publication.
Added runtime-line security scan and whole-owned-scope private-identity scan
are clear. Final acceptance waits for the cooperating gate lease.

## Local source and integration preparation — 2026-10-03 02:15 CEST

Reviewed source `0fc483c26cb73525fc4041bc74b5f953c350bc43` committed with exact22 owned files, required
author/committer and no co-author; tree/readback verified. Not published.
Actual candidate merge with `be88e7b90cd622e921e675d4d58d01029ed22258` had two doc conflicts, no source
conflicts. Both own entries and complete authoritative upstream handover/status
bodies preserved; handover truncation negative control rejects archive loss.
Five history/guard sources equal their owner byte-for-byte; parameter tests,
parameter boundary/goal and upstream log equal their published owner.
Integrated domain overlap reviewed separately; actual merged gate is PENDING,
as are final doc acceptance and publication under the common gate lease.

## Residue

Durable long-session journals, Web/global status/scope adoption, complete
physical-device recovery and independent vendor/device evidence remain open.
The controller still owns final whole-product review, statistics and release.
Shared dirty root, U16 lock and foreign worktrees were not edited.
