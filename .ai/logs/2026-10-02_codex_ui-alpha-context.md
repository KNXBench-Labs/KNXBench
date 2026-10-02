# UI alpha context — KL-82 follow-up (2026-10-02)

## Scope and authority

User requested goal-ui rows from docs/ALPHA_READINESS.md. Four previous gaps
were already published at 6c16fe5a with correction 474c55e4. This package owns
only the authoritative monitor-context follow-up in isolated ui-alpha-context,
reservation 79dc56dd. Root foreign work, corpus originals and other tracks are
untouched. No hardware, productive discovery/tunnel, real backend, release tag,
reserved ADR activation, quota probe or subagent is authorized.

## Actual implementation and boundary matrix

| State | Server / HTTP | UI / fixture |
| --- | --- | --- |
| current | Compare exact actual session style/name/resolved-DPT snapshot with current project; opaque incarnation and projectOpen | May establish freshness without local browser record; actual no-project state displayed |
| stale | Renamed or DPT-edited project differs; style command's actual replacement can restore equivalence | Lock compose; local storage only invalidates, never establishes proof |
| unavailable | Busy project lock or busy/poisoned interpretation snapshot never becomes current | Missing/malformed/legacy response or poll failure disables compose, including explicit DPT |
| paused | context_only returns no rows and retains requested cursor | Continue context/status checks; preserve captured rows and cursor; Resume catches up |
| late/replaced | Numeric ID plus server incarnation distinguish session | Generation/identity guards reject delayed current polls and obsolete reattach failures |

No project names/paths are projected by these fields. This is an interpretation
snapshot comparison, not complete project identity, collaboration/push, historic
row reinterpretation, write authorization or transactional send-context binding.
Older servers remain readable; absent freshness evidence cannot enable Send.

## Verification already executed

- Behavioral RED/GREEN: actual server project vs browser record; no authoritative
  context; pause/cursor and late replies. Rust HTTP matrix, focused UI/compose/API
  suites and TypeScript passed before widening.
- Separate in-session diff review recorded IMPORTANT missing uncertainty coverage,
  obsolete reattach rejection, and legacy fail-closed documentation; MINOR duplicate
  setters. Fixed those findings. Deferred-rejection regression first failed and
  then passed; busy/poisoned snapshot unit and restored focused suites pass.
- Ten behavioral negative controls rejected with assertion failures, not compile
  errors: ignored names, unknown verdict rounded current, paused freshness halted,
  generation removed, poll error retained current, paused cursor advanced, paused
  rows leaked, empty incarnation accepted, obsolete reattach error accepted, busy
  snapshot rounded current. Exact source restoration checked in finally blocks.
- Mocked Chromium monitor-control fixture passed against loopback-only Vite,
  every API intercepted. No live-bus/native/Orca acceptance inferred.
- Full inherited handover suffix compared with an on-disk complete Git blob:
  byte-exact. An initial comparison with truncated terminal stdout returned False;
  it was not an archive-loss finding and is not preservation evidence.

## First complete-gate failure and correction

`proc_227baec12b91` exited 1 at Web tests: 82 files pass, one fails;
1,342 tests pass, one diagnostic-shell CSS assertion fails. No Rust/build/E2E
acceptance follows from that interrupted sequence. The real missing
`bus-compose-unverified-hint` rule now uses the existing theme warning token
with enough specificity to survive the generic hint rule. Focused RED/GREEN,
CSS-rule-removal control and restored focused UI/diagnostic suites pass.
The first restoration patch failed; the exact saved CSS segment was reapplied
and the entire stylesheet byte-compared before rerunning tests. Eleven total
behavioral controls are recorded, all original source restored. Failed logs
are retained separately under task scratch/failed-first-gate.

Renewed `proc_4258b3542021` exited 0; every one of twelve steps was read back.
Web 83 files / 1,343 tests, Chromium 35 fully intercepted cases, Rust 146 result
blocks / 2,890 passed / zero failed / 163 ignored; source freeze 570. Type/build,
strict Clippy/fmt and all four repository gates pass at the intended worktree.
No ignored private-corpus execution or native/live acceptance is inferred.
Final separate in-session source/test/contract review closes R1–R6; no external
reviewer approval is claimed. R6 only clarifies the prop fallback and removes
three duplicate comment lines after candidate acceptance; the integrated gate
must cover that precise source before publication.

## PENDING

Complete coordinated candidate gates, final full-diff sign-off, fresh-upstream
integration and repeated gates; publication/ref/artifact readback, owner receipt
and task-owned cleanup. Keyboard/modal/help-tip contracts follow separately.
Native/AT/domain/sample boundaries remain open; no alpha-ready claim.
