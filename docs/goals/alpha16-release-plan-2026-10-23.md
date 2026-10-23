# Alpha 16 release plan — 23 October 2026

## Authority, baseline and outcome

Owner request: plan Alpha 16, finish sensible implementable work, include System-2
download, minimise test execution and check the integrated Alpha 16 package.
Owner implementation Go received on 23 October 2026, including takeover of B10
where required. The previous implementation pause is lifted for this plan's scope.
This authorises implementation and consolidated software acceptance, not release
publication, deployment, desktop control or bus/device operations. No target
release date is promised before the blocking work has been reconciled.

Inspected published source: `f2ee6df3d50c211ecc79c6d7e5d28c7256a3a694`.
Latest published release at inspection: `v0.1.0-alpha.15`, 19 October 2026.
Alpha 16 was not tagged. GitHub had open issues #22 and #7 and no open PRs.
An open issue is not proof that its original code change is still missing.

Release outcome: a usable, bounded **experimental System-2 download** for the
exact UP 258D12 application `M-0001_A-9205-04-9270`, using the selected project's
actual configuration, plus completed catalogue, comparison and data-portability
workflows. Do not market this as all-System-2 support, ETS equivalence or proven
physical recovery. A defaults-only simulator command is not a substitute for
the mandatory download outcome.

This plan proposes prioritising the complete System-2 outcome alongside the
remaining catalogue workflow, ahead of optional layout expansion. It does not
silently resume or transfer ownership of paused work. Existing WP numbers keep
their scope. On implementation Go, this plan's bundled Alpha-16 acceptance cadence
replaces repeated per-increment package gates for this release; it does not waive
data-integrity assertions, tests or review.

## Verified starting point — do not implement twice

- Product DB v28 promotion repair, WP-04.1 measurement and WP-04.2 bounds work
  are delivered. They need release inclusion, not another implementation.
- WP-04.3 has owner/witness/copy, transitive control, DTO/spool/artifact and job
  foundations, preview/apply rehearsal and subsequent integrity corrections.
  Production routes, lifecycle and shared UI are not complete. The paused
  retained-COMMIT increment is not accepted merely because partial files exist.
- System-2 defaults preparation, complete candidate procedure, durable simulator
  execution/recovery, and a consent-gated CLI/App/Net hardware route exist.
  [ADR-0152](../adr/0152-gated-hardware-system2-download.md) is accepted; older
  blanket hardware-refusal prose is not the current complete description.
- The procedure source already uses full application identity for table task
  segments. The older research note's requested zero-identity correction must
  not become a duplicate coding task. Task-start mapping still needs explicit
  source/observation reconciliation before claiming procedure agreement.
- [Project-configured preparation](../contracts/project-system2-offline-preparation.md)
  is delivered **offline only** and explicitly simulator-bound. It cannot simply
  be relabelled/rebound as a physical download bundle.
- WP-18 comparison differences, WP-20 system labels, the UI feedback set,
  monitoring snapshot export, ZIP integrity repairs and native Rolldown build
  preflight are delivered. Include their post-Alpha-15 deltas where applicable;
  do not reconstruct the features from old unchecked prose.

Evidence entry points: [OPEN_WORK](../OPEN_WORK.md),
[delivery index](../IMPLEMENTATION_STATUS.md),
[canonical ledger](../status/LEDGER.md),
[older successor sequence](alpha16-successor-sequence-2026-10-19.md).
Source inspection establishes implementation boundaries, not fresh runtime success.

## Required release work

Each row specifies action → deliverable → acceptance. Acceptance below belongs to
one integrated release candidate, not a separate full gate per row.

