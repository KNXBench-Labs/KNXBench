# S2-05 — System-2: result in the device list, report, project settlement

**Priority:** P1. **Status:** bounded result/report and automatic project-belief
integration accepted on main-first `8c06ddc96` (hash mapping 2026-11-22: alpha-22 history; on current main as `cc71fe73e`), including supervised terminal
settlement and application-level status/reload ownership. Feature and integrated
selected gates15/15 each passed. #30's allocation mismatch is accounted for by the
firmware clearing the hidden `Download_Flag` octet itself (observed 2 November 2026,
[research](../research/2026-11-01-up258-post-load-flag-octet.md)); that one octet is
tolerated, every other octet is verified strictly, and the 8 November 2026 reporter run finished verified
([field run](#field-run-8-november-2026)); #30 and #31 are closed. Open: when the
firmware clears that octet, and general per-part source equivalence.
Origin: GitHub issue #31.

[Acceptance, counts and boundaries](../status/2026-10-28-system2-result-integration.md).

Contract: [system2-download-result](../contracts/system2-download-result.md).

## Software consolidation, 4 November 2026

The [E1–E17 software matrix consolidation](../status/2026-11-04-s2-05-software-matrix.md)
is accepted main-first as `cee41bf0d` (hash mapping 2026-11-22: alpha-22 history; on current main as `38babefc5`) on 5 November 2026: App integrity/ownership
adversaries, actual-worker catalogue/project contention, full lifecycle and
history conservation, changed-source large-project measurement and exact limits.
Two independently compiled wrong variants fail their owning assertions, with
healthy and restored controls. Independent source/claims reviews and the complete
selected 14-stage integrated gate passed; earlier failed attempts remain explicit.
The narrow caption class and physical/general-equivalence limits are unchanged.

## Open

- Splitting "final step did not finish" into verification mismatch, read failure and final
  write failure is implemented in source (see the dated note below); which cause lies behind
  the allocation mismatch in #30/#22 was not observed directly. The reporter's snapshots after
  the KNXBench download (2 November 2026) show the firmware clearing the hidden `Download_Flag`
  octet itself ([research](../research/2026-11-01-up258-post-load-flag-octet.md)). Source since
  2 November 2026: a revision-3 UP258 preparation declares that octet, journals and shows each
  tolerated value; every other byte stays strict. The requested field run is done
  ([field run](#field-run-8-november-2026)). Open: the exact clear time (Load Completed or
  restart); the run reads the octet back only after Load Completed, so it does not settle that.
- General per-part source equivalence stays open. Since main-first `0d6299187` (hash mapping 2026-11-22: alpha-22 history; on current main as `11e7985db`)
  (2 November 2026) a verified run on a proven
  unchanged source credits the parts the current project still states as written.
  Independent re-review and both selected 14-stage package gates passed
  ([acceptance](../status/2026-11-02-system2-selective-settlement.md)); no hardware run.
  The [3 November caption-only witness](../research/2026-11-03-s2-caption-source-equivalence.md)
  distinguishes old pinned-profile refusal before plan comparison from equal plans
  with an explicitly new source-and-package-pinned profile. No settlement-only
  re-pinning contract or production equivalence policy is implemented by that evidence.
  The explicitly software-authorized [narrow caption-only rule](../status/2026-11-03-s2-caption-settlement-candidate.md)
  is accepted main-first as `288bd8005` (hash mapping 2026-11-22: alpha-22 history; on current main as `9c15583a2`) after independent review and both complete
  selected 14-stage gates. It implements the bounded [settlement-only design](../design/system2-settlement-source-equivalence.md),
  not general source equivalence or new start/recovery authority. The later
  [software consolidation](../status/2026-11-04-s2-05-software-matrix.md) maps all
  finite E-rows to existing or newly executed controls, including actual-worker
  lease/proof counts, causal project contention and changed-source measurements;
  finite software consolidation is accepted main-first as `cee41bf0d`. It does not
  claim every possible
  adversary, individual SQL counts or broader source policy. A bounded
  [E15 changed-caption persistence-failure follow-up](../status/2026-11-03-s2-caption-persistence.md)
  is accepted main-first as `99f7e65dd` (hash mapping 2026-11-22: alpha-22 history; on current main as `b0ad55b35`): focused mounted execution, independent
  review with four compiled wrong-variant assertion failures and both complete
  selected 14-stage gates. This closes that bounded failure cross, not all E15.
  The [E13 malformed-current-parameter follow-up](../status/2026-11-04-s2-current-parameter-refusal.md)
  has focused mounted proof for both selector families/namespaces and prior
  pending/marked states plus independent terminal-refusal and address-fallback
  calibration, accepted main-first as `37fcc9b50` (hash mapping 2026-11-22: alpha-22 history; on current main as `8b59d8c8a`) after both complete selected
  14-stage gates. This closes that bounded current-input cross, not all E13.
  The [E14 command-bound current-target/revision follow-up](../status/2026-11-04-s2-current-target-revision.md)
  is accepted main-first as `fb1facc41` (hash mapping 2026-11-22: alpha-22 history; on current main as `04cc47ba3`): four new leaves/16 mounted cells across
  both families/namespaces and pending/marked beliefs, exact post-command
  preservation, a genuine revision-only positive control, independent four-variant
  assertion calibration and both complete selected 14-stage gates. The callback
  counter is a separately labelled private seam probe, not worker read-count
  evidence. This closes that bounded cross, not all E14 or lifecycle/recovery.
  The [E16 ineligible final-verification follow-up](../status/2026-11-04-s2-ineligible-terminal.md)
  is accepted main-first as `633d669e3` (hash mapping 2026-11-22: alpha-22 history; on current main as `dd85a0130`): two new leaves/four retained/ns20 cases
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
  is published main-first as `77edfc3b3` (hash mapping 2026-11-22: alpha-22 history; on current main as `05dd69639`): four standalone/retained20/23 mounted
  leaves check actual original recovery, exact OLD model restoration, original
  evidence/confirmation authority and conservative MEMORY/history settlement.
  Independent review closed a redo-oracle gap; both selected 14-stage gates passed.
  This closes that finite cross, not all E11/E16. The separate
  [wrong-observed-serial recovery proof](../status/2026-11-04-s2-caption-recovery-serial.md)
  is published main-first as `18ae7406b` (hash mapping 2026-11-22: alpha-22 history; on current main as `81da9e023`): four mounted source/namespace leaves
  preserve the existing no-write refusal, original Verified facts and conservative
  history settlement; four valid-original neighbors remain. Independent source
  review/correction closure, one compiled guard-bypass calibration with four intended
  failures/four valid neighbors, restored eight-leaf control and both selected
  14-stage gates passed. That historical package closes only its wrong-actual-serial
  cross. Later lifecycle, complete backup conservation and real-worker contention
  evidence is recorded in the software consolidation above; it does not rewrite
  these historical verdicts or authorize physical recovery.
- Field verification: done for this device and configuration, see below.

## Field run, 8 November 2026

The optional reporter run requested in
[#30](https://github.com/KNXBench-Labs/KNXBench/issues/30#issuecomment-6718382084) finished
on 8 November 2026. The reporter names build `0.1.0-alpha.19+gac8f005`
([report](https://github.com/KNXBench-Labs/KNXBench/issues/30#issuecomment-6719332657));
that revision contains the tolerance merge `046ce3fef` (the request asked for any
build containing `e3cd474c0`). The bundle identifies its exporter as that build,
but records `executionBuild: unknown`; the execution version remains a reporter
statement, not independent bundle evidence. The public reporter observations are:

- One operation on the UP 258D12 (application `M-0001_A-9205-04-9270`): finished,
  27/27 steps, `downloadResult: "verified"`, project settlement `persisted`; the
  programming marks were set, which is field evidence for the #31 display fix.
- The preview named the tolerated octet before confirmation. Tolerated line after the
  run: object 3, `3CF4h`, written `01`, observed `00`, prestate `00`. Every other octet
  was verified strictly.
- The reporter states that the exported diagnostic bundle and functional confirmation
  were sent privately, and publicly confirms normal detector function
  ([confirmation](https://github.com/KNXBench-Labs/KNXBench/issues/30#issuecomment-6719421160)).
  The maintainers checked the bundle against its checksums and the report
  ([reply](https://github.com/KNXBench-Labs/KNXBench/issues/30#issuecomment-6719833331)).
  The journal-derived report confirms `finished`, `verified`, 27/27 and one object-3
  post-load observation, without byte values. Raw byte values and project settlement
  above are reporter statements, not facts independently reconstructed from that
  report; normal function is a reporter observation, not our hardware measurement.
  Private bundle contents are not reproduced here.
- The fix shipped in Alpha 20; #30 was closed on 10 November 2026
  ([comment](https://github.com/KNXBench-Labs/KNXBench/issues/30#issuecomment-6759794276)).

This is one verified download of one device with one configuration: no general
System-2, MV-0025 or recovery claim, and System-2 stays experimental. Broader support
stays in issue #22 and [WP-12](WP-12-field-evidence-and-owner-acceptance.md).

## Update 28 October 2026: final-step failure category (source, focused proof)

Net classifies a failure of the last `Disconnect` step's own operations from the typed
operation in flight; App journals it (envelope version 3, closed `final_step` record, strict
replay), reopen and the reports carry it, status schema 4 serves `finalStepFailures`, and
the panel shows it in EN and DE with an explicit "unavailable" for older journals. Scope,
grammar and limits: [contract](../contracts/system2-download-result.md#final-step-failure).
Proven by focused Net, App, server and web runs, both 15-stage package gates and the
isolated built-server smoke. Source `96f41c7f1` (hash mapping 2026-11-22: alpha-22 history; on current main as `051763e61`) is delivered main-first as `309384726`;
the docs-only upstream delta carries the product acceptance from `8bdc42c7e`, not a new
product run. [Acceptance and exact boundaries](../status/2026-10-28-system2-final-step-failure.md#software-acceptance-and-source-delivery).
No hardware evidence exists.
