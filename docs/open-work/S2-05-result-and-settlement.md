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
  adversaries, broader source-positive recovery crosses, race/read-count instrumentation
  and changed-source large-project measurements remain open. A bounded
  [E15 changed-caption persistence-failure follow-up](../status/2026-11-03-s2-caption-persistence.md)
  is accepted main-first as `99f7e65dd`: focused mounted execution, independent
  review with four compiled wrong-variant assertion failures and both complete
  selected 14-stage gates. This closes that bounded failure cross, not all E15.
  The [E13 malformed-current-parameter follow-up](../status/2026-11-04-s2-current-parameter-refusal.md)
  has focused mounted proof for both selector families/namespaces and prior
  pending/marked states plus independent terminal-refusal and address-fallback
  calibration, accepted main-first as `37fcc9b50` after both complete selected
  14-stage gates. This closes that bounded current-input cross, not all E13.
  The [E14 command-bound current-target/revision follow-up](../status/2026-11-04-s2-current-target-revision.md)
  is accepted main-first as `fb1facc41`: four new leaves/16 mounted cells across
  both families/namespaces and pending/marked beliefs, exact post-command
  preservation, a genuine revision-only positive control, independent four-variant
  assertion calibration and both complete selected 14-stage gates. The callback
  counter is a separately labelled private seam probe, not worker read-count
  evidence. This closes that bounded cross, not all E14 or lifecycle/recovery.
  The [E16 ineligible final-verification follow-up](../status/2026-11-04-s2-ineligible-terminal.md)
  is accepted main-first as `633d669e3`: two new leaves/four retained/ns20 cases
  cross a healthy caption-ALL witness with actual failed/healthy terminal workers,
  exact MEMORY/history conservation and frozen facts. Independent review calibrated
  two real owner variants; both complete selected 14-stage gates passed. Version
  observations use current-producer deviation-only semantics, not retained raw
  bytes or actual-worker read counts. This closes this bounded failure cross,
  not all E16 or S2-05. The separately [extended final-verification matrix](../status/2026-11-04-s2-ineligible-terminal-matrix.md)
  covers both source families and namespaces 20/23, failed/healthy workers and
  pending/marked beliefs (eight leaves/16 cases), published main-first as
  `446480584` after independent correction review and both selected 14-stage
  gates. The bounded [caption-positive original recovery proof](../status/2026-11-04-s2-caption-recovery-authority.md)
  is published main-first as `77edfc3b3`: four standalone/retained20/23 mounted
  leaves check actual original recovery, exact OLD model restoration, original
  evidence/confirmation authority and conservative MEMORY/history settlement.
  Independent review closed a redo-oracle gap; both selected 14-stage gates passed.
  This closes that finite cross, not all E11/E16. The separate
  [wrong-observed-serial recovery proof](../status/2026-11-04-s2-caption-recovery-serial.md)
  is published main-first as `18ae7406b`: four mounted source/namespace leaves
  preserve the existing no-write refusal, original Verified facts and conservative
  history settlement; four valid-original neighbors remain. Independent source
  review/correction closure, one compiled guard-bypass calibration with four intended
  failures/four valid neighbors, restored eight-leaf control and both selected
  14-stage gates passed. This closes only that wrong-actual-serial cross. Wider
  terminal/lifecycle/recovery, cross-bound journals and race/read-count obligations
  remain separate and open.
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