| ID | Action and deliverable | Completion condition |
| --- | --- | --- |
| A16-01 | **Reconcile System-2 procedure and admission.** Compare the current ordered source-to-plan translation with the documented ETS observation; settle task starts, full identities, resource/machine mapping and TaskCtrl2 within the exact profile. Reconcile contradictory current guidance against ADR-0152, retaining dated history and unresolved hypotheses. | Every source position is accounted for, unsupported required transformations refuse, and emitted operations have a named source/evidence basis. No guessed padding, byte-order switch, allocation rule or wider mask admission. Existing full-table identity is reused, not reimplemented. |
| A16-02 | **Complete project → physical System-2 execution/recovery.** Design and implement a separate, target-bound admission of the actual configured request into the existing hardware service; retain configuration, source/profile/plan identity and fresh device observation. Reuse the existing executor, journal, exclusive gateway ownership and both recovery-track choices. | Selected parameter values, group links and flags actually reach the plan; a new unsaved project works. Wrong/stale source, project, application or target refuses. Every mutation, including setup, follows a persisted/read-back fresh actual-state backup. Cancel/failure/reopen retain uncertainty and the original backup; recovery is explicit, best-effort and never reports Restored without full required verification. Simulator-only project bundles do not gain admission by changing a flag. |
| A16-03 | **Expose the complete System-2 workflow.** Add a dedicated path in the existing device download UI, backed by shared App logic and thin API/CLI adapters: preparation, exact scope/limitations, preview, backup destination, ordinary write confirmation plus experimental-risk acknowledgement, execution, result and explicit recovery. Reconcile CLI help/preview text with its existing hardware branch. | The UI downloads the selected project configuration, not product defaults. UI, CLI and API apply the same admission and stale-state rules; neither a client boolean nor opening the dialog grants write authority. Progress and cancellation are truthful; no automatic retry/restore. Private recovery stays local and only a reviewed minimal report is shareable. This is a complete bounded experimental workflow, not a generic System-2 support claim. |
| A16-04 | **Finish WP-04.3 as one usable catalogue feature.** Reconcile the paused retained-COMMIT handoff, then close actual managed-configuration/commit-settlement gaps, same-gate cancellation/COMMIT handling, poisoned ownership and recovery-artifact disposition. Finish durable artifact publication/resource bounds, production routes/worker, standalone/embedded/desktop close-drain, and App-root-owned UI. | Large-profile upload → preview → confirmation → atomic apply works through the existing installer. Cancel before commit publishes nothing; witnessed commit cannot become cancelled/rolled-back because later cleanup fails. Unknown settlement quarantines and preserves recovery. Status/cancel remain responsive, wizard unmount does not lose ownership, and exit/restart cannot silently strand a writer. One bounded large-profile scenario records actual resource behaviour without claiming aggregate quotas or universal 8–64 GiB performance. |
| A16-05 | **Implement WP-19 comparison synchronisation.** Amend the comparison contract/ADR-0130; preview and atomically adopt only verified, unambiguously decoded device values into the project as one undoable command. Project → device hands over to the existing validated download path, including A16-03 when applicable. | Stale/wrong-application/ambiguous adoption refuses the whole change. Unknown/raw/masked bytes are not adopted. Undo restores the complete prior state and simulator re-comparison confirms adopted fields. No differential-download implementation or second write route. |
| A16-06 | **Repair catalogue-authored project portability (KL-183).** First define exact product-source dependency acquisition/ownership for catalogue-added devices; then reuse native retained-source/manifest infrastructure on Save/Save As. Report unavailable dependencies explicitly rather than advertising a self-contained file. | An admitted catalogue-created project saves, reopens against an empty product database and resolves its retained admitted product sources with identical project state. Missing/corrupt/ambiguous sources cause explicit diagnostics or refusal, never silent source substitution. Preserve opaque originals and existing import/source ownership. Previously unresolved old files are not magically recoverable without their original sources. |
| A16-07 | **Finish evidenced WP-05 layout fidelity.** Create the finite source-feature table; prioritise block/separator caption language overlays and resolvable row/column caption references using the existing evaluator/text resolver. Include further Table/Grid, inline/nested/CollapseIfEmpty behaviour only where source authority is established before scope freeze. | Each admitted feature has an explicit contract and inert unsupported fallback; drafts, access/activation, search and narrow row/grid views remain consistent. No second parameter evaluator, guessed manufacturer hint, module identity inferred from captions or unbounded “all widgets” promise. Evidence-blocked subfeatures remain named exclusions, not false completions. |
| A16-08 | **Reconcile release-facing contracts and upgrade guidance.** Update ARCHITECTURE, COMPATIBILITY, relevant limitations, ROADMAP, implementation/manual feature status and the Alpha 16 scope note against actual source and accepted outcomes. Compare highlights with Alpha 15 ancestry. Reconcile #7's already-implemented import work separately from reporter acceptance; keep #22 open until its actual hardware outcome is established. | No contradiction between experimental admission, simulator-only preparation and hardware verification. Product26 → product28 upgrade requires a separate closed-catalogue backup; older readers' refusal is disclosed. Native/model version is read from the final candidate, not assumed to change. No old work promoted to a new fix or issue closed merely for shipping infrastructure. |
| A16-09 | **Freeze, accept and prepare release artifacts.** Reconcile owned/unpublished work, freeze a full SHA, bump all application versions/locks together, run the consolidated acceptance below, and build AppImage/MCP and amd64/arm64 container artifacts. | All required feature rows are complete or their precise conditional subfeature exclusions are explicit; one final candidate has accepted source review, complete gate receipt, SHA-bound successful packaging dry runs and isolated artifact smokes. No tag, release upload or container publication without explicit owner publication Go. |

