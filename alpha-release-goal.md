# KNXBench goal — alpha-release readiness, without taking over UI or commissioning

**Created:** 2026-10-01 20:50 CEST.
**Planning baseline:** `origin/main` at `dfa0cc79ca50bc7da747058e97978c7e3a2de25c`, refreshed after the UI closure receipt published during planning.
**Input inventory:** [OFFENE_PUNKTE.md](OFFENE_PUNKTE.md), all 180 main-table IDs; its original audit revision and date remain unchanged.

Use **this file** as the instruction for the autonomous alpha-readiness session. Do not create or use a second `goals.md`. Creating this plan does **not** start implementation, authorize a hardware operation, approve a release tag or publish anything.

The outcome is an honestly scoped, tested Linux-first alpha candidate and an evidence-backed release decision, not the disappearance of every limitation. An accepted boundary is not a bug waiting to be secretly volunteered for overtime.

## 0. Scope and source of truth

This goal owns remaining **non-UI, non-commissioning** correctness, data-integrity, import/product-data, backend reporting, documentation, packaging and whole-product acceptance work derived from the inventory. It turns the list into small work packages with explicit prerequisites, evidence and stop boundaries.

- [`goal-ui.md`](goal-ui.md) remains the sole execution owner of its issues, their existing server/domain halves, U13 review findings, UI acceptance and the Web lock. **No U-package is copied into this queue.**
- [`goal-commission.md`](goal-commission.md) remains the sole execution owner of programming, download/recovery, commissioning research, their server/CLI/UI paths and hardware evidence. **No K-package is copied into this queue.**
- [`goal.md`](goal.md) remains historical/contextual evidence for the other backlog, accepted exclusions and decisions. When this new goal is explicitly started, use its AR packages as the execution queue for the overlapping non-UI/non-commissioning scope; do **not** simultaneously run a second `goal.md` executor on those same tasks. This file does not change either other track's instructions or approvals.
- [PROJECT_CONTEXT](docs/PROJECT_CONTEXT.md), current source/tests, ADRs and current maintained documentation win over the dated inventory, historical checkboxes and this planning snapshot. Never treat a branch name or old success count as current proof.
- Accepted DIN-26 exclusions, Secure/Functions/multi-user deferrals, unknown manufacturer semantics and withdrawn `.knxproj` export remain intact. This plan is not a blanket scope expansion.

### Current cross-track observation, not a permanent status

At the refreshed planning baseline, the UI owner has published its **U0–U13 closure receipt** (`dfa0cc79`, handover timestamp 2026-10-01 20:58 CEST). The user-approved independent GPT-6.1-Sol review replaced the unavailable Claude reviewer; its original **changes required** verdict remains archived, not relabelled as approval. The owner reports selection/detail identity, Undo/Redo refresh, autosave cleanup and offline discovery findings corrected and integrated gates green. Consume the dated receipt and its evidence; do not repeat those fixes. The final implementation review was an in-session self-review, not a second independent verdict. Native WebKitGTK, real-screen-reader and actual multicast boundaries remain disclosed. This planning task does not independently rerun or certify the UI owner's product gates.

Commissioning has subsequently hardened failed-download cleanup, worker outcome bookkeeping and backup-directory synchronization offline. Those fixes do not prove process-crash rollback, power-loss recovery, a complete device image or a new address-write authorization. Keep these changes in their original track, not on this queue.

## 1. Ownership and interface boundaries

| Surface or work | Execution owner | Alpha session's permitted involvement |
| --- | --- | --- |
| U0–U13, ISSUE-01–13 slices assigned to UI, including their already assigned server/domain work | `goal-ui.md` | Consume its integrated evidence; report newly found defects to that owner, not implement them twice. |
| `apps/knx-web`, messages, generated bindings, browser/native UI acceptance | `goal-ui.md`, under its Web lock | Read-only inspection and tests; propose a concrete handoff if backend work requires a new consumer. No bypass through “only generated” files. |
| Device programming, reset, download, partial selector, recovery, debug service-control writes, activity/history UI | `goal-commission.md` | Record its actual supported/refused boundary in release documentation; do not implement or run its actions. |
| Commissioning image placement, download flags/timing, RF, write-related protocol research and live cEMI evidence | `goal-commission.md` | Reference evidence or the named blocker. No formula guesses, live readback or re-dispatch of completed research. |
| General core ID exhaustion, command invariants, storage contracts, manufacturer import/reporting and backend projection | This goal, only outside another track's assigned package | Coordinate shared modules before editing; preserve public contracts and hand over any new UI-dependent behavior. |
| CSV/report policy, backend catalog-version policy, compatibility claims | This goal | Prepare or implement the approved data/backend half; disputed product-policy choices remain explicit decisions. |
| Triage/status reconciliation, manual acceptance, local alpha artifact, final whole-product review | This goal | Consume both owners' receipts, preserve their detailed evidence and do not substitute a product review for U13/K-track acceptance. |
| Statistics collector, private captures and historical Paperclip material | Separate tooling/private evidence | Use only permitted aggregate evidence and already authorized bookkeeping; no collector rewrite, publication of private inputs or revival of historical board tasks. |

Mixed items have **one primary route** in the appendix, not two implementation jobs. In particular:

- `DATA-01` is general allocator exhaustion outside the catalog's scoped fix. Catalog/editor regressions already owned by U13 stay there.
- `DATA-02` covers the general storage contract; AR04 must not redesign U12 editing. Whole-project save is the valid fallback unless incremental support is genuinely implemented and tested.
- `KL-146`, `KL-37` and parameter/localization work here cover imported data and backend projection only. Labels, controls and new UI consumers stay with UI; never mark a vertical feature complete without its owner's receipt.
- `KL-79` is primarily the UI discovery-acceptance dependency. Docker's multicast constraint is release documentation, not a second discovery-fix package.
- `KL-60` is a policy decision here; implementing any new diff presentation belongs to the UI owner. Existing paging is not silently called accepted.
- A reference to an optional owner follow-up is **not** a new task inserted into that owner's goal. Its existing completion contract still controls it.

## 2. Autonomous execution contract

### 2.1 Start and isolation

Before every package:

1. Read `docs/PROJECT_CONTEXT.md`, then `.agent-memory/PROJECT_MEMORY.md` as a read-only discovery aid. Read the newest `.ai/CURRENT_STATE.md`, `AGENTS.md`, `CLAUDE.md`, this goal, relevant ADRs and current implementation/limitations. Open only relevant linked memory notes.
2. Run `git status`, `git branch`, `git worktree list` and `git fetch origin`. Read **the current remote-tip** goal/handover/docs, not the dirty root's old tracked versions. Check active work and ownership before picking a task.
3. Start one owned `alpha-<topic>` worktree from freshly fetched `origin/main`, under `/mnt/daten-i/Sourcecode/KNXBench.worktrees/`. Keep scratch and per-worktree Cargo targets under `~/.hermes/profiles/knxbench/cache/scratch/alpha-release/`. Never reuse a gate binary built in a removed or different worktree.
4. Do not pull, reset, stash, rebase, clean or overwrite the shared dirty root merely to make it current. Do not modify `ui-*`, `iaw-*`, independent-review worktrees or another session's scratch/processes. Do not edit foreign uncommitted research, statistics, telemetry or shutdown documents.
5. For code gates, install/build the existing frontend in the owned tree (`npm ci`, then `npm run build` in `apps/knx-web`) before workspace Rust checks that need its resources. This is a build, not permission to change Web sources.
6. Provide private corpus access explicitly when needed: an ignored worktree link to the root's `OriginalData`, with `KNXBENCH_PRODUCT_CORPUS` pointing to the root's `OriginalData/ProductDatabases`. Verify the requested fixture and test actually ran. Missing data or a test's skip path is not evidence. Remove only your own links afterward.

