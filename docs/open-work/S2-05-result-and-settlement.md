# S2-05 — System-2: result in the device list, report, project settlement

**Priority:** P1. **Status:** bounded result/report and automatic project-belief
integration accepted on main-first `8c06ddc96`, including supervised terminal
settlement and application-level status/reload ownership. Feature and integrated
selected gates15/15 each passed; #30's allocation mismatch remains unresolved.
Origin: GitHub issue #31.

[Acceptance, counts and boundaries](../status/2026-10-28-system2-result-integration.md).

Contract: [system2-download-result](../contracts/system2-download-result.md).

## Open

- Splitting "final step did not finish" into verification mismatch, read failure and final
  write failure is implemented in source (see the dated note below); which cause lies behind
  the allocation mismatch in #30/#22 was not observed directly. The reporter's snapshots after
  the KNXBench download (2 November 2026) show the firmware clearing the hidden `Download_Flag`
  octet itself ([research](../research/2026-11-01-up258-post-load-flag-octet.md)). Source since
  2 November 2026: a revision-3 UP258 preparation declares that octet, journals and shows each
  tolerated value; every other byte stays strict. Open: a field run of a build that contains it
  (not requested), and the exact clear time (Load Completed or restart).
- General per-part source equivalence stays open. Since main-first `0d6299187`
  (2 November 2026) a verified run on a proven
  unchanged source credits the parts the current project still states as written.
  Independent re-review and both selected 14-stage package gates passed
  ([acceptance](../status/2026-11-02-system2-selective-settlement.md)); no hardware run.
  The [3 November caption-only witness](../research/2026-11-03-s2-caption-source-equivalence.md)
  distinguishes old pinned-profile refusal before plan comparison from equal plans
  with an explicitly new source-and-package-pinned profile. No settlement-only
  re-pinning contract or production equivalence policy is implemented by that evidence.
  The explicitly software-authorized [narrow caption-only rule](../status/2026-11-03-s2-caption-settlement-candidate.md)
  is accepted main-first as `288bd8005` after independent review and both complete
  selected 14-stage gates. It implements the bounded [settlement-only design](../design/system2-settlement-source-equivalence.md),
  not general source equivalence or new start/recovery authority. Exhaustive E-matrix
  adversaries, source-positive recovery/persistence-failure crosses, race/read-count
  instrumentation and changed-source large-project measurements remain open.
- Field verification by the reporter is not requested; no hardware retry is needed
  for the software change.

## Update 28 October 2026: final-step failure category (source, focused proof)

Net classifies a failure of the last `Disconnect` step's own operations from the typed
operation in flight; App journals it (envelope version 3, closed `final_step` record, strict
replay), reopen and the reports carry it, status schema 4 serves `finalStepFailures`, and
the panel shows it in EN and DE with an explicit "unavailable" for older journals. Scope,
grammar and limits: [contract](../contracts/system2-download-result.md#final-step-failure).
Proven by focused Net, App, server and web runs, both 15-stage package gates and the
isolated built-server smoke. Source `96f41c7f1` is delivered main-first as `309384726`;
the docs-only upstream delta carries the product acceptance from `8bdc42c7e`, not a new
product run. [Acceptance and exact boundaries](../status/2026-10-28-system2-final-step-failure.md#software-acceptance-and-source-delivery).
No hardware evidence exists.