### System-2 release blocker and evidence boundary

**A16-01–03 are mandatory.** If the configured experimental download is not
usable, Alpha 16 is not ready under this plan; do not quietly replace it with a
simulator milestone or move it to Alpha 17.

Reuse ADR-0152's explicit best-effort release decision. Do not waive fresh actual
backup, exact target binding, ordinary confirmation or honest failure reporting.
The masked/unwritten regions and indirect firmware effects remain disclosed;
consent is neither a general compatibility certificate nor recovery proof.

Software acceptance uses the real integrated caller with a hardware-kind test
transport and an independent persistent-state simulator. A real-device trial is
separate, target/operation-specific and requires fresh consent; this planning
request authorises none. If no such trial is available at cutoff, the release
must say **experimental, software-tested, hardware outcome unverified**. Claiming
verified device support requires actual readback and the operator's functional
check; merely returning success from transport is insufficient.

## Remaining queue: finish only admitted, meaningful slices

This disposition covers the existing WP queue without converting accepted limits
or unavailable source semantics into speculative implementation tasks.

| Queue scope | Alpha 16 disposition |
| --- | --- |
| WP-01, WP-02 including promotion repair, WP-04.1/.2, WP-15, WP-17, WP-18, WP-20 | Already delivered; include applicable deltas and final package regression, no duplicate implementation. |
| WP-04.3, WP-19 | Required: A16-04 and A16-05. |
| WP-05 | A16-07, limited to source-backed subfeatures. |
| WP-03 legacy binary enum values | Conditional: implement only after an independently matched legacy/conversion witness defines lexeme/width mapping. The G16 diagnostic slice already ships in source. If the mapping remains absent, keep the explicit report/refusal; no generic legacy importer rewrite. |
| WP-06 object folders/priority | Conditional: reuse the delivered folder evidence slice, then admit deterministic project folder identity/rendering if supported by retained sources. Manufacturer mapping must not be guessed. Priority needs its raw/effective provenance ADR and full persistence/import/undo path; no UI-only priority field. Do not let this expand the mandatory release critical path without an evidenced contract. |
| WP-07 module-instance authority | Evidence-blocked until an independently counted instance/repeat/offset mapping exists; do not infer identity from text or widen writes to finish a checklist. |
| WP-08 Date/Colour/Picture | Conditional per exact encoding/resource form and independent vectors. No blanket widget support or substitution of DPT bus codecs for product parameter codecs. |
| WP-09 image evidence | Implement only rules actually established for A16-01/02 or a separately admitted exact case. UTF-8 termination, LoadedImage/CheckSums and unknown placement rules stay refused when undefined. |
| WP-10 import/model | Supplied, authorised source cases only. No invented revision exports, Site/SubModule grammar, additional schema compatibility or AES/Secure scope. Issue #7 being open does not justify rebuilding its delivered parser route. |
| WP-11 native Linux | Routine isolated release-artifact startup/open/save smoke is in A16-09. Broader native desktop-control/Wayland/Orca work needs its separate permission and evidence; disclose what was not verified. |
| WP-12 field/reporter evidence | Consume available authorised evidence and report its actual scope. No automatic bus trial, vendor contact, upload, purchase or external recovery certification. |
| WP-13 documentation | Named old prose sweep is delivered; A16-08 handles the new release drift. KL-183 runtime repair is separately A16-06. |
| WP-14 other complete family outcomes | System 1/B/300 remain outside this release unless separately scoped and source-ready; they must not delay the mandatory exact System-2 path or become new mask allowlists. |
| WP-16 ZIP metadata residue | Existing directory/file/encryption repairs are delivered. Bit-6/AES metadata questions are conditional integrity fixes only if source-backed contradictions are established; no new cipher claim. |
| Deliberate later/externally blocked ledger rows | No automation VM, online vendor catalogue, Powerline, differential download, Secure/key management, collaboration/mobile, statistics collector, allocator-policy change or history/container cleanup is added by this plan. |