### 2.2 Work loop

- Execute the earliest ready unchecked AR package. Order is risk/dependency driven; a blocked item does not halt unrelated offline work. Within one large package, split focused subpackages rather than mixing fixes.
- Inspect definitions and all callers first. For format/protocol claims, read primary specifications and current repository research before coding; record unresolved ambiguity instead of inventing a rule.
- Define the acceptance contract. Add a failing regression first for functional changes, implement the smallest clean fix, and test malformed input, unsupported fields, duplicate objects, invalid addresses, version mismatches, atomic failure and persistence where relevant.
- Review the complete candidate in a **separate in-session pass**. No `delegate_task` subagents. Check newly introduced guards with a realistic revert/mutation, restore the source, and prove the restoration. Compilation failure is not a meaningful caught behavioral mutant.
- Run the applicable gates, update focused docs/status, commit only the package's files as `KNXBench <github@knxbench.com>`, without a `Co-Authored-By` trailer. Concise, factual and slightly gloomy humor is welcome.
- On an explicitly started autonomous run, publish each reviewed, integrated and green package from an owned checkout. Inspect the outgoing range, fetch/reconcile current upstream, and gate the actual integrated candidate. Never publish another session's unreviewed commits from root `HEAD`.
- Read back the remote ref and exact published artifact before claiming publication. Write a newest-first handover, then clean only completed package-owned worktrees, branches, build targets and scratch.
- Continue immediately to the next ready package after delivery. Do not stop merely to ask “shall I continue?” The user monitors quota: no quota probes or quota questions. An explicit pause remains binding until the user's explicit go; an actual tool/service refusal is a real blocker.
- Use statuses `TODO`, `IN_PROGRESS`, `DONE`, `BLOCKED_EXTERNAL`, `WAITING_OWNER`, `WAITING_DECISION`, `ACCEPTED_BOUNDARY` or `LATER`. `DONE` requires a named test/artifact/review and revision. Do not close work based on an unchecked report, planned gate or optimistic narrative.
- Keep the progress checklist and the per-ID appendix current. Every blocked item records its missing input, responsible owner, safe fallback and exact unblock condition. Keep all accepted limits visible without treating them as required implementation.

### 2.3 Safety and authority

This session is **offline only**. No real KNX socket, discovery probe, scan, monitor, group send, property read/write, restart, address programming or hardware capture is authorized by this goal. Simulator/local fixture tests must explicitly use local adapters. Never contact or scan alarm-panel address `1.1.220`.

A candidate device, previous go, diagnostic dump, preserved property bytes or accepted unavailable vendor information is not a complete recovery contract. Never weaken a commissioning gate to make the alpha look more complete. No keys, passwords, corpus payloads, private device identities, host backup paths or raw telegrams belong in Git, reports or handovers.

Do not silently accept a new product-policy exception or reopen a previously accepted scope. Ordinary implementation choices are autonomous; reserved user decisions remain reserved. Batch independent questions at a real decision boundary and continue other ready work instead of asking after every package.

No release tag, hosted release, certificate, signing claim, CLA change, license change, dependency upgrade or system/firewall change is authorized by a generic continuation. Obtain a scoped decision where required. Never claim full ETS compatibility, KNX certification or untested platform support.

### 2.4 Handover and bookkeeping

Each significant package writes `.ai/logs/YYYY-MM-DD_codex_alpha-<topic>.md` or the equivalent `_claude_` filename for its actual agent. Prepend `.ai/CURRENT_STATE.md` with the existing fields: **Last Agent**, **Timestamp** from `date`, **Completed**, **Pending/Next Steps**, **Notes for Codex oder Claude**. Preserve both owners' entries and any active lock; never write “Web lock released” unless this session held it.

Address cross-track findings under **For the UI session:**, **For the commissioning session:** or **For the alpha-release session:**. A handoff names the source IDs, exact data/API contract, landed revision, tests and remaining acceptance; a DTO without a consumer is not a finished feature.

For tracked/ignored `.ai/` files, stage only explicitly owned paths (`git add -f` where required), not the entire foreign handover. Integrate shared-doc changes at merge time, keep both sides and retain old fragment anchors when headings change.

The existing statistics refresh requirement remains bookkeeping, not permission to change the external collector. A refresh must describe the actually integrated history and have a real successful generator result. If the canonical root is dirty/behind or the collector is unavailable, coordinate with its owner and record the blocker; never reset that root, generate a misleading report from its stale history or fabricate metrics. Do not make instrumentation work from `TOOLS-*` an alpha prerequisite.

## 3. Ordered work packages

**Execution started:** 2026-10-01, user request `arbeite alpha-release-goal.md ab`.
AR00 is `DONE` on the isolated `alpha-queue` checkout at baseline
`307a5970`; per-ID evidence, decision provenance, stable unnumbered aliases
and safe fallbacks are in [ALPHA_READINESS](docs/ALPHA_READINESS.md).
No other `goal.md` executor was found in the startup process/worktree check;
that file now explicitly points here rather than dispatching overlapping work.
Evidence: `.ai/logs/2026-10-01_codex_alpha-queue.md`, verified 180-ID
priority/route/owner ledger, 110-heading/105-residue recount, and fresh-target
anchor gate. This is documentation/source inspection, not a new product test run.
AR01 is `DONE`: runtime target/coverage guards, 87 xtask tests, five rejected
behavioral mutants and emitted candidate scope; see [verification](docs/VERIFICATION.md)
and `.ai/logs/2026-10-01_codex_alpha-gate-scope.md`. AR00 was published as
`6f4cef24` with remote/artifact readback and its owned checkout removed.
AR02 is `DONE`, published as `e691bc1318d0785289f8132378a0f26c9a829b27`:
all nine checked allocators,
cross-layer refusal/atomicity/native boundary regressions and three caught
behavioral mutants. Final stable-source gates: 141 Rust result blocks,
2,855 passed / zero failed / 161 ignored; 77 explicitly executed private
offline corpus/roundtrip tests; Web 1,312, strict Clippy, dependency policy,
typecheck, build and repository gates green. Receipt:
`.ai/logs/2026-10-01_codex_alpha-id-exhaustion.md` (2026-10-01 UTC receipt).
Its remote ref/artifact matched the gated tree; owned checkout, branch and
AR02 targets/scratch are removed. AR03's bounded source audit and proposed
scope are documented in [the enforcement audit](docs/ADR0039_ENFORCEMENT_AUDIT.md).
AR03 remains `WAITING_DECISION`; AR04 is published as
`216c673e7c32a4bd82a308e06544a4fd239d7b3f` with exact remote/artifact readback.
Next ready work is AR05. The activation
prompt has no recorded answer; empty input is not consent or an accepted deferral.
The canonical-root statistics refresh is blocked by foreign local report work;
this does not block unrelated offline packages and is not a fabricated refresh.

