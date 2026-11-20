# OWN-01 — Owner decisions and deferred items

**Status:** Decisions recorded; remaining evidence and owner review pending. Listed in [OPEN_WORK](../OPEN_WORK.md); per-ID status of record is the [ledger](../status/LEDGER.md).

## Decision recorded — 20 November 2026 (third round)

- **The owner grants no further consents.** WP-12 (field evidence under
  operation-specific hardware consent) stays **blocked** permanently unless a
  new, specifically named consent (device + operation) is given. All bus
  evidence stays simulator/loopback-only; no field runs are commissioned.
  This records the owner's stated wish not to grant any more consents — it is
  deliberately **not** a blanket release of devices or operations: per the
  1 November 2026 decision, a blanket "approve everything" is not a consent
  for bus or device writes, and none is recorded here. Read-only access to
  the owner's own installation is unaffected (it never needed a consent).
- Still pending: website / Evolution Story privacy copy, KL-186 plug-in
  acceptance, SmartSensor bootloader follow-up.
- None of this is authority for bus or device writes, releases or deployments.

## Decision recorded — 20 November 2026

- **REL-01 beta-exit catalogue accepted.** The owner accepts the proposed
  19-criteria catalogue (B-01 to B-19, Must/Should) as the definition of beta,
  recorded as [ADR-0178](../adr/0178-rel01-catalogue-defines-the-beta-exit.md);
  [REL-01](REL-01-beta-exit-criteria.md) stays the living checklist. The version
  scheme decision of 12 November (standard SemVer, beta is `0.1.0-beta.N`)
  stands.
- **B-11 decided: ETS 5.7 supported for beta.** The evidence that schema-20
  projects import since 12 October 2026 (`f23be82cc`, KL-177) on three public
  ETS 5.7 samples, with unknown schemas refused naming the supported list
  (11, 20, 21, 23), is accepted as sufficient for the beta declaration. Not
  verified on the #7 reporter's project; the supported ETS versions must be
  listed in the release notes.
- **Still pending from REL-01:** B-06 (System-2 experimental vs fix-first),
  B-05/B-08 (declared family list, simulator-only evidence), B-18 (Docker
  prerelease channel name). Still pending overall: Docker signing, website
  privacy copy, KL-186 and WP-12 consents.
- None of this is blanket authority for bus or device writes, releases or
  deployments; tagging `0.1.0-beta.1` additionally requires the Must cells of
  REL-01 to hold.

## Decision recorded — 20 November 2026 (second round)