Conditional work is admitted **before** final scope freeze, with a concrete
implementation and source contract. There is no repeated broad research campaign
just to keep the release busy. A negative result is recorded honestly, not turned
into a claim that the capability is implemented.

## Implementation order and ownership

1. After Go, reconcile the paused retained-COMMIT handoff and all current owners;
   preserve partial work and evidence rather than adopting it as accepted.
2. Run two bounded tracks: **System-2 A16-01 → 02 → 03**, and **catalogue
   A16-04 → comparison A16-05**. These may proceed independently where file
   ownership is disjoint. Shared App/download/UI integration is serialised.
3. Develop A16-06 and source-backed A16-07 slices without competing migrations,
   write-route changes or edits to another track's frozen files. Reconcile
   conditional queue items once before cutoff, not after every small commit.
4. Finish A16-08, integrate the complete feature set, review the whole delta and
   fix blocking findings. Then freeze the candidate and execute A16-09 once.
5. Publish only after explicit owner release Go; verify actual public assets,
   registry revision/digests and checksums before saying released. Retire only
   owned temporary artifacts after required evidence is retained.

No requirement to obtain manufacturer/KNX Association approval is introduced.
No worktree, original corpus, live installation or user container is changed by
this plan.

## Minimal, consolidated acceptance

**No tests or build gates are run to produce this plan.** This is not an accepted
release candidate. During implementation, add the necessary focused regressions
with each change; keep core tests mandatory and small, but collect their execution
in the final integrated package rather than repeatedly running complete suites
for every foundation, documentation edit, feature branch and merge.

Final acceptance is one ordered campaign on the frozen Alpha 16 candidate:

1. Review the complete delta and close blocking correctness/data-loss findings.
   Cheap formatting/type/build/documentation checks are not substitutes for tests;
   they may catch unfinished code without launching another full campaign.
2. Use the repository's maintained `tools/run_package_gate.py --full` on the
   actual frozen root. Its receipt must bind the candidate SHA, requested full
   scope, complete ordered stages, valid proofs, frozen inputs and untouched
   protected user data. Do not invent a parallel hand-maintained mega-gate.
3. Include only release-required lanes not already covered there: applicable
   opt-in real-source/corpus cases (including the exact System-2 source), newly
   added workflow tests, needed production/native configurations and admitted
   product-matrix checks. Select from actual discovery/runner inventories and
   deduplicate coverage; a guard-skip is not a pass.
4. Within that campaign exercise the integrated System-2 configured download,
   backup-before-setup, wrong-confirmation/target/stale-input refusal, interruption
   and explicit recovery; catalogue cancellation/commit/exit; comparison adoption
   plus undo; and catalogue-authored save/reopen with an empty product database.
   Reuse existing tests and fixtures instead of new overlapping test pyramids.
5. SHA-bound AppImage/MCP and Docker packaging dry runs may run alongside the
   source gate. Require build success and skipped publication jobs. Then smoke
   the actual artifacts in isolated homes/network, checking version, startup and
   one import/save/reopen/upgrade route; do not repeat the entire source suite
   inside every packaging format or architecture.

No routine mutation campaign, repeated independent rereview loop, extra benchmark
campaign or branch-plus-identical-merge full gate is scheduled. If acceptance
finds a real defect, preserve the failed attempt, fix the cause and rerun affected
checks. Changed release candidates still need source-bound acceptance and fresh
packaging where consumed inputs changed; “only once” never means shipping red or
crediting old results to new bytes.

## Release exit checklist

- [ ] A16-01–03 provide the complete bounded experimental System-2 workflow.
- [ ] A16-04–06 provide atomic catalogue, comparison and source-portability outcomes.
- [ ] A16-07 has an explicit implemented/excluded disposition for each admitted slice.
- [ ] A16-08 removes contradictory claims and documents actual upgrades/limitations.
- [ ] A16-09 has one complete, valid candidate acceptance and actual artifact evidence.
- [ ] Explicit owner publication Go precedes any tag/release/container publication.
- [ ] Public readback, checksums/digests, handover and owned cleanup are complete.

No checkbox is completed by creating this plan. Fewer gate laps, same brakes.