The original plan started with all checkboxes open. Checked steps below now
refer to actual execution evidence, not planning success. A package can close
as verified correction, delivered scoped code, explicit accepted boundary or
recorded external prerequisite **only under the status rules above**. Package
closure does not automatically mean every source limitation was removed.

### AR00 — Establish the live queue and remove stale dispatch instructions

**Sources:** `DOC-01`, `DOC-02`, `TOOLS-06`, plus every appendix route.
**Mode:** offline documentation/evidence. **Dependencies:** none.

- [x] Reconcile each source ID against current code, status, ADRs and both owners' latest receipts; preserve the dated input inventory rather than pretending it was a new audit.
- [x] Verify the already-fixed `KL-18`, `KL-23`, `KL-24` and ISSUE-04 assertions from their implementations/tests, then correct stale non-owner goal/manual dispatch text with retained historical provenance. Do not reimplement streaming, file gestures or completed PDB/UI packages.
- [x] Record stable IDs for both §130 entries and unnumbered limits. Recount triage mechanically without destructive renumbering or broken fragment links.
- [x] Classify `ACCEPTED` and `LATER` appendix entries using their actual decision/evidence. If a purported acceptance lacks provenance, leave `WAITING_DECISION`; do not manufacture consent.
- [x] Record current UI/commissioning dependencies as owner receipts, not local tasks. Establish that no old `goal.md` executor is simultaneously editing the same alpha-owned scope.

**Exit evidence:** complete 180-ID routing/decision ledger, mechanically verified counts, corrected actionable queue and doc gates. No release-ready claim.

### AR01 — Make verification target and coverage trustworthy

**Sources:** `KL-130-GATE`. **Dependencies:** AR00.

- [x] Reproduce the wrong/removed-worktree false-green case in isolated gate tests.
- [x] Implement the smallest explicit target/coverage guard or documented invocation contract that refuses a missing target or unexpectedly empty source set; do not embed today's counts as magic constants.
- [x] Test valid tree, wrong tree, deleted tree, empty scan and intentional fixture scopes. Show changed gate code was compiled and that the emitted scope matches the candidate.

**Exit evidence:** named RED/GREEN regression, rejected guard mutant, non-empty per-gate scope and up-to-date limitation wording. A zero exit inspecting zero files is not green.

### AR02 — Refuse ID exhaustion atomically on general creation paths

**Sources:** `DATA-01`. **Dependencies:** AR01.

- [x] Enumerate the current allocator methods and all non-catalog callers; preserve the scoped catalog safeguard and U13-owned fixes.
- [x] Add explicit checked exhaustion behavior with typed errors and no wrap, panic, overwritten entity or partially consumed batch. Respect ADR-0039's monotonic high-water marks.
- [x] Verify each affected entity kind at the maximum representable ID, batch rollback, undo/redo, save/reopen and caller error propagation. Keep malformed imported counters separate from new allocation.

**Exit evidence:** per-kind boundary regressions and cross-layer refusals; domain/application fixes rather than a UI-only guard.

### AR03 — Decide and, only when activated, finish ADR-0039 enforcement

**Sources:** `KL-129`. **Dependencies:** AR02. **Initial status:** `WAITING_DECISION` for the previously parked public-surface work.

- [x] Audit the current implementation against ADR-0039 phases 3–5. Do not repeat phases 1–2 or claim duplicate-ID data loss is still open.
- [x] Record the smallest proposed activation scope and the reason to unpark it. The accepted design is evidence, but this plan does not erase the old parked status by inference.
- [ ] Once activation is explicitly recorded, implement phase 3, phase 4 and phase 5 as separate reviewed packages: eliminate live bypasses, seal only the agreed allocator surface, then add the source-mutation gate with exact exemptions.
- [ ] Preserve detached import construction and source data; verify seed enrichment equality, lock/plan/apply races, monotonic reservation, undo/redo, native roundtrip and positive/negative gate fixtures.

**Exit evidence:** recorded activation or accepted continued deferral; if activated, per-phase tests/gates and the documented heuristic-versus-type-system boundary. No full-project sealing refactor or persisted undo history is smuggled in.

**Audit receipt:** [ADR-0039 enforcement audit](docs/ADR0039_ENFORCEMENT_AUDIT.md)
at published AR02 `e691bc13`: six direct live allocator calls, detached catalog
allocation with a remaining single-create assignment, and post-command seed
enrichment. Historical bypass counts are not current proof. Private `Project.ids`
and the mutation gate remain unimplemented. Activation question unanswered;
`KL-129`/AR03 stay `WAITING_DECISION`, with no runtime test or acceptance claim
from this docs-only audit. Continue independent AR04.

### AR04 — Make storage guarantees match actual command coverage

**Sources:** `DATA-02`, `KL-42`. **Dependencies:** AR02; coordinate any AR03 surface changes.
**Status:** `DONE`; implementation published as
`216c673e7c32a4bd82a308e06544a4fd239d7b3f`, exact remote/artifact and author
policy verified. Baseline `4f47059c`. [Storage contract](docs/STORAGE_COMMAND_CONTRACT.md).

- [x] Trace every relevant structural command through `command_sync`, whole-project save, undo/redo and reopen. Identify unsupported incremental cases explicitly.
- [x] Choose the smallest safe behavior: verified incremental synchronization or an explicit whole-project-save fallback. Never make an unsupported command appear durably saved by an incremental success.
- [x] Test injected failures, reopened relationships/order, opaque metadata, rollback and undo/redo. Update overstated module documentation.

**Exit evidence:** an explicit coverage contract and persistence regressions. UI editor work remains owned by U12; alpha does not change its controls or multi-installation scope.
Parameter RED/GREEN, nine focused regressions, three file-backed tests, separate
in-session review and three compiled behavioral mutants are measured; all source
hashes restored. Source enumeration is 50 variants, not per-variant runtime
certification. Complete coordinated gates exited 0: 142 Rust suites, 2,859
passed / zero failed / 161 ignored / zero skip markers; private store 2/2,
Web 1,312; strict Clippy, type/build/bindings/dependency and repository gates
green. Source hashes are stable; implementation publication verified, closing
documentation/cleanup receipt follows.
Receipt
`.ai/logs/2026-10-01_codex_alpha-storage-contract.md`.

### AR05 — Close manufacturer-report omissions without rewriting history

**Sources:** `PDB-09`, `KL-86`, `KL-87`, `PDB-06`, `PDB-07`, `PDB-11`.
**Dependencies:** AR01; relevant current product-store schema.