- **B-06 decided: System-2 ships in the beta as experimental** with its own
  consent text — the current UI behaviour ("System-2 — experimental configured
  download", own risk text and phrase confirmation) stands. Evidence: one
  device and configuration only, reporter run 8 November verified 27/27;
  the `Download_Flag` clear time stays unknown.
- **B-05/B-08 decided: the declared programming-family list may include
  families with simulator/loopback-only evidence.** Each family's evidence
  class (simulator only, or field report) is named beside it in the release
  notes and manual. This does not lift WP-12: operation-specific hardware
  consents stay blocked — on 20 November 2026 the owner was asked for a
  consent and answered "all devices that need a manufacturer plugin, all
  actions"; that is a blanket phrase, not a named device-and-operation
  consent, and is **not** recorded as authority for any bus or device write.
- **B-18 decided: the Docker prerelease channel is `next`.** `latest`
  semantics stay unchanged during beta (from the first stable release it
  follows only the newest verified stable release, per the 27 October
  decision). All five REL-01 owner decisions (B-01, B-11, B-06, B-05/B-08,
  B-18) are now recorded; [REL-01](REL-01-beta-exit-criteria.md) cells updated.
- **Docker image signing: the bounded design proposal**
  ([2026-10-27-docker-image-signing-design](../research/2026-10-27-docker-image-signing-design.md)
  — identity, verification, recovery) **is approved**, to be implemented
  before the first stable release. No key creation, external login, image
  publication or retrospective change is authorised by this approval.
- **Kept pending by owner:** website / Evolution Story privacy copy (no blanket
  approval without a concrete review copy), KL-186 plug-in acceptance (lift
  needs owner-verified acceptance in the originating plug-in), the SmartSensor
  bootloader follow-up. WP-12 stays blocked on named consents.
- None of this is blanket authority for bus or device writes, releases or
  deployments.

## Decision recorded — 17 November 2026

- **WP-07 unblocked.** The owner accepts the independently counted
  module-mapping witness
  ([2026-11-17-wp07-module-mapping-witness](../research/2026-11-17-wp07-module-mapping-witness.md))
  as the evidence for WP-07's first deliverable: 149 module instances across
  eight ETS exports of one fictional project plus two modular corpus projects,
  292/292 argument values matching `Start + Allocates·(k−1)`, RepeatIndex
  149/149, 23 697/23 697 corpus-wide `BaseOffset`/`BaseNumber` argument
  references resolving. The evidence block ("Blocked: evidence") is lifted;
  WP-07 moves from evidence-gated to open implementation work. Still out of
  scope and not claimed: download-image arithmetic on the wire, bus behavior,
  device readback. This is not blanket authority for bus or device writes,
  releases or deployments, and it does not close WP-07 as a whole.

## Decision recorded — 12 November 2026

- **Release versions follow standard SemVer.** The beta keeps the release
  target and changes only the pre-release identifier: `0.1.0-alpha.N` is
  followed by `0.1.0-beta.N`, then `0.1.0-rc.N`, then `0.1.0`. Recorded in
  [REL-01](REL-01-beta-exit-criteria.md) (B-01); the beta exit catalogue itself
  is not accepted yet, and this decision does not start a beta.

## Decisions recorded — 8 November 2026

Owner answers to that day's list of open decisions. As before, none of them is
blanket authority for bus or device writes, releases or deployments.

- **Evidence by clean-room observation.** Evidence-gated work may use data the
  project observes itself: ETS recorded against KNXBench's own simulator or
  exporting fictional projects, the private local corpus, material a reporter
  sent for an issue, and backups of the owner's own devices. Findings are
  written in own words with their provenance; vendor code, product data,
  private projects and raw captures stay out of Git. Rule:
  [evidence routes](../RESEARCH.md#evidence-routes). This opens a route for
  WP-05, WP-06, WP-07, WP-08, WP-09, WP-10 (IMPORT-02: such ETS exports count as
  authorised exports) and WP-14; it is not itself evidence for any of them.
- **Reporter field runs (#30/#22).** One optional System-2 attempt on a current
  `main` build was requested ([#30](https://github.com/KNXBench-Labs/KNXBench/issues/30#issuecomment-6718382084), pointer in #22). It serves as the
  S2-02 reporter retest and as the S2-05 field run of the `Download_Flag`
  tolerance. Done 8 November 2026: the run finished verified 27/27
  ([S2-05](S2-05-result-and-settlement.md#field-run-8-november-2026)); #30 closed
  10 November, S2-02 and S2-04 closed 15 November without a proven original cause.
- **WP-11 scope: the simplest complete target.** See
  [acceptance target](WP-11-native-linux-acceptance.md#acceptance-target--owner-decision-8-november-2026).
- **GATE-01:** the larger scheduler and the historical timeouts are reconsidered
  only during release preparation.
- **KL-006 coverage pins (PRG-01):** replace the unattributable historical
  105-package selection with a new documented baseline. Implemented as the
  [122-package characterisation baseline](../research/commissioning/19-22-product-coverage-characterisation-baseline.md);
  broader [PRG-01](PRG-01-programming-gaps.md#open) remains open.
- **Docker base-image rebuilds:** approved; implement before the first stable
  release. Published version tags are not silently overwritten; how a rebuild is
  published is settled in that package. The prerelease channel's name remains
  open (proposal: `next`). Supersedes the 27 October deferral below.
- **Public research summary:** publication approved once a separately authored
  summary passes the confidentiality and provenance review. Originals stay
  private.
- **Private mailbox verification** is the owner's own action and is no longer
  tracked here.
- **KL-038b — ETS group-address CSV import.** (1) CSV import encoding is an
  explicit choice, UTF-8 (default) or Windows-1252, never detected; undefined
  Windows-1252 bytes are refused. (2) Group addresses get a description in the
  model (native schema migration, `.knxproj` mapping, undoable commands, diff,
  read-only display). (3) The full CSV profiles apply `Description`, and a
  `DatapointType` declares the type of an address the import creates; existing
  addresses keep the read-only comparison. Implemented as
  [ADR-0164](../adr/0164-group-address-description-and-csv-encoding.md);
  boundaries in [KL-038](../limitations/KL-038-csv-interoperability.md).

Still open after this round: Docker signing approval, the website / Evolution
Story privacy review copy, KL-186 plug-in acceptance, the WP-12 operation-specific
hardware consents and the SmartSensor bootloader follow-up.

## Decisions recorded — 1 November 2026

- OKX-05 passive Button labels: the cue wording "Source button label – action
  unsupported" (German "Beschriftung einer Quell-Schaltfläche – Aktion nicht
  unterstützt") is accepted, and skipping the source read for programs whose
  Buttons cannot show a label is approved
  ([contract](../contracts/alpha16-parameter-layout.md#passive-source-button-labels-button-literal-passive-v1)).
  A blanket "approve everything" reply is not a consent for bus or device
  writes, releases or deployments; those still need their own named go.

## Decisions recorded — 27 October 2026

These decisions are not delivery evidence or blanket execution authority.

- [WP-11](WP-11-native-linux-acceptance.md): desktop control is authorised for
  bounded native acceptance with isolated fictional/simulator data. No bus
  access or real-project changes. The first bounded Alpha 17 slice is now
  [recorded](../status/2026-10-27-wp11-native-acceptance.md); WP-11 remains open.
- [S2-03](S2-03-up258-image-interpretation.md): retain the existing source-based
  communication-bit policy. Invisible-parameter precedence: decided 28 October
  2026 as defaults v3 template retention for explicit `Background` only
  (observation-supported hypothesis); no ETS equivalence or hardware approval follows.
- [WP-10 / MODEL-05](WP-10-import-and-model-evidence.md): no current native-site
  need across separate installations; explicitly deferred, not implemented.
  Other import/model evidence cases remain independent.
- Root-only research and triage originals remain private; this resolves their
  former publish-or-keep-local question. A separately authored,
  publicly supportable summary may be prepared for review after confidentiality
  and provenance checks; publication is not authorised.
- CI stays manually disabled; established local gates remain authoritative.
  This is not an instruction to disable release, website or Docker Hub
  description workflows, or to reactivate CI.
- KL-185: accept the residual risk of `refs/pull/3` and `refs/pull/24` serving
  pre-rewrite commits. No GitHub Support request or other new contact.
- Technical leftovers may be considered for later cleanup only after fresh
  ownership, reachability and use checks. No cleanup or deletion is authorised
  now. Active worktrees, used symlinks, other tracks, private notes, backups and
  the parked `wip/manufacturer-plugin-research-20261013` branch remain protected.
  This folds in the old container/source-path and local
  `refs/kl185-rewritten/*` leftovers: none may be removed without that fresh
  ownership/reachability/use check; previously used paths are not presumed idle.
- Keep the private pre-launch rollback repository; it is not a cleanup target.
- Docker signing: the [bounded design proposal](../research/2026-10-27-docker-image-signing-design.md)
  covers identity, verification and recovery before the first stable release.
  Design preparation is complete; approval, implementation and operational
  acceptance remain pending. No key creation, external login, image publication
  or retrospective Alpha 17 change is authorised.
- Scheduled Docker base-image rebuilds remain deferred; reconsider before the
  first stable release. No automatic republication or silent version-tag reuse.
- Docker `latest`: from the first stable release, follow only the newest verified
  stable release. A separate prerelease channel's name and implementation remain
  future work. Until then the existing alpha rule is unchanged; no tag or
  workflow change is commissioned here.

## Already delivered foundation

Docker Hub description sync is already documented in
[ADR-0097](../adr/0097-docker-hub-release-image.md#consequences) and implemented in
[its separate workflow](../../.github/workflows/dockerhub-description.yml),
including exact public readback against the maintained source. It is not a new
owner-decision gap. This reconciliation does not claim a fresh workflow run or
live Docker Hub verification.

## Remaining evidence and owner review

- WP-11 has a first bounded native slice; the cells of the 8 November
  acceptance target stay open, not accepted by consent alone.
- S2-03's invisible-parameter precedence was decided by the owner on 28 October
  2026 (defaults v3, observation-supported hypothesis); defining evidence is still
  absent. Other import/model and hardware evidence boundaries are unchanged.
- Website / Evolution Story privacy acceptance requires a concrete review copy;
  no blanket privacy approval was given. Optional WebKit evidence remains open.
- A public research summary still needs its confidentiality/provenance review;
  publication after that review was approved on 8 November 2026. The originals
  stay private.

## Deferred by recorded decision (reopen only on a new decision)

- ADR-0039 phases 3–5, sealing the ID allocator (KL-129).
- Package/application version pinning and a selector (KL-135).
- Live tunnel from a Docker bridge container (Route Back): only with a go.
- New real-hardware, power-loss, vendor and ETS validation for
  commissioning.
- Durable pre-write recovery for K6 button / serial / K13 address writes;
  every new write needs its own go.
- Reset UI (UI-04): refused with a user notice.

Existing later scope (LS-01–LS-07), parked research and other tracks remain
unchanged. No new reporter attempt, bus action, release or deployment follows
from this reconciliation.

## References

[P6–P8](../goals/open-work-2026-10-17/p6-p8-not-goal-ready.md) ·
[parked-work triage](../status/2026-10-12-parked-work-triage.md) ·
[ledger](../status/LEDGER.md) ·
[history](../history/OPEN_WORK_2026-10-25.md#3-deferred-by-a-recorded-user-decision)
