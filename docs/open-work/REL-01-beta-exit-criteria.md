# REL-01 — Beta exit criteria: what has to hold before KNXBench leaves alpha

**Status:** Proposed 12 November 2026; owner approval pending. Listed in
[OPEN_WORK](../OPEN_WORK.md). Nothing here is decided until the owner accepts it;
on acceptance the definition becomes an ADR and this file stays the checklist.

## Why this exists

Until now no document defined "beta". [ADR-0040](../adr/0040-programming-requires-release-stage-consent.md)
only makes the programming consent stage-aware (a remembered alpha consent does
not cover beta), and [ADR-0084](../adr/0084-first-run-guide-once-per-release-stage.md)
shows the first-run guide once per stage. Without criteria, the step from
`0.1.0-alpha.N` to a beta version would be a label change, not a promise.

The current-state cells below were read from `main` at `fbaaca115`
(12 November 2026). A cell says *unverified* where it was not checked against code.

## Proposed definition

A KNXBench beta promises four things, and nothing more:

1. **Your data survives.** No open defect is known to lose or corrupt project
   data, and every project file a beta writes opens in every later beta and
   release (forward migration; going back stays unsupported).
2. **The scope is declared.** The release notes name the workflows and the
   device/program families KNXBench may program. Everything else is either
   refused with a reason or visibly marked as experimental.
3. **No known blocker in the declared scope.** Open issues that stop a declared
   workflow are fixed, or the workflow leaves the declared scope.
4. **The limits are written down.** Known limitations and their user-facing
   consequences are current in the manual and the release notes.

A beta is not a stable release, not ETS-equivalent and not KNX-certified.
KNXBench stays "KNX-compatible".

## Criteria

**Must** blocks the beta. **Should** is expected but can be waived by a
recorded owner decision. Each row ends with what the owner has to decide, if anything.

### Data integrity and file formats