- [ ] Add corpus-observed handling/reporting for master `Languages` attributes, preserving bytes and reporting unknowns instead of silently declaring them known.
- [ ] Audit duplicate normalized IDs and within-file DPT provenance; distinguish retained source, chosen normalized value and lost/uninterpreted semantics.
- [ ] Make bounded Dynamic diagnostics disclose truncation/budget effects rather than claiming exhaustive subordinate coverage.
- [ ] Test upgrades and explicit re-derivation from stored blobs. Preserve historical installation reports; keep metrics unavailable for pre-ledger installs instead of backfilling guessed zeros.

**Exit evidence:** malformed/unknown/duplicate/budget regressions, upgrade tests and explicitly executed private corpus cases with aggregate-only evidence. No regeneration is allowed to falsify the original install report.

### AR06 — Harden supported import boundaries and truthful unsupported-format errors

**Sources:** `KL-1`, `KL-11`, `KL-125`, `KL-128`, `KL-15`, `IMPORT-06`, `PDB-08`, `PDB-10`.
**Dependencies:** AR05 where reports/migrations intersect.

- [ ] Verify current schema/namespace detection, mapping and atomic refusal. Separate `.knxproj` evidence from `.knxprod` evidence and independent installations from reexports of the same installation.
- [ ] Preserve unreadable attributes and untyped master/version lexemes where technically possible, or report their exact boundary. Do not present raw Secure capacities/MinEtsVersion/ReplacesVersions as tested abilities.
- [ ] Improve misleading VD3–VD5/PR3–PR5 rejection text without implementing their deferred importer or enabling VD2. Maintain explicit unsupported diagnostics and native-data integrity.
- [ ] Verify DefaultLine diagnostics and device-local communication-object mapping with synthetic regression cases and existing authorized corpus. New genuine project-schema/model evidence remains externally blocked until available.

**Exit evidence:** bounded compatibility/import report, malformed-input and atomicity tests, native save/load evidence and an explicit sample matrix. Missing independent samples remain `BLOCKED_EXTERNAL`, not “compatible”.

### AR07 — Research and validate supported parameter semantics, never execute unknown vendor logic

**Sources:** `KL-3`, `KL-146`, `PDB-01`, `PDB-02`, `PDB-03`, `PDB-05`, `R-DYNAMIC-01`, `R-MODULE-03`, `R-MODULE-04`.
**Dependencies:** AR05/AR06. **Mode:** bounded offline research first.

- [ ] Compare current handling with primary, available schema/specification evidence and existing packages; separate supported behavior, unexpanded Repeat, raw allocator/calculation data and unknown activation from actual correctness defects.
- [ ] Fix only semantics demonstrably established by those sources, with an ADR before any new domain/storage contract. Preserve opaque forms and warn on unsupported constructs; no guessed RepeatIndex, placement formula, DPT or visibility.
- [ ] Verify activation-budget/refusal behavior, nested/duplicate module identities and parameter validation; retain safe read-only behavior when evidence is insufficient.
- [ ] Improve data/projection provenance for channel names and indeterminate activation without inventing missing manufacturer texts. Hand presentation changes to UI.

**Exit evidence:** research decisions and supported-semantics regressions or exact external blockers. ParameterCalculation scripts, Button handlers, DLLs and unknown vendor code are not executed; a complete Repeat/Allocator engine is not authorized merely because it appears in the inventory.

### AR08 — Complete the safe data/application half of supported password import

**Source:** `KL-13`. **Dependencies:** AR06 and existing secret-handling conventions.

- [ ] Trace the existing ZipCrypto reader into application and CLI entry points; add only missing approved plumbing, not a second cipher implementation.
- [ ] Test correct/wrong/missing password, corrupted data, decompression/size bounds, nested archives, failed-import atomicity and native roundtrip. Passwords must not enter persistent project data, process arguments, logs or compatibility reports.
- [ ] Hand any new password dialog to UI rather than editing it here. Record whether the entry path is library-, CLI-, application- or end-to-end verified.
- [ ] Leave AES project decryption sample-gated and encrypted manufacturer packages unsupported. Do not enable AES or make a real-ETS claim from synthetic success alone.

**Exit evidence:** supported entry-path tests and redaction checks, with genuine ETS/AES evidence gaps still visible. Secret transport must follow the repository's verified mechanism; if none exists for an intended surface, document/design it first.

### AR09 — Verify DPT/model fidelity without changing wire rulings by guesswork

**Source:** `KL-61`. **Dependencies:** AR06/AR07 for imported type provenance.

- [ ] Audit implemented types and observable encoding rulings against current primary evidence. Fix proven format defects only; keep format validation distinct from unverified subtype tables.
- [ ] Investigate explicitly declared group-address DPT versus linked-object inference in supported project schemas. If modeling/resolution changes are justified, write an ADR and migration/roundtrip contract first.
- [ ] Test explicit, inferred, conflicting, missing and unsupported types, retained raw payload and error propagation. Never silently override a conflict or discard a source declaration.

**Exit evidence:** model/import/native roundtrip and codec tests, or a documented prerequisite. No LTE/200-series scope expansion, scene-display change in UI or real group write.

### AR10 — Finish evidence-backed backend localization paths

**Sources:** `KL-14`, `KL-37`, `KL-64`, `KL-66`.
**Dependencies:** AR06/AR07 and established language-pack contracts.

- [ ] Trace source/default language, manufacturer/master translations and backend diagnostics into their current readers and projection.
- [ ] Use a source-backed language when available; otherwise expose the fallback rather than inferring one from unrelated UI settings or installation names.
- [ ] Add language-aware lookups/structured diagnostics only on established paths, preserving original text, missing-translation behavior and raw diagnostic details.
- [ ] Test both installed languages, absent/ambiguous translations, existing native migrations and fallback reproducibility. Send required new catalogue/rendering work to UI under its ownership contract.

**Exit evidence:** data/application/CLI localization regressions and clear boundary wording, not a “fully localized” claim. A missing frontend consumer keeps that portion waiting on its owner.

### AR11 — Resolve remaining CSV/report decisions and verify the selected backend scope

**Sources:** `KL-38`, `KL-40`, `KL-44`, `KL-47`, `KL-51`, `KL-60`.
**Dependencies:** AR06; AR07 for supported parameter output.

- [ ] Reproduce current behavior and provide a bounded decision for the unaccepted §40 and §60 residues. Recommended default: derived counts stay read-only; add source-backed Description/Comment roundtrip only under an approved schema/CSV contract; retain explicit paging unless the UI owner adopts a measured alternative.
- [ ] Obtain the reserved scope decision before calling either residue accepted or implementing a behavior change. Continue other packages while awaiting it.
- [ ] Implement and test only the approved backend slice: malformed/duplicate CSV, address conflicts, atomic preview/apply, undo/redo, native roundtrip and no hidden lossy columns.
- [ ] Keep raw module/AllocatorRef output explicitly warned where semantics are unknown. Document HTML/diff/custom-CSV scope without claiming ETS, ESF/OPC or layout parity.

