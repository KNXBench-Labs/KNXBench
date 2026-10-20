# Bounded UP258 System-2 validation path

20 October 2026; agent: codex.

## Scope and ownership

The owner requested a complete bounded path for UP258D12 application
`M-0001_A-9205-04-9270`: image, all ordered source instructions, existing
executor, durable operation-specific recovery and simulator fault acceptance.
A later clarification requires publication as a **strong hypothesis, tested in
the simulator**, followed by a short reporter-run validation request in issue
#22 with persistent protocol capture. No local bus/device operation is approved.
The later clarification does not authorize omitting an indispensable unknown.

An isolated feature worktree starts from published `8d720a95`. The selected root
is intentionally stale/dirty and remains untouched except for an additive shared
claim. WP-04.2 resolver files and its queued WP-04.3/WP-19 work remain reserved
by the other session. Root-only untracked research is not incorporated.

## Independently inspected sources

The research note
`docs/research/2026-10-20-system2-validation-rules.md` distinguishes actual
retrieval/visual inspection from snippets, failed retrievals and candidate
interpretations. Mirrored historical KNX-authored schema pages and a separately
retrieved revision-bound namespace-20 schema are not presented as newly
authenticated official distributions. Original authorized Management Procedures
pages constrain wire encodings; they do not magically define modern XML policies.
No supplied research wording is reused.

## Implementation checkpoints

- Existing strict System-2 image and ordinary memory-download admission remain
  separate from the experimental candidate entry.
- The source-procedure adapter retains every source index, emits explicit zero-wire
  tool metadata and expands allocations into protected-mask-aware data intervals.
  Its policy names the allocation-count, checksum-flag and table-identity
  hypotheses explicitly; this is not hardware acceptance.
- Parent focused procedure tests currently pass 3/3. The first attempted original
  run encountered another worker's transient compilation error and is retained
  as setup failure, not RED. An immutable original-source snapshot then reached
  the intended missing-module/API diagnostics. Final tests/gates and whole-path
  acceptance remain pending.
- Net recovery/execution, experimental image semantics and App/CLI persistence
  are assigned disjoint implementation surfaces. Independent source research and
  integration inventory are read-only workstreams. Their claims are not accepted
  until inspected and exercised by the parent.

## Integration checkpoint, 20 October 2026

The independent official package read reconfirms the precise mask footprint:
1,514 allocation bytes, 1,176 writable bytes and 338 protected bytes under the
current explicit mask interpretation. Consequently a generic executor that
requires full direct write coverage cannot admit this exact target physically.
The guard is not weakened. Simulator recovery must inspect every allocated
byte and model allocations/task state rather than pass merely because memory
happens to survive Unload.

A versioned external data profile and the App source-preparation service now
connect the actual product image builder and source translator to the immutable
simulator bundle. The bounded configuration is complete product defaults with
no project overrides or group links. Preparation's original-source missing-API
failure is retained; integrated execution is still pending worker freeze. An
intermediate shared-tree compile failure was another worker's incomplete edit,
not a preparation regression result. No source/simulator acceptance is inferred
from the prepared JSON or a proposed CLI command.

The `npm ci --prefer-offline` prerequisite finished with exit 0 (63 packages,
zero reported vulnerabilities). This is dependency installation, not a test gate.

## Remaining delivery boundary

Finish the actual source-bound integrated artifact, exercise success plus
identity/unsupported/persistence/partial-failure/cancellation/reopened recovery
and recovery-failure cases, independently review, freeze and run relevant gates
in one final chain where practical. Update status/compatibility/contracts and
handover from real results, then focused KNXBench-authored commits and verified
publication. Only after publication may the reporter be asked to use the actual
executable workflow; no request for an unavailable or unsafe write command.
The issue remains open until target transfer/readback and human function checks
supply independent evidence. No automatic Verified promotion.

## Integrated artifact and independent final review

The parent executed the actual pinned public package through production ingest,
read-only complete preparation, CLI preview, simulator download and recovery in a
second CLI process. All six raw stage receipts (including expected physical
refusal exit1) are private; no fabricated transfer output is substituted.
The independent OLD typed fixture separately declares extents, attributes,
pointers and task identity/control metadata. Seven initial source-bound tests
passed, with four deterministic crash/cancel cuts in one of those tests; a later
failed/cancelled-recovery regression joins the final acceptance inventory.

Independent image/preparation/CLI review found no blocking defect. Its minor
missing reference-activation policy mutation test was added with empty/unknown
cases and unchanged-database assertions. A one-sentence header correction is
metadata-only. The Net/App reviewer independently demonstrated two real defects:
ordinary hardware authorization bypassed source/recovery admission for synthetic
complete-write plans, and final recovery could claim success with missing PID13.
Both intended refusal regressions first failed semantically (12pass/2fail), then
passed with the fixes (14pass). Final focused independent re-review ACCEPTS both:
25tests pass including hardware-kind execute/restore full-write/task-only zero-
frame controls and durable RestoreFailed for missing/short/different identities.
No physical target was used by either reviewer.

All Hardware-kind experimental execution now refuses before session/callback.
Mutable application identity may be absent at recovery start, but the captured
identity must exist again after replay; no guessed PID13 write is invented.
Masked/unwritten full-allocation verification remains mandatory. Hardware
approval is not a substitute for missing real masked-byte/resource rules.

## Final combined acceptance

At frozen `1d761840` the entire final chain exits0. Maintained package receipt:
complete/success/planCompleted/inputsFrozen all true, 10/10 intended stages, each
exit0 and proofValid; nested full web receipt69/69 with the same complete verdict.
Private corpus155/155 in9targets. Ordinary Rust summaries5578pass/222ignored;
these are run summaries, not a deduplicated all-tests count. Explicit exact-source
App8/8 and experimental-image16/16 include the actual pinned public package with
zero ignored tests. Rebuilt CLI workflow returned finished/restored, and the
separate expected physical-refusal exit1 occurs before socket/operation creation.
All tracked inputs still equal immutable start/end manifests, HEAD unchanged,
worktree clean; protected user-data touch/unchecked lists empty.

The first complete attempt failed before product execution on an upstream test
fixture that reacquired the checkout lease. Separate reviewed fixture correction
`1d761840` and single-reference Clippy correction `1d1bf2fc` precede the successful
full rerun. Failed and successful receipts remain distinct.

The delivery `54843a03` was pushed to main and independently read back from the
live remote ref, matching the clean publication checkout. The issue22 reporter
was invited to run the exact simulator-only executable workflow in comment
`6299113198`; posted body and issue-open state were read back and verified.
The invitation says strong hypothesis/simulator-tested, no real-device command,
retains private logs/backups locally and requests only reviewed shareable reports.
No release, tag, deployment or bus/device operation occurred. Final evidence
archival and owned scratch/worktree retirement are housekeeping, not new runtime
acceptance or physical readiness.