| ID | Level | Criterion → acceptance | Current state |
| --- | --- | --- | --- |
| B-01 | Must | **Owner accepts this catalogue** → ADR recorded, version scheme chosen (`0.1.0-beta.1` or `0.2.0-beta.1`). | Proposed. *Decide:* catalogue, version scheme. |
| B-02 | Must | **Written compatibility promise** for `.knxdb`, product database, settings and journals from the first beta on: forward migration for every schema change, a clear refusal of newer files, no silent loss ([ADR-0020](../adr/0020-migrations-may-rederive-from-stored-bytes.md)). | Migrations exist per alpha, but there is no written promise. Alpha 20 alone moved native 16 → 19 and product database 31 → 32 ([upgrade notes](../releases/alpha20.md#upgrade)). |
| B-03 | Must | **Upgrade test from pinned release fixtures**: a gate stage opens, saves and reopens a fictional project written by each published beta (and by the last alpha) and compares the names and counts. | Only a one-off release receipt check exists (Alpha 19 project, 647/647 names in the [Alpha 20 receipt](../evidence/release-alpha20-2026-11-09.json)). No permanent fixture per release. |
| B-04 | Must | **No open data-loss or corruption defect.** Every open issue is triaged for data loss, and the result is recorded. | Not triaged against this criterion yet. Open issues #7, #22, #36 and #38 do not report data loss (based on their titles and the open-work notes, not re-verified). |

### Declared programming scope

| ID | Level | Criterion → acceptance | Current state |
| --- | --- | --- | --- |
| B-05 | Must | **Declared family list**: the release notes and the manual name the masks, procedures and programs that may be downloaded, each with its evidence (simulator only, or field report). Everything outside the list is refused with a reason. | The fail-closed fallback exists ([SAFE-01](../status/LEDGER.md#snapshot-ids)), and field evidence exists for named 0701h programs ([KL-136](../KNOWN_LIMITATIONS.md#catalogue-kl-136)). No single declared list exists; Alpha 20's download types are *untested* on hardware. *Decide:* which families go on the list. |
| B-06 | Must | **System-2 (#22/#30)**: either the allocation-mismatch cause is found and fixed and a reporter run confirms it, or System-2 programming is marked experimental in the beta with its own consent text. | Cause unproven ([S2-05](S2-05-result-and-settlement.md)); S2-02 and S2-04 are waiting for reporter retests. The web UI already titles it "System-2 — experimental configured download" with its own risk text and phrase confirmation (`apps/knx-web/src/messages/en.ts`). *Decide:* fix first, or ship as experimental. |
| B-07 | Must | **Recovery-gated address writes stay gated** (KL-116, KL-139, KL-140, SAFE-01): the beta keeps the current fail-closed behaviour, and the manual says so. Lifting them needs hardware evidence and is not a beta requirement. | Accepted boundaries since 5 October; public writes fail closed without recovery. Manual wording for beta is *unverified*. |
| B-08 | Should | **Field evidence for the declared families** under operation-specific consent ([WP-12](WP-12-field-evidence-and-owner-acceptance.md)); the Hager 0701 report set completed ([WP-33](WP-33-hager-0701-download-evidence.md)). | WP-12 is blocked on consent. WP-33: only the parameters-only report is recorded; the complete and parameters+GA reports are missing. *Decide:* consents, or a list that only contains families with existing evidence. |
| B-09 | Should | **Non-Secure programming gaps** that affect declared families are closed or listed ([PRG-01](PRG-01-programming-gaps.md)). | In progress; bounded increments shipped in Alpha 19. |

### Known blockers in the core workflows

| ID | Level | Criterion → acceptance | Current state |
| --- | --- | --- | --- |
| B-10 | Must | **Device editor does not hang** after save or after the download dialog (#36, [WP-31](WP-31-device-editor-unavailable-after-save.md)): reproduced and fixed, or the remaining cause is known and has a recovery in the UI. | Partly fixed; the reporter's error message is not reproduced. |
| B-11 | Must | **ETS import scope is declared**: ETS 5.7 projects (schema 20, #7, [WP-10](WP-10-import-and-model-evidence.md)) are either imported or refused with a clear message, and the supported ETS versions are listed. | #7 is open and input-gated. Whether schema 20 gets a clear refusal message today is *unverified*. *Decide:* import ETS 5.7 before beta, or declare it unsupported. |
| B-12 | Should | **Pending changes are visible everywhere** a download can change something ([WP-37](WP-37-pending-changes-across-ui.md)). | Steps 1–2 on main; per-link/object marks and module parameters are open. |
| B-13 | Should | **Unsupported parameter features are visible**: wherever WP-05 to WP-09 and WP-14 are evidence-gated, the editor shows the item as unsupported or read-only, and a download refuses values it cannot encode. | The refusal rules exist per feature; that every gated construct is visible as such is *unverified*. |

### Platforms and transports

| ID | Level | Criterion → acceptance | Current state |
| --- | --- | --- | --- |
| B-14 | Must | **Transport scope is declared**: UDP tunnelling is the supported path. TCP ([WP-29](WP-29-knxnet-ip-tcp-tunnelling.md)) and the USB/serial TP-UART ([WP-26](WP-26-status-bar-bus-connection.md)) are marked experimental in the UI and the notes until a real-interface test exists. | The release notes state the limits. The web message catalogue has no "experimental" wording for TCP or USB/serial (only for System-2), so the UI does not mark them yet. |
| B-15 | Should | **Native Linux acceptance** reaches the 8 November target ([WP-11](WP-11-native-linux-acceptance.md#acceptance-target--owner-decision-8-november-2026)). | In progress; no whole-package closure. |

### Release mechanics

| ID | Level | Criterion → acceptance | Current state |
| --- | --- | --- | --- |
| B-16 | Must | **Full gate is reliable**: `run_package_gate.py --full` passes twice in a row without a historical timeout ([GATE-01](GATE-01-test-gate-follow-ups.md)). | Parked until release preparation by owner decision (8 November). |
| B-17 | Must | **Stage-aware texts are checked**: the beta consent sentence (ADR-0040), the first-run guide (ADR-0084), About and update hints show "beta" on a real beta build. | The stage parsing has unit tests; a real beta build was never run. |
| B-18 | Must | **Docker channel decided**: where beta images go (proposal: `next`) and what `latest` means during beta ([OWN-01](OWN-01-owner-decisions.md)). | Prerelease channel name is still open. *Decide:* channel name. |
| B-19 | Should | **Docs current for beta**: manual stage wording, limitations summary review, a beta upgrade section; the website/Evolution Story privacy copy if the website announces the beta. | Manual describes alpha releases; privacy copy is pending in OWN-01. |

## Not required for beta

These stay open, documented and visible, but do not block the stage change:

- KNX Secure, `.knxproj` export, multi-installation projects ([MODEL-05](../status/LEDGER.md)).
- Lifting the recovery-gated address writes (B-07 keeps them gated).
- Full parameter-layout fidelity, module authority and codecs (WP-05 to WP-09,
  WP-14), provided B-13 holds.
- Docker image signing: required before the first **stable** release
  ([roadmap](../ROADMAP.md#docker-signing-design-before-the-first-stable-release)).
- Later-scope items LS-01 to LS-07.

## Owner decisions, smallest first

1. Accept, change or reject this catalogue (B-01) and choose the version scheme.
2. ETS 5.7 import: before beta, or declared unsupported (B-11).
3. System-2: fix first, or ship it as experimental (B-06).
4. The declared family list, and whether it may include families with simulator evidence only (B-05, B-08).
5. Docker prerelease channel name (B-18).

## History

- 12 November 2026: first proposal after the question "what is missing for
  KNXBench to leave alpha?". No implementation started.