**Exit evidence:** recorded decisions, approved slice tests and a UI handoff for any new display. DIN-26 CSV ranges, native PDF, prose catalog and diff application/correlation exclusions stay accepted, not reopened.

### AR12 — Resolve package-version policy before implementing selection

**Source:** `KL-135`. **Dependencies:** AR05/AR06. **Initial status:** `WAITING_DECISION`.

- [ ] Enumerate actual stored package/application versions and conflicts, current winner policy and effects on existing project references.
- [ ] Propose the smallest explicit version/pinning choice; record a user-approved policy or continued deferral. Do not automatically replace project applications or reinterpret raw ReplacesVersions.
- [ ] If approved, implement only its versioned data/application contract, with compatibility/conflict/rollback/migration tests and no manufacturer hard-coding.
- [ ] Hand user selection UI to its owner. A backend selector without its agreed consumer remains partial, not a delivered whole feature.

**Exit evidence:** explicit policy and tests/owner receipt where activated. No speculative online catalog or firmware/device update.

### AR13 — Verify privacy and the actual deployment-security boundary

**Sources:** `KL-106`, `KL-22`, `KL-65`.
**Dependencies:** AR01; can run independently of UI/commissioning acceptance.

- [ ] Audit debug/report export against bounded privacy-pattern classes and synthetic sensitive fixtures, not one remembered address. Ensure newly handled fields are redacted or explicitly warned without claiming perfect anonymization.
- [ ] Verify existing server authentication/bind refusal and documented reverse-proxy/TLS requirements offline. Keep a shared password distinct from roles, multi-user isolation and public-Internet safety.
- [ ] Make artifact version/provenance accurately describe its built revision; test any adopted dirty-build marker rather than claiming version output proves a clean source tree.

**Exit evidence:** privacy/authentication/provenance regressions and deployment/privacy checklist. No automatic TLS service, user-role system, new license regime, credential disclosure or host configuration change.

### AR14 — Verify existing non-commissioning bus/CLI contracts offline

**Sources:** `KL-29`, `KL-31`, `KL-62`, `KL-72`, `KL-73`, `KL-74`, `KL-75`, `KL-76`, `KL-77`, `KL-78`, `KL-126`, `KL-102`.
**Dependencies:** AR09 for DPT changes; otherwise independent.

- [ ] Verify supported CLI address/name formatting and existing routing/scan/monitor contracts with fakes and local adapters. If a case is already assigned to U13/ISSUE-12 or another owner, consume its test instead of rebuilding it here.
- [ ] Test truthful busy/vacant/unknown results, timeout boundaries, exclusion enforcement and reconciliation preserving project identity; occupancy is not product identity.
- [ ] Maintain bounded one-tunnel/one-line scope, uncertain retries and custom multicast evidence gaps. Real gateway/custom-group evidence is external and belongs to an authorized owner, not this goal.
- [ ] For an unreachable public error branch, document that reachability boundary and test only genuinely reachable behavior; do not fabricate a wire event to tick a coverage box.

**Exit evidence:** offline contract/regression dossier or retained documented boundaries. No real bus run, multi-tunnel redesign, automatic coupler traversal, scan speed promise or new hardware support. Monitor UI and discovery acceptance remain external owner work.

### AR15 — Reconcile release documentation and limitations on the finished scope

**Sources:** `DOC-03`, `KL-9`, `KL-16`, `KL-46`, plus all earlier results and accepted/later routes.
**Dependencies:** completed/explicitly blocked AR00–AR14; newest owner receipts.

- [ ] Reconcile `IMPLEMENTATION_STATUS`, `KNOWN_LIMITATIONS`, `LIMITATION_TRIAGE`, `ROADMAP`, `GAP_ANALYSIS_ETS`, `COMPATIBILITY`, `IMPORT_EXPORT` and the relevant model/architecture docs with actual source/tests.
- [ ] Recount limitations and derived tables programmatically; preserve duplicate-ID disambiguation, historical anchors and new entries published by either other track.
- [ ] Document native SQLite versus text-diff limits, product-dependent report names, GTK3 platform dependency and actual deployment/import/hardware boundaries. Withdraw resolved prose, not remaining evidence gaps.
- [ ] Produce the alpha scope/risk/decision matrix: supported and verified, simulator-only, externally blocked, accepted boundary and later work. Every unaccepted blocker gets a release disposition, not a hidden waiver.

**Exit evidence:** mutually consistent release claims, full source-ID disposition and doc gates. This dossier can advance while a final owner receipt is pending; final acceptance cannot.

### AR16 — Accept the existing manual after UI closure

**Source:** `RELEASE-03`. **Dependencies:** AR15 and integrated U13 closure receipt.

- [ ] Verify the current UI owner's closure, actual tested surfaces and any native/accessibility exceptions; do not substitute a self-review or an old unsuccessful invocation.
- [ ] Resolve the reserved manual location/screenshot decision consistently with ADR-0024: maintainer `docs/` does not silently become a bundled in-app help system.
- [ ] Review the existing manual claim by claim against the finished application, update workflows/screenshots at their actual tested scope, and remove stale “never written”, browser-export and count statements without broadening hardware claims.
- [ ] Record explicit manual acceptance and unresolved user-owned exceptions.

**Exit evidence:** dated manual checklist and approved location/screenshot policy. Missing UI/native/live evidence remains marked, not filled with invented screenshots or observations.

### AR17 — Build and inspect the local Linux alpha candidate

**Sources:** packaging part of `RELEASE-04`; no new inventory task implied.
**Dependencies:** AR15/AR16 and the integrated code candidate.

- [ ] Follow the repository's existing AppImage workflow/ADR-0021, using the recorded toolchain and current build commands rather than guessed installation steps.
- [ ] Build an actual Linux x86_64 artifact, run `xtask check-appimage` with its real artifact directory and record the built revision, version, digest and result.
- [ ] Perform authorized **offline** smoke checks: launch, create/open/save/reopen, clear failure messages and no backend/device auto-contact. Native UI checks themselves remain coordinated with the UI owner; consume its verified result rather than claiming them from headless Chromium.
- [ ] Check packaging for private corpus, secrets and accidental maintainer-doc bundling. Record exactly the Linux environment tested.

**Exit evidence:** actual local artifact, validator/smoke output and narrow packaging manifest. No tag, upload or claim of all-distribution/platform support; this is a candidate, not a release.

### AR18 — Final integrated gates and independent whole-product review

**Sources:** `RELEASE-01`, `RELEASE-02`.
**Dependencies:** AR15–AR17, integrated U13 receipt, commissioning's dated scope/safety receipt, and decisions for every unresolved alpha-owned blocker.

