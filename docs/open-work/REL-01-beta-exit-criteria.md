# REL-01 — Beta exit criteria: what has to hold before KNXBench leaves alpha

**Status:** Catalogue **accepted by owner decision 20 November 2026** — recorded
as [ADR-0178](../adr/0178-rel01-catalogue-defines-the-beta-exit.md); version
scheme decided (standard SemVer, see B-01); B-11, B-06, B-05/B-08 and B-18
decided (see the cells). This file stays the living checklist. Listed in
[OPEN_WORK](../OPEN_WORK.md).

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
| B-01 | Must | **Owner accepts this catalogue** → ADR recorded, version scheme chosen. | **Version scheme decided 12 November 2026: standard SemVer.** The release target stays `0.1.0`; only the pre-release identifier changes, giving `0.1.0-alpha.20 < 0.1.0-beta.1 < … < 0.1.0-rc.1 < 0.1.0` (SemVer 2.0.0 §9, §11). The web update check already implements §11 precedence, and its tests cover the specification chain `alpha.1 → beta → beta.11 → rc.1` (`apps/knx-web/src/releaseUpdates.test.ts`); the AppImage and Docker workflows accept any `vMAJOR.MINOR.PATCH[-PRERELEASE]` tag. **Catalogue accepted by owner decision 20 November 2026 — recorded as [ADR-0178](../adr/0178-rel01-catalogue-defines-the-beta-exit.md).**
| B-02 | Must | **Written compatibility promise** for `.knxdb`, product database, settings and journals from the first beta on: forward migration for every schema change, a clear refusal of newer files, no silent loss ([ADR-0020](../adr/0020-migrations-may-rederive-from-stored-bytes.md)). | Migrations exist per alpha, but there is no written promise. Alpha 20 alone moved native 16 → 19 and product database 31 → 32 ([upgrade notes](../releases/alpha20.md#upgrade)). |
| B-03 | Must | **Upgrade test from pinned release fixtures**: a gate stage opens, saves and reopens a fictional project written by each published beta (and by the last alpha) and compares the names and counts. | Only a one-off release receipt check exists (Alpha 19 project, 647/647 names in the [Alpha 20 receipt](../evidence/release-alpha20-2026-11-09.json)). No permanent fixture per release. |
| B-04 | Must | **No open data-loss or corruption defect.** Every open issue is triaged for data loss, and the result is recorded. | Not triaged against this criterion yet. Open issues #7, #22, #36 and #38 do not report data loss (based on their titles and the open-work notes, not re-verified). |

### Declared programming scope

| ID | Level | Criterion → acceptance | Current state |
| --- | --- | --- | --- |
| B-05 | Must | **Declared family list**: the release notes and the manual name the masks, procedures and programs that may be downloaded, each with its evidence (simulator only, or field report). Everything outside the list is refused with a reason. | The fail-closed fallback exists ([SAFE-01](../status/LEDGER.md#snapshot-ids)), and field evidence exists for named 0701h programs ([KL-136](../KNOWN_LIMITATIONS.md#catalogue-kl-136)). No single declared list exists; Alpha 20's download types are *untested* on hardware. **Decided 20 November 2026: the declared list may include families whose evidence is simulator/loopback only; each family's evidence class is named beside it** (B-05/B-08, [OWN-01](OWN-01-owner-decisions.md#decision-recorded--20-november-2026)). |
| B-06 | Must | **System-2 (#22/#30)**: either the allocation-mismatch cause is found and fixed and a reporter run confirms it, or System-2 programming is marked experimental in the beta with its own consent text. | The #30 mismatch is accounted for by the firmware-cleared `Download_Flag` octet (observed on the reporter's device; clear time unknown); its tolerance ships in Alpha 20, and one reporter run on 8 November finished verified 27/27 ([S2-05](S2-05-result-and-settlement.md#field-run-8-november-2026)); #30 closed. S2-02 and S2-04 closed 15 November without a proven original cause. One device and configuration only. The web UI already titles it "System-2 — experimental configured download" with its own risk text and phrase confirmation (`apps/knx-web/src/messages/en.ts`). **Decided 20 November 2026: ship as experimental with its own consent text** — the current UI behaviour stands ([OWN-01](OWN-01-owner-decisions.md#decision-recorded--20-november-2026)). |
| B-07 | Must | **Recovery-gated address writes stay gated** (KL-116, KL-139, KL-140, SAFE-01): the beta keeps the current fail-closed behaviour, and the manual says so. Lifting them needs hardware evidence and is not a beta requirement. | Accepted boundaries since 5 October; public writes fail closed without recovery. Manual wording for beta is *unverified*. |
| B-08 | Should | **Field evidence for the declared families** under operation-specific consent ([WP-12](WP-12-field-evidence-and-owner-acceptance.md)); the Hager 0701 report set completed ([WP-33](WP-33-hager-0701-download-evidence.md)). | WP-12 is blocked on consent. WP-33 closed 13 November: complete, parameters-only and parameters+GA reports recorded for the Hager 0701 push button; group addresses only was not run. **Decided 20 November 2026: simulator-only evidence is accepted for the declared list** (with B-05); WP-12 operation-specific consents stay blocked — a blanket "all devices, all actions" is not a consent ([OWN-01](OWN-01-owner-decisions.md#decision-recorded--20-november-2026)). |
| B-09 | Should | **Non-Secure programming gaps** that affect declared families are closed or listed ([PRG-01](PRG-01-programming-gaps.md)). | In progress; bounded increments shipped in Alpha 19. |

### Known blockers in the core workflows

| ID | Level | Criterion → acceptance | Current state |
| --- | --- | --- | --- |
| B-10 | Must | **Device editor does not hang** after save or after the download dialog (#36, [WP-31](WP-31-device-editor-unavailable-after-save.md)): reproduced and fixed, or the remaining cause is known and has a recovery in the UI. | **Reported #36 blocker closed 14 November 2026.** Original download-dialog path confirmed fixed 10 November; save/reopen retest on `0.1.0-alpha.21+g6a9336c5` no longer hangs or shows the unavailable-editor message, and the reporter closed the issue. No reverse proxy in that setup. [Evidence and limits](../status/2026-11-14-issues36-48-reporter-closure.md): the exact historical cause remains unproven; this closes the reported blocker, not a universal no-hang proof or beta approval. |
| B-11 | Must | **ETS import scope is declared**: ETS 5.7 projects (schema 20, #7, [WP-10](WP-10-import-and-model-evidence.md)) are either imported or refused with a clear message, and the supported ETS versions are listed. | Checked 12 Nov 2026: ETS 5.7 schema-20 projects **import** since 12 Oct 2026 (`f23be82cc`, KL-177), on evidence from three public ETS 5.7 samples; the import report carries an `unsupported` entry naming that evidence boundary. Unknown schemas are refused with a message naming the schema and the supported list (11, 20, 21, 23); schema 22 stays refused. Not verified on the #7 reporter's project. Remaining: list the supported ETS versions in the release notes. **Decided 20 November 2026: the public-sample evidence suffices — ETS 5.7 (schema 20) is declared supported for beta** ([ADR-0178](../adr/0178-rel01-catalogue-defines-the-beta-exit.md)). |
| B-12 | Should | **Pending changes are visible everywhere** a download can change something ([WP-37](WP-37-pending-changes-across-ui.md)). | Steps 1–4 shipped in Alpha 21, including per-link/object marks and section/ancestor marks; #48 reporter acceptance done and issue closed 14 November ([evidence](../status/2026-11-14-issues36-48-reporter-closure.md)). Module-scoped parameters shipped 17 November 2026 (schema 22, ADR-0189) with the image-builder admission of base-offset module images (PR !60, `b4d141288`; stored module-qualified values refuse, WP-48 F14), and the group-address views now carry last-download link marks (PR !104, `aa14a7329`, 20 November 2026). **Complete for the software/UI scope** — all WP-37 slices delivered on main; simulator/intercepted-browser evidence only, no hardware run; the remaining WP-37 items are documented boundaries, not open work. |
| B-13 | Should | **Unsupported parameter features are visible**: wherever WP-05 to WP-09 and WP-14 are evidence-gated, the editor shows the item as unsupported or read-only, and a download refuses values it cannot encode. | The refusal rules exist per feature; that every gated construct is visible as such is *unverified*. |

### Platforms and transports

| ID | Level | Criterion → acceptance | Current state |
| --- | --- | --- | --- |
| B-14 | Must | **Transport scope is declared**: UDP tunnelling is the supported path. TCP ([WP-29](WP-29-knxnet-ip-tcp-tunnelling.md)) and the USB/serial TP-UART ([WP-26](WP-26-status-bar-bus-connection.md)) are marked experimental in the UI and the notes until a real-interface test exists. | UI part met on main (12 Nov 2026): the transport selectors in Settings, the status-bar drop-up and Bus Monitor, and the line-scan TCP option read "experimental" (en+de) with a note naming UDP as the supported path; the status-bar state, the monitor session and a TCP line-scan result carry an experimental tag. No behaviour change. The release notes still need the same wording at the next release. |
| B-15 | Should | **Native Linux acceptance** reaches the 8 November target ([WP-11](WP-11-native-linux-acceptance.md#acceptance-target--owner-decision-8-november-2026)). | In progress; no whole-package closure. |

### Release mechanics

| ID | Level | Criterion → acceptance | Current state |
| --- | --- | --- | --- |
| B-16 | Must | **Full gate is reliable**: `run_package_gate.py --full` passes twice in a row without a historical timeout ([GATE-01](GATE-01-test-gate-follow-ups.md)). | Parked until release preparation by owner decision (8 November). |
| B-17 | Must | **Stage-aware texts are checked**: the beta consent sentence (ADR-0040), the first-run guide (ADR-0084), About and update hints show "beta" on a real beta build. | The stage parsing has unit tests; a real beta build was never run. |
| B-18 | Must | **Docker channel decided**: where beta images go (proposal: `next`) and what `latest` means during beta ([OWN-01](OWN-01-owner-decisions.md)). | **Decided 20 November 2026: the prerelease channel is `next`; `latest` semantics stay unchanged during beta** ([OWN-01](OWN-01-owner-decisions.md#decision-recorded--20-november-2026)). |
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

1. ~~Accept, change or reject this catalogue (B-01). The version scheme is decided.~~
   **Accepted 20 November 2026 — ADR-0178.**
2. ~~ETS 5.7 import: before beta, or declared unsupported (B-11).~~ **Decided
   20 November 2026: schema 20's public-sample evidence suffices; ETS 5.7 is
   declared supported for beta (ADR-0178).**
3. ~~System-2: fix first, or ship it as experimental (B-06).~~ **Decided
   20 November 2026: ship as experimental with its own consent text.**
4. ~~The declared family list, and whether it may include families with simulator evidence only (B-05, B-08).~~
   **Decided 20 November 2026: yes, simulator/loopback-only evidence is
   accepted for the declared list; each family's evidence class is named.**
5. ~~Docker prerelease channel name (B-18).~~ **Decided 20 November 2026:
   `next`; `latest` unchanged during beta.**

## History

- 12 November 2026: first proposal after the question "what is missing for
  KNXBench to leave alpha?". No implementation started.
- 12 November 2026: owner decision "standard SemVer principle" recorded in B-01
  and [OWN-01](OWN-01-owner-decisions.md). The beta is `0.1.0-beta.N`.
- 20 November 2026: owner accepted the catalogue (B-01) and decided B-11
  (ETS 5.7 supported for beta); recorded in
  [ADR-0178](../adr/0178-rel01-catalogue-defines-the-beta-exit.md) and
  [OWN-01](OWN-01-owner-decisions.md#decision-recorded--20-november-2026).
- 20 November 2026 (second round): B-06 (System-2 experimental with own consent
  text), B-05/B-08 (simulator-only evidence accepted for the declared list) and
  B-18 (Docker prerelease channel `next`) decided; all five REL-01 owner
  decisions are now recorded. Docker image signing design approved for before
  the first stable release. See
  [OWN-01](OWN-01-owner-decisions.md#decision-recorded--20-november-2026).