- [ ] Freeze the complete integrated candidate and artifact provenance. All actionable alpha-owned items are delivered, or their release exceptions have explicit user acceptance. Merely logging a blocker is not permission to call the alpha ready.
- [ ] Run all explicit gates in §5 on that exact integrated revision, including private corpus cases that cover affected behavior; verify target, counts, changed-crate compilation and no silent skips.
- [ ] Obtain the separately chosen **independent whole-product** source/test/artifact review. U13's independent UI review is necessary but not a substitute for this review. Do not use a forbidden subagent or count an unavailable reviewer invocation/self-review as a verdict; obtain the user's reviewer/result decision if necessary.
- [ ] Fix blocking findings in the owning track, integrate their receipts and rerun affected/full final gates and review as required. A reviewed old candidate is not evidence for a new candidate.
- [ ] Record ready/not-ready, accepted remaining boundaries, test counts, exact revision/artifact and review verdict. If only an external gate remains, stop with its precise owner/unblock action.

**Exit evidence:** final integrated gate dossier and independent verdict with no unresolved blocking finding. This is the **last readiness stage**; any new feature or fix afterward invalidates the affected evidence and returns here.

### AR19 — User-controlled release decision; never auto-tag

**Source:** `RELEASE-04`. **Dependencies:** AR18.

- [ ] Present the exact candidate, tested scope, manual, remaining accepted limitations and independent review result. Request the user's explicit tag/version/publication decision once.
- [ ] If approved, follow the repository's version/tag/artifact procedure and verify the published target/artifact. If declined or deferred, record `READY_NOT_PUBLISHED` or `WAITING_DECISION`; do not invent approval.

**Exit evidence:** a recorded release decision, and only if authorized an actual verified publication. A “go” to continue alpha hardening is not consent to create a hosted release.

## 4. Accepted boundaries, later work and external prerequisites

The appendix accounts for every main-table ID. It is a routing ledger, **not 180 new implementation tasks**.

- `ACCEPTED`: preserve the recorded boundary and its public limitation. AR00 checks provenance; AR15/AR18 ensure release claims respect it. No silent reopening of DIN-26, Secure, multi-user, plugins, spatial canvas, platform or logo decisions.
- `LATER`: optional/new-scope/tooling work is visible but not on the alpha execution queue. Project notes, selective imports, historical backups, automation/MCP, Functions, online services, extra humor and collector development need their own scope and, where relevant, ADR.
- `UI`: owner-only reference, including current acceptance and future owner-scoped residue. Only that owner decides whether an optional follow-up enters its queue. Alpha consumes evidence/accepted limitations rather than demanding every optional UI extension as a release prerequisite.
- `COMMISSIONING`: owner-only reference, including all P0 hardware gates and unfinished research. The alpha can be scoped with these actions unavailable; **only explicit release acceptance** can admit a still-blocked safety feature as a documented unavailable boundary. Its absence is not permission to relax the gate or require unauthorized live experimentation.
- `ARxx`: owned work or bounded investigation. Source evidence may still leave part `BLOCKED_EXTERNAL`/`WAITING_DECISION`; package mapping is not an assertion that undocumented semantics are implementable.

The pre-main-table solved/withdrawn items in `OFFENE_PUNKTE.md` remain non-tasks: `KL-18`, `KL-23`, `KL-24`, `KL-90`, `KL-95`, withdrawn ETS export/reimport, reclassified GAP-T30-05/-06/-10, closed ISSUE-04, old archiving advisories and settled licensing. AR00/AR15 may correct misleading references, never resurrect their old implementation queues.

## 5. Verification contract

### Documentation/planning-only changes

Run `git diff --check`, the fresh-worktree-built `xtask check-anchors`, and the 180-ID/owner/table validation. No full Rust/Web product suite or hardware run is claimed for merely writing a plan.

### Functional packages and final candidate

Build the frontend resources first in the owned checkout. Run the actual commands, judge exit codes **and** emitted coverage, and record ignored/skipped tests separately:

| Gate | Required verification |
| --- | --- |
| Rust formatting | `cargo fmt --all -- --check` |
| Strict Rust lint | `cargo clippy --workspace --all-targets -- -D warnings` |
| Workspace tests | `cargo test --workspace --no-fail-fast` |
| Architectural layering | `cargo run -p xtask -- check-layering` |
| Source headers | `cargo run -p xtask -- check-headers`; current configured ceiling, not an old number copied here |
| Documentation anchors | `cargo run -p xtask -- check-anchors`; non-empty target coverage |
| Corpus-gating conventions | `cargo run -p xtask -- check-corpus-gates` |
| Dependency/security policy | `cargo deny check` |
| Frontend compatibility | In `apps/knx-web`: `npx tsc --noEmit`, `npx vitest run`, `npm run build` |
| Patch integrity | `git diff --check`, explicit staged/outgoing scope |
| Private corpus regressions | Named affected tests explicitly included, fixture/env verified, skip path rejected |
| Artifact | AR17/AR18: `cargo run -p xtask -- check-appimage --artifact-dir <actual-directory>` and actual offline smoke evidence |
| Mutation enforcement | Once AR03 phase 5 actually exists: its verified new xtask gate; do not invoke an invented current command |

The old “nine gates” wording is not a reason to omit corpus policy, build prerequisites, whitespace, the artifact or a newly introduced invariant gate. Documentation-only packages use the narrower contract above.

Only one workspace Cargo gate runs at a time across sessions. Coordinate the start mechanically using a common advisory gate lock under the Git common directory, and check for existing non-cooperating Cargo gates before starting; do not race a printed process check against another start, kill their process or wait on a shell pattern that matches itself. Use a fresh **per-worktree** target so `env!("CARGO_MANIFEST_DIR")` does not audit somebody else's checkout. Run relevant integration/corpus checks again after upstream reconciliation; feature-branch success does not prove the combined tree.

Never run every ignored test indiscriminately: some are hardware-writing tests. Include only explicitly identified offline/private-fixture cases and ensure no gateway/device environment accidentally activates live paths.

## 6. Completion and continuation

Distinguish these terminal states:

1. **Offline work exhausted, readiness blocked:** all ready alpha-owned packages are delivered; named owner/external/user decisions remain. Report exact blockers and continue only when they resolve. This is **not** alpha readiness.
2. **Alpha-ready, not published:** AR18 is green for the actual candidate, both tracks' relevant receipts/accepted boundaries and manual/release exceptions are settled, and the independent whole-product review has no blocking findings. AR19 may still wait for publication approval.
3. **Published alpha:** the user explicitly approved the exact release action and its target/artifact have been read back and verified. Do not claim this from a local AppImage or a version string.

Do not use a completion percentage for unequal tasks. Report completed packages, current verification, owner/external blockers, accepted limitations and the next ready package or exact unblock action. Once this goal is explicitly started, a normal green package boundary is not a stop boundary.

## 7. Complete source-ID routing ledger

The following table is the authoritative one-primary-route map for the 180 main-table entries in `OFFENE_PUNKTE.md`. Priority is copied from that inventory, not recomputed. The inventory contains the corresponding descriptions and exact source paths. Preserve IDs when updating status; new post-snapshot findings receive their own documented identity and do not silently change the input count.

**Current execution state:** AR00 reconciliation on baseline `307a5970`;
[ALPHA_READINESS](docs/ALPHA_READINESS.md) records each row's evidence and
unblock contract. Owner references and accepted/later entries are not unchecked
implementation jobs here. U13/ISSUE-12's dated tasks are done, while their
native/accessibility/multicast and optional boundaries remain disclosed.
Technical rulings for KL-70/88/134 are not silently upgraded to user release
waivers; FUTURE-05 is unscheduled (`LATER`), not an accepted release exception.

| Source ID | Priority | Primary route | Current status |
| --- | --- | --- | --- |
| `KL-116` | P0 | `goal-commission.md` — owner only | WAITING_OWNER |
| `KL-139` | P0 | `goal-commission.md` — owner only | WAITING_OWNER |
| `KL-140` | P0 | `goal-commission.md` — owner only | WAITING_OWNER |
| `SAFE-01` | P0 | `goal-commission.md` — owner only | WAITING_OWNER |
| `KL-79` | P1 | `goal-ui.md` — owner only | WAITING_OWNER |
| `KL-61` | P1 | AR09 | TODO |
| `KL-99` | P1 | `goal-commission.md` — owner only | WAITING_OWNER |
| `KL-112` | P1 | `goal-commission.md` — owner only | WAITING_OWNER |
| `KL-136` | P1 | `goal-commission.md` — owner only | WAITING_OWNER |
| `KL-138` | P1 | `goal-commission.md` — owner only | WAITING_OWNER |
| `KL-141` | P1 | `goal-commission.md` — owner only | WAITING_OWNER |
| `KL-142` | P1 | `goal-commission.md` — owner only | WAITING_OWNER |
| `KL-7` | P1 | `goal-commission.md` — owner only | WAITING_OWNER |
| `KL-92` | P1 | `goal-commission.md` — owner only | WAITING_OWNER |
| `DEBUG-01` | P1 | `goal-commission.md` — owner only | WAITING_OWNER |
| `SAFE-02` | P1 | `goal-commission.md` — owner only | WAITING_OWNER |
| `SAFE-03` | P1 | `goal-commission.md` — owner only | WAITING_OWNER |
| `DATA-01` | P1 | AR02 | DONE |
| `KL-129` | P1 | AR03 | WAITING_DECISION |
| `KL-106` | P1 | AR13 | TODO |
| `DOC-01` | P1 | AR00 | DONE |
| `KL-1` | P1 | AR06 | BLOCKED_EXTERNAL |
| `KL-13` | P1 | AR08 | TODO |
| `PDB-09` | P1 | AR05 | TODO |
| `R-MODULE-01` | P1 | `goal-commission.md` — owner only | WAITING_OWNER |
| `KL-130-GATE` | P1 | AR01 | DONE |
| `RELEASE-01` | P1 | AR18 | WAITING_OWNER |
| `RELEASE-02` | P1 | AR18 | WAITING_OWNER |
| `KL-22` | P1 | AR13 | TODO |
| `KL-63` | P1 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY |
| `KL-8` | P1 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY |
| `UI-01` | P1 | `goal-ui.md` — owner only | DONE |
| `UI-02` | P1 | `goal-ui.md` — owner only | DONE |
| `KL-126` | P2 | AR14 | TODO |
| `KL-29` | P2 | AR14 | TODO |
| `KL-31` | P2 | AR14 | TODO |
| `KL-62` | P2 | AR14 | TODO |
| `KL-72` | P2 | AR14 | TODO |
| `KL-73` | P2 | AR14 | TODO |
| `KL-74` | P2 | AR14 | TODO |
| `KL-75` | P2 | AR14 | TODO |
| `KL-77` | P2 | AR14 | TODO |
| `KL-78` | P2 | AR14 | TODO |
| `KL-105` | P2 | `goal-commission.md` — owner only | WAITING_OWNER |
| `KL-108` | P2 | `goal-commission.md` — owner only | WAITING_OWNER |
| `KL-101` | P2 | `goal-commission.md` — owner only | WAITING_OWNER |
| `KL-104` | P2 | `goal-commission.md` — owner only | WAITING_OWNER |
| `KL-109` | P2 | `goal-commission.md` — owner only | WAITING_OWNER |
| `KL-111` | P2 | `goal-commission.md` — owner only | WAITING_OWNER |
| `KL-113` | P2 | `goal-commission.md` — owner only | WAITING_OWNER |
| `KL-114` | P2 | `goal-commission.md` — owner only | WAITING_OWNER |
| `KL-143` | P2 | `goal-commission.md` — owner only | WAITING_OWNER |
| `KL-144` | P2 | `goal-commission.md` — owner only | WAITING_OWNER |
| `KL-145` | P2 | `goal-commission.md` — owner only | WAITING_OWNER |
| `KL-93` | P2 | `goal-commission.md` — owner only | WAITING_OWNER |
| `GAP-T30-01` | P2 | `goal-commission.md` — owner only | WAITING_OWNER |
| `GAP-T30-02` | P2 | `goal-commission.md` — owner only | WAITING_OWNER |
| `GAP-T30-03` | P2 | `goal-commission.md` — owner only | WAITING_OWNER |
| `GAP-T30-07` | P2 | `goal-commission.md` — owner only | WAITING_OWNER |
| `GAP-T30-08` | P2 | `goal-commission.md` — owner only | WAITING_OWNER |
| `GAP-T30-09` | P2 | `goal-commission.md` — owner only | WAITING_OWNER |
| `R-DL-01` | P2 | `goal-commission.md` — owner only | WAITING_OWNER |
| `R-DL-02` | P2 | `goal-commission.md` — owner only | WAITING_OWNER |
| `DATA-02` | P2 | AR04 | TODO |
| `DATA-03` | P2 | `goal-ui.md` — owner only | WAITING_OWNER |
| `KL-87` | P2 | AR05 | TODO |
| `AUDIT-01` | P2 | `goal-commission.md` — owner only | WAITING_OWNER |
| `KL-137` | P2 | `goal-ui.md` — owner only | WAITING_OWNER |
| `KL-36` | P2 | `goal-ui.md` — owner only | WAITING_OWNER |
| `KL-82` | P2 | `goal-ui.md` — owner only | WAITING_OWNER |
| `DOC-03` | P2 | AR15 | TODO |
| `RELEASE-03` | P2 | AR16 | WAITING_OWNER |
| `RELEASE-04` | P2 | AR19 | WAITING_DECISION |
| `KL-127` | P2 | `goal-ui.md` — owner only | WAITING_OWNER |
| `MODEL-01` | P2 | `goal-ui.md` — owner only | WAITING_OWNER |
| `MODEL-02` | P2 | `goal-ui.md` — owner only | WAITING_OWNER |
| `MODEL-03` | P2 | `goal-ui.md` — owner only | WAITING_OWNER |
| `IMPORT-05` | P2 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY |
| `IMPORT-06` | P2 | AR06 | TODO |
| `KL-11` | P2 | AR06 | TODO |
| `KL-125` | P2 | AR06 | TODO |
| `KL-128` | P2 | AR06 | TODO |
| `KL-15` | P2 | AR06 | TODO |
| `KL-2` | P2 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY |
| `KL-14` | P2 | AR10 | TODO |
| `KL-37` | P2 | AR10 | TODO |
| `KL-64` | P2 | AR10 | TODO |
| `KL-66` | P2 | AR10 | TODO |
| `KL-12` | P2 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY |
| `KL-135` | P2 | AR12 | WAITING_DECISION |
| `KL-146` | P2 | AR07 | TODO |
| `KL-3` | P2 | AR07 | TODO |
| `KL-6` | P2 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY |
| `KL-68` | P2 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY |
| `KL-69` | P2 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY |
| `KL-71` | P2 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY |
| `KL-85` | P2 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY |
| `KL-86` | P2 | AR05 | TODO |
| `PDB-01` | P2 | AR07 | TODO |
| `PDB-02` | P2 | AR07 | TODO |
| `PDB-05` | P2 | AR07 | TODO |
| `PDB-06` | P2 | AR05 | TODO |
| `PDB-08` | P2 | AR06 | TODO |
| `PDB-10` | P2 | AR06 | TODO |
| `R-DYNAMIC-01` | P2 | AR07 | TODO |
| `R-MODULE-03` | P2 | AR07 | BLOCKED_EXTERNAL |
| `R-MODULE-04` | P2 | AR07 | BLOCKED_EXTERNAL |
| `KL-133` | P2 | `goal-ui.md` — owner only | WAITING_OWNER |
| `KL-38` | P2 | AR11 | TODO |
| `KL-39` | P2 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY |
| `KL-40` | P2 | AR11 | TODO |
| `KL-44` | P2 | AR11 | TODO |
| `KL-47` | P2 | AR11 | TODO |
| `KL-51` | P2 | AR11 | TODO |
| `KL-60` | P2 | AR11 | TODO |
| `R-SEC-01` | P2 | Later / separate scope — not an alpha task | LATER |
| `UI-03` | P2 | `goal-ui.md` — owner only | WAITING_OWNER |
| `UI-04` | P2 | `goal-commission.md` — owner only | WAITING_OWNER |
| `KL-130-ZOOM` | P2 | `goal-ui.md` — owner only | WAITING_OWNER |
| `KL-20` | P2 | `goal-ui.md` — owner only | WAITING_OWNER |
| `KL-124` | P3 | `goal-ui.md` — owner only | WAITING_OWNER |
| `KL-76` | P3 | AR14 | TODO |
| `KL-102` | P3 | AR14 | TODO |
| `KL-110` | P3 | `goal-commission.md` — owner only | WAITING_OWNER |
| `KL-115` | P3 | `goal-commission.md` — owner only | WAITING_OWNER |
| `GAP-T30-04` | P3 | `goal-commission.md` — owner only | WAITING_OWNER |
| `HISTORY-01` | P3 | Later / separate scope — not an alpha task | LATER |
| `HISTORY-02` | P3 | Later / separate scope — not an alpha task | LATER |
| `DOC-02` | P3 | AR00 | DONE |
| `MODEL-04` | P3 | `goal-ui.md` — owner only | WAITING_OWNER |
| `MODEL-05` | P3 | Later / separate scope — not an alpha task | LATER |
| `MODEL-06` | P3 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY |
| `MODEL-07` | P3 | Later / separate scope — not an alpha task | LATER |
| `IMPORT-01` | P3 | Later / separate scope — not an alpha task | LATER |
| `IMPORT-02` | P3 | Later / separate scope — not an alpha task | LATER |
| `IMPORT-03` | P3 | `goal-commission.md` — owner only | WAITING_OWNER |
| `IMPORT-04` | P3 | Later / separate scope — not an alpha task | LATER |
| `KL-100` | P3 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY |
| `KL-48` | P3 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY |
| `KL-134` | P3 | Recorded boundary — AR00 provenance / AR15 claims | WAITING_DECISION |
| `KL-70` | P3 | Recorded boundary — AR00 provenance / AR15 claims | WAITING_DECISION |
| `KL-88` | P3 | Recorded boundary — AR00 provenance / AR15 claims | WAITING_DECISION |
| `PDB-03` | P3 | AR07 | TODO |
| `PDB-04` | P3 | Later / separate scope — not an alpha task | LATER |
| `PDB-07` | P3 | AR05 | TODO |
| `PDB-11` | P3 | AR05 | TODO |
| `FUTURE-01` | P3 | Later / separate scope — not an alpha task | LATER |
| `FUTURE-02` | P3 | Later / separate scope — not an alpha task | LATER |
| `FUTURE-03` | P3 | Later / separate scope — not an alpha task | LATER |
| `FUTURE-04` | P3 | Later / separate scope — not an alpha task | LATER |
| `FUTURE-05` | P3 | Recorded boundary — AR00 provenance / AR15 claims | LATER |
| `FUTURE-06` | P3 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY |
| `FUTURE-07` | P3 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY |
| `KL-107` | P3 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY |
| `KL-16` | P3 | AR15 | TODO |
| `KL-42` | P3 | AR04 | TODO |
| `KL-65` | P3 | AR13 | TODO |
| `KL-41` | P3 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY |
| `KL-45` | P3 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY |
| `KL-46` | P3 | AR15 | TODO |
| `KL-52` | P3 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY |
| `KL-53` | P3 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY |
| `KL-54` | P3 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY |
| `KL-55` | P3 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY |
| `KL-56` | P3 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY |
| `KL-9` | P3 | AR15 | TODO |
| `TOOLS-06` | P3 | AR00 | DONE |
| `TOOLS-01` | P3 | Later / separate scope — not an alpha task | LATER |
| `TOOLS-02` | P3 | Later / separate scope — not an alpha task | LATER |
| `TOOLS-03` | P3 | Later / separate scope — not an alpha task | LATER |
| `TOOLS-04` | P3 | Later / separate scope — not an alpha task | LATER |
| `TOOLS-05` | P3 | Later / separate scope — not an alpha task | LATER |
| `FUTURE-08` | P3 | Later / separate scope — not an alpha task | LATER |
| `KL-121` | P3 | `goal-ui.md` — owner only | WAITING_OWNER |
| `KL-43` | P3 | `goal-ui.md` — owner only | WAITING_OWNER |
| `KL-97` | P3 | `goal-ui.md` — owner only | WAITING_OWNER |
| `KL-98` | P3 | `goal-ui.md` — owner only | WAITING_OWNER |
| `UX-01` | P3 | `goal-ui.md` — owner only | WAITING_OWNER |
| `UX-02` | P3 | `goal-ui.md` — owner only | WAITING_OWNER |
| `UX-03` | P3 | `goal-ui.md` — owner only | WAITING_OWNER |

Entries spanning supported and blocked subcases (notably `KL-13`, `KL-11`, `KL-61` and parameter semantics) require a subcase disposition in their AR package. `TODO` authorizes verification/planning within the stated boundaries, not guessing the missing semantics or claiming implementation is absent.
