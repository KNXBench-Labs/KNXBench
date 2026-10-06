# KNXBench goal — alpha-release readiness, without taking over UI or commissioning

**Created:** 2026-10-01 20:50 CEST.
**Planning baseline:** `origin/main` at `dfa0cc79ca50bc7da747058e97978c7e3a2de25c`, refreshed after the UI closure receipt published during planning.
**Input inventory:** [OFFENE_PUNKTE.md](docs/archive/OFFENE_PUNKTE.md), all 180 main-table IDs; its original audit revision and date remain unchanged.

Use **this file** as the instruction for the autonomous alpha-readiness session. Do not create or use a second `goals.md`. Creating this plan does **not** start implementation, authorize a hardware operation, approve a release tag or publish anything.

The outcome is an honestly scoped, tested Linux-first alpha candidate and an evidence-backed release decision, not the disappearance of every limitation. An accepted boundary is not a bug waiting to be secretly volunteered for overtime.

## Telegram-flow Alpha addition — user decision 2026-10-04

The user confirmed [the session-local nervous-system contract](docs/TELEGRAM_FLOW_VISUALIZATION.md)
and explicitly authorized the responsible Goal sessions to implement it.
**It belongs in the Alpha**; omission needs a new explicit user scope decision.
The new packages are AR20 (read-only data/server evidence contract) and AR21
(integrated acceptance); UI U19–U21 stay exclusively in `goal-ui.md`.
No commissioning package, spatial domain entity, persistent traffic store or new
bus operation is added. The older flow-inspector-first recommendation is narrowly
superseded by [ADR-0077](docs/adr/0077-session-local-telegram-flow-view.md), not by
reopening accepted floor-plan/spatial-editor exclusions.

Schedule **U19 → AR20 → U20 → U21 → AR21** as prerequisites of the finished
AR15–AR18 dossier/artifact/review, not after AR19 merely because the additive IDs
are higher. Existing unrelated packages/owner work continue; an owner dependency
is not permission to bypass its lock. AR19's exact release approval stays separate.
Approval does not auto-start/resume another session or authorize this planning
session to implement. Package checklists track this new user feature; the frozen
180-ID source inventory and existing canonical ledger dispositions stay unchanged.

## 0. Scope and source of truth

This goal owns remaining **non-UI, non-commissioning** correctness, data-integrity, import/product-data, backend reporting, documentation, packaging and whole-product acceptance work derived from the inventory. It turns the list into small work packages with explicit prerequisites, evidence and stop boundaries.

- [`goal-ui.md`](goal-ui.md) remains the sole execution owner of its issues, their existing server/domain halves, U13 review findings, UI acceptance and the Web lock. **No U-package is copied into this queue.**
- [`goal-commission.md`](goal-commission.md) remains the sole execution owner of programming, download/recovery, commissioning research, their server/CLI/UI paths and hardware evidence. **No K-package is copied into this queue.**
- [`goal.md`](docs/archive/goal.md) remains historical/contextual evidence for the other backlog, accepted exclusions and decisions. When this new goal is explicitly started, use its AR packages as the execution queue for the overlapping non-UI/non-commissioning scope; do **not** simultaneously run a second `goal.md` executor on those same tasks. This file does not change either other track's instructions or approvals.
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
| Telegram-flow read-only participant/type/age/session-context contract | This goal, AR20 after U19 handoff | Publish exact semantic contract and named regressions; no Web rendering/binding edits without the UI owner. |
| Telegram-flow visualization and owner acceptance | `goal-ui.md`, U19–U21 under the Web lock | Consume the exact U21 receipt in AR21; no duplicated UI package or new K-package. |
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
Next ready work is AR06 (AR05's scoped delivery is recorded below). The activation
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

**State 2026-10-04:** user decision: phases 3–5 deferred past the Alpha; phases 1–2 plus AR02 are the boundary. Ledger row `KL-129`.

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

**Status:** `DONE` at scoped implementation/audit delivery;
published `04900fbc35b2daec5e766a32f99c263a04700e0e`, exact remote ref and
all 25 owned artifacts verified. Isolated base AR04 receipt `f4b845a3`.
Shared master Languages evidence and v18 -> v19
byte-only re-derivation implemented; DPT provenance and Dynamic truncation
audited with owning-crate regressions. Deferred commit rollback corrected after
behavioral RED; two behavioral mutants caught and source hashes restored.
Scoped corpus matrix/migration comparison passed after independent exact-row
reconciliation and verified re-pinning; no prior evidence or normalized data
lost. Complete corrected-candidate acceptance proc_f9f87cee4329 exited 0:
2,884 workspace / 585 ProductDB / 1,312 Web tests; all 20 expected steps,
three explicitly executed private cases and 595 stable source files. Separate
in-session review findings and compiled guard mutants are closed; source
restoration verified. Implementation publication/readback complete; closing
documentation published as `65b91777` and both owned checkouts/branch/scratch
cleaned after exact verification. This scoped delivery
does not close missing DPT source-winner provenance, remove numbered residuals,
accept a release waiver or claim full manufacturer compatibility. Contract:
[manufacturer report](docs/MANUFACTURER_REPORT_CONTRACT.md).

- [x] Add corpus-observed handling/reporting for master `Languages` attributes, preserving bytes and reporting unknowns instead of silently declaring them known.
- [x] Audit duplicate normalized IDs and within-file DPT provenance; distinguish retained source, chosen normalized value and lost/uninterpreted semantics.
- [x] Make bounded Dynamic diagnostics disclose truncation/budget effects rather than claiming exhaustive subordinate coverage.
- [x] Test upgrades and explicit re-derivation from stored blobs. Preserve historical installation reports; keep metrics unavailable for pre-ledger installs instead of backfilling guessed zeros.

**Exit evidence:** malformed/unknown/duplicate/budget regressions, upgrade tests and explicitly executed private corpus cases with aggregate-only evidence. No regeneration is allowed to falsify the original install report.

### AR06 — Harden supported import boundaries and truthful unsupported-format errors

**Sources:** `KL-1`, `KL-11`, `KL-125`, `KL-128`, `KL-15`, `IMPORT-06`, `PDB-08`, `PDB-10`.
**Dependencies:** AR05 where reports/migrations intersect.

**Status:** `DONE_SCOPED`; verified delivery 95e6bcb0, independent samples `BLOCKED_EXTERNAL`, not full ETS/sample acceptance.
Scoped root/metadata namespace and XML-value hardening plus explicit optional
master diagnostics/admission implemented; final three-crate gates pass (162
passed / zero failed / 75 ignored). Synthetic evidence includes six seeded-DB
refusals, foreign-master typed-write prevention and native source/project reopen
with/without shared products, plus compiled negative controls. No whole-checklist,
private-corpus, independent-sample or new compatibility acceptance.
Contract: `docs/IMPORT_BOUNDARY_CONTRACT.md`. Discovery/checkpoint:
`.ai/logs/2026-10-02_codex_alpha-import-boundaries.md`.

2026-10-02 13:02 CEST follow-up: filename-only legacy refusal and early CLI
destination guards are implemented and verified, with full native/opaque reopen
and seeded main/WAL preservation. Products-only CLI also refuses unsupported
master roots before manufacturer writes. Five compiled killed mutants; all
production sources restored; five-crate gates 899 passed / zero failed /
119 ignored, strict Clippy/format/whitespace green. No private/wider/integrated
acceptance or publication yet; the comprehensive checkboxes remain open until
their scoped delivery/evidence is reconciled. No legacy importer is activated.

2026-10-02 13:16 CEST renewed candidate receipt: `proc_11bf8a0ab11f` exited 0,
all twelve explicit steps/logs reconciled; 2,904 workspace passes, zero failures,
163 ignored and ten actually executed selected private offline cases. Strict
workspace Clippy/build, format/whitespace and nonempty intended-root repository
gates pass; thirteen Rust files remained frozen and Web/binding delta is empty.
This is not whole-corpus/independent-sample or integrated publication acceptance.
Raw-field/sample and DefaultLine/device-local evidence and final delivery remain
open; missing genuine independent samples stay BLOCKED_EXTERNAL.

2026-10-02 14:34 CEST mapping checkpoint: renewed proc_2296901dfdb3 exited 0;
six compiled behavioral mutants killed with byte-exact mapper restoration,
170 three-crate passes / zero failures / 76 ignored, strict Clippy/format green.
Eight selected private offline cases ran without skips; synthetic native reopen
covers 24 combinations with/without an empty product DB. Six existing raw
catalogue/producer tests also passed. The contract now records exact raw-field
boundaries and three existing project exports from two documented installations,
not three independent installations. Complete feature review, renewed wider /
integrated gates and publication are pending; checkboxes remain open until that
delivery boundary. Genuine independent module/schema samples remain external.

2026-10-02 15:17 CEST review checkpoint: the prior eighteen-step mapping gate
exited 0 (2,910 workspace passes / zero failures / 164 ignored / 146 blocks;
eighteen selected private cases plus six existing raw tests, no skips; fifteen
Rust sources frozen). Separate caller review then reproduced premature product
DB creation on unsupported-master refusal. CLI project admission now precedes
DB opening; missing-target/non-openable-sentinel controls, a compiled killed
mutant, eight ordinary and eight private CLI tests and strict lint pass. The
earlier broad receipt belongs to the preceding tree; renewed broad verification,
remaining whole-feature review and integrated publication are still pending.

2026-10-02 15:52 CEST final code/fixture review checkpoint: corrected-CLI
eighteen-step proc_129928b4f8dc exited 0 with the same verified 2,910/zero/164
workspace scope and eighteen actual private/six raw passes. All fourteen changed
Rust diffs reviewed (runner additionally freezes the unchanged mapper).
An evidence gap remained: application refusal fixture had not saved its domain
seed. `NotSaved` RED reproduced that gap; explicit nonempty save/equality checks
and a caught compiled destructive-store mutant now pass. Restored service
four ordinary tests/zero failures/three ignored and strict app lint pass.
Both review findings resolved, no remaining blocking in-session code finding;
final strengthened-fixture and integrated gates/publication remain pending.

2026-10-02 16:14 CEST candidate acceptance: final strengthened-fixture
proc_0c225d9b5f0e exited 0, all eighteen expected steps and raw logs reconciled,
2,910 workspace passes/zero failures/164 ignored/146 blocks, eighteen selected
private and six existing raw tests without skips, fifteen protected sources
unchanged. Fresh origin/main b6a43f3b includes later UI/monitor source changes;
preserving/integrating those and regating the combined tree precedes publication.
This is current candidate acceptance, not a completed delivery or sample waiver.

2026-10-02 16:56 CEST integrated acceptance: proc_8211942fb620 exited 0; all 24
steps/logs independently reconciled on 1404f39b. 595 source/config inputs frozen;
workspace 2,916 passed/zero failed/164 ignored/146 blocks, Web 1,357 unit and
52 intercepted Chromium fixture tests, eighteen selected private and six raw
cases without skips. Bindings/dependency policy/Clippy/build/nonempty root gates
pass. Documentation-only e98a0b58 integration and publication remain pending;
prior pending combined-gate wording is historical. Contract/sample matrix keeps
independent missing samples BLOCKED_EXTERNAL; no full compatibility claim.

2026-10-02 16:59 CEST scoped delivery: source f8b6f27e published through 95e6bcb0;
local/fetched/live main matched, divergence 0/0. Two later UI theme documentation
commits preserved without code changes; doc gates/whitespace pass. Four scoped
verification items below are complete under the contract and explicit sample
matrix, not a full legacy importer/ETS approval or waiver of external samples.
Both review findings closed; task-owned cleanup then AR07 offline research next.

- [x] Verify current schema/namespace detection, mapping and atomic refusal. Separate `.knxproj` evidence from `.knxprod` evidence and independent installations from reexports of the same installation.
- [x] Preserve unreadable attributes and untyped master/version lexemes where technically possible, or report their exact boundary. Do not present raw Secure capacities/MinEtsVersion/ReplacesVersions as tested abilities.
- [x] Improve misleading VD3–VD5/PR3–PR5 rejection text without implementing their deferred importer or enabling VD2. Maintain explicit unsupported diagnostics and native-data integrity.
- [x] Verify DefaultLine diagnostics and device-local communication-object mapping with synthetic regression cases and existing authorized corpus. New genuine project-schema/model evidence remains externally blocked until available.

**Exit evidence:** bounded compatibility/import report, malformed-input and atomicity tests, native save/load evidence and an explicit sample matrix. Missing independent samples remain `BLOCKED_EXTERNAL`, not “compatible”.

**Ledger rows reconciled 2026-10-05:** `KL-128` `DONE`; `KL-11`, `KL-125` `BLOCKED_EXTERNAL`; `IMPORT-06`, `KL-15`, `PDB-08`, `PDB-10` `ACCEPTED_BOUNDARY`; `KL-1` unchanged (`BLOCKED_EXTERNAL`). Evidence and test names in the ledger rows.

### AR06P — Admit real-world product packages the supported grammar already covers

**Sources (post-snapshot, outside the 180-ID ledger):** `KL-149`, `KL-150`, `KL-151`, `KL-152`, `KL-153`.
**Origin:** test-only run of the release `knx products ingest` against 853 public
manufacturer downloads on 2026-10-03 (`2cceea4e`; [corpus run](docs/PRODUCT_DATABASE_CORPUS.md#public-crawler-corpus-run-2026-10-03)).
644 installed. The refusals are real compatibility gaps for downloads a user
actually gets from Siemens, ABB, Hager/Berker and MDT.
**Dependencies:** AR06's import-boundary contract (`docs/IMPORT_BOUNDARY_CONTRACT.md`).
`KL-150` touches the nested-module model of AR07/R-MODULE-04, so coordinate with
the AR07 executor before changing `dynamic/parse.rs`. No dependency on UI or
commissioning.
**Status:** `DONE` 2026-10-05. All five items closed; `KL-153` admitted exact
scheme 10 (ADR-0083). Closing 853-file release measurement (fresh database per
file, baseline `e6099fb7` vs. candidate, originals rehashed unchanged):

| Outcome | 2026-10-03 (shared run) | baseline, standard | candidate, standard | candidate, `--allow-large-package` |
|---|---|---|---|---|
| installed | 644 | 692 | 837 | 852 |
| unsupported namespace | 147 | 145 | 0 | 0 |
| CLI extension case (§149) | 46 | 0 | 0 | 0 |
| size limit (§151) | 13 | 15 | 15 | 0 |
| evidence item limit (§152) | 1 | 0 | 0 | 0 |
| database constraint (§150) | 1 | 0 | 0 | 0 |
| unreadable ZIP (a PDF) | 1 | 1 | 1 | 1 |

Only transition baseline → candidate: 145 `unsupported namespace` →
installed (the 146th scheme-10 file is the large bundle, counted under size
limit). The first column is the original shared-database run
([corpus run](docs/PRODUCT_DATABASE_CORPUS.md#public-crawler-corpus-run-2026-10-03));
the three new columns use a fresh database per file.

**Earlier status (history):** `IN_PROGRESS`. `KL-149` has a scoped candidate with independently
verified public16 (Rust3000/0/165, Web1702, Chromium72,17 equal bindings,
700 exact inputs), five CLI regressions and two compiled routing mutants.
A separate same-release-profile baseline/candidate pair proves two named
baseline REDs and candidate5/0/0. The full same853 original-filename measurement
is independently accepted: installed644→687, Hager/Berker2→45,43 newly
admitted; two ZIP-limit and one evidence-item refusal remain explicit and
atomic. All originals unchanged, no private raw/item logs. Normal fresh U18
merge dd5a350c independently passes NEW actual16: Rust3000/0/165/151 blocks,
Web1702, Chromium82 inventory/pass,17 bindings,704 committed-exact inputs.
All285 CLI Rust/build/test inputs equal the measured producer; private evidence
retains its original run identity. Scoped KL-149 delivery362fec24 is
published/live/fetched read back at equal0/0: source704 and ten exact owned
artifacts, full owner history preserved. Final five acceptance Markdown gates
passed;16 completed own build/snapshot/shadow/XDG directories removed with
all aggregate/public evidence retained. Closing metadata cb5781c7 is also
published/live/fetched byte-verified; final target, seventeen runner/config
scaffolds, owned clean checkout and ancestor-confirmed branch removed.
`KL-150` is delivered:actual public16/private pair,source706 and whole853
687→688 admission verified,code1b215d51 and closing5540dcac live/fetched/blob
readback exact,own cleanup complete. Earlier dated candidate checkpoints below
remain history,not current status. No R-MODULE-04 runtime expansion.
`KL-151` resource research is delivered at80a5500d,but production cap/caller
acceptance remains open. `KL-152` actual maxima and coupled policy are verified:
latest7f57abbb public18 Rust3028/0/176,Web1739,Chromium82/source712 accepted;
fresh full853688→690 independently reconciled after six owner Rust changes.
All other outcomes unchanged;853 originals/archives/atomic refusals verified.
Earlier ed03cb85 receipts and440-input equivalence are historical,not current.
Code published as2b2a267f7873137ccbf3d0a5052a541a76573d59 with exact
remote/artifact readback; four runtime directories removed. Closing metadata
and final hygiene remain separate,not a whole-Alpha completion.
`KL-153` is bounded research; AR06P/AR07/Alpha remain open.

Ordered by value per effort:

- [x] `KL-149` (P2): compare `.knxprod`/`.vd*` extensions case-insensitively in `knx products ingest`, matching `install_package`. Add a CLI regression test with an upper-case name and a negative control showing that `.KNXPROJ` still routes to the project importer. Verified effect from the same853 original-filename release measurement: Hager/Berker installs go from 2 to 45 of 48.
- [x] `KL-150` (P1): synthetic nested-definition RED, lexical identity/argument-position stack, atomic rollback/replay and three compiled mutants verified. Actual integrated cbe9952f public16/private pair accepted; code published as1b215d51 after preserving a foreign stats-only commit, live/fetched equal. Full853 Release CLI: installed687→688, one constraint refusal→installed, other outcomes unchanged; originals independently rehashed, no private raw/item records. Source706 unchanged;14 own build/snapshot/browser directories removed. Closing metadata/final checkout hygiene pending. AR07 receives the retained-source scope witness, not new R-MODULE-04 runtime semantics or a private committed fixture.
- [x] `KL-151` (P2): measure peak RSS, ingest time and database growth for the eight over-limit bundles and five over-limit members, using a temporary raised limit in a scratch build only. Then decide on a documented bound (or streaming) with a hostile-ZIP regression test. Do not just remove the limit. Siemens' only current download (1,006 MiB expanded) is the reference case.
  - Research accepted at base5540dcac (2026-10-03 22:02 UTC): exact853 hashes,15 size-selected pairs including2 later dispatch admissions, scratch fixed member256MiB/expanded4GiB/compressed256MiB. Baseline15 atomic size refusals, raised14 installs/1 namespace refusal;16 hostile controls/six helper controls, source706/Release binaries exact. Max771.46MiB RSS/221.55s/7.038GiB DB, total638.04s/20.462GiB DB. Originals rehashed/private temporary data removed/no raw or item-vector output. Production caps unchanged; direct HTTP install/product mutex requires owner-aware latency/resource acceptance before a global raise. Research subpackage is not KL151 completion or a new UI task.
  - **Closed 2026-10-05 (ADR-0082):** `PackageLimits::{STANDARD, LARGE}`; `knx products ingest --allow-large-package` opts into 256 MiB per member / 4 GiB total, default and HTTP unchanged. Release measurement on the 15 size-refused public packages: standard 15 typed refusals with hint, large 14 installed + verified, 1 scheme-10 namespace refusal; max 760.5 MiB RSS / 256 s / 7.18 GiB DB; Siemens 91 s / 402 MiB; control 20/20 identical; originals rehashed. 853 effect 690 → 704 (derived). [Measurement](docs/PRODUCT_ZIP_LARGE_PROFILE.md).
  - 2026-10-04 AR06V existing-count contract checkpoint: public synthetic Scheme11 archives prove inclusive4096 ZIP entries/install/exact retained archive/replay and4097-entry SizeLimit refusal preserving every seeded database value. Native2/0/0 and two compiled semantic controls (cap4095 and4097) independently verified; canonical cap4096 never mutated. This is count-boundary evidence only, not byte-size/caller/resource-owner acceptance, a limit raise or KL151 completion. Branch public8 all exit0:ProductDB640/0/25, strict package Clippy/fmt/fresh-root doc-policy/whitespace,784 public inputs exact. Native2 is a subset of640;25 ignored tests not passes. Actual integrated4dbca7e5 public15 accepted:Rust3147/0/177 across170 blocks,Web1761,Chromium90 plus separate probe1,793 sources/CLI hash+version exact. Earlier optional-placeholder startup refusal and90 socket-path browser launch failures retained separately. New published Web-owner changes require latest integration/regating before publication; no new private corpus run.
- [x] `KL-152` (P2): measure the actual evidence-item maxima of the two refused packages. Then either size the budget with a hostile-input test, or change evidence collection to a counted summary that stays explicit and loss-reporting.
  - Research at80a5500d accepted:4 size-admitted scheme14 packages/16 XML docs,2 selected; actual Release/Release scratch pair baseline2 atomic item refusals/observer2 installs. Max802433 items/155281510 estimated bytes, observer134552KiB RSS/max2.985s; archives/all853 originals independently exact/private temp0. Item-only raise insufficient because byte64MiB remains. Candidate1048576 items/256MiB retains depth1024/ZIP/namespace/grammar/all-or-nothing evidence,no truncation,master-language64MiB/262144 unchanged. Two named public REDs/six new hostile-boundary/late-preservation tests. Latest7f57abbb actual public18 GREEN:Rust3028/0/176/153 blocks,Web1739,Chromium82,source712/logs/Release binary exact. Fresh full853 Release/Release comparison independently reconciled after six owner Rust changes:688 unchanged table-count installs/163 unchanged normalized refusals/2 item-budget admissions→690 installs. Both binaries/all853 originals independently rehashed,retained archives exact,atomic refusals/private temp0. Earlier80a/ed03 receipts/binding17/eight controls and440-input equivalence retain historical identities. First latest rebuild-verifier rejection remains separate; corrected actual18 retry passes. Latest full CLI peak275128/275440KiB,max wall13.658/13.468s are cohort observations,not HTTP or resource-policy guarantees. Fresh binding17/eight controls and actual integrated doc5 GREEN; docs-only bb62ae57 preserved at20a3c4cd/source712 unchanged. Code and acceptance docs published as2b2a267f7873137ccbf3d0a5052a541a76573d59; normal main push/live/fetched/nine blobs exact. Four own runtime directories removed; closing metadata/final hygiene tracked separately. Not complete Alpha or ETS/runtime compatibility.
- [x] `KL-153` (P2): collect grammar evidence for scheme 23 first (current ETS6 product downloads, 2 files) the same way schemes 12–14/21 were admitted. Scheme 10 (146 ABB ETS4-era files) follows only if its grammar differences are bounded. Any scheme without evidence stays an explicit refusal.
  - First bounded research at575a2d1d:all853 hashes/852 master XML/one BadZipFile census refusal,2 selected scheme23 packages/8 complete XML (2 each Master/Catalog/Hardware/ApplicationProgram),no observed namespace mismatch/foreign elements/qualified attrs,4 positive/3 negative controls. Current ProductDB631/0/25/28 blocks/Clippy/fresh Release GREEN; real original-name CLI2 atomic exact namespace refusals and independently rehashed853/temp0. Official project Schema23 v01.00.00 (2024-03-01) excludes full manufacturer semantics;see docs/PRODUCT_SCHEME_23_RESEARCH.md. Counters and retained-language23 evidence are not typed product admission. No production changes;research doc5/in-session/private-delta review GREEN and eight research docs published as87df5d82a126384da904fc47277b0bd0101bf198 with exact live/fetched/blob readback. Closing bookkeeping/own hygiene remain separate;later synthetic admission/unknown-reporting/atomicity/caller evidence still required,KL153 unchecked.

  - Scheme23 bounded import is now locally verified on integrated `a346fa30` (ADR-0072): actual native6/caller4, four semantic baseline caller REDs and six compiled guard controls; original survivors/rejected wrappers retained. Original Release Full853 Source713690/161/2 is producer-bound,not a newly run current private matrix. Import/storage/evidence source and CLI entrypoint remain byte-identical; upstream query.rs only adds a read-only server helper/test. Actual Source770 public22:Rust3089/0/176 across161 blocks,ProductDB638/0/25,Web1739,Chromium82,17 fresh byte-bound shadow binding pairs,strict Clippy/build/docs/dependency gates. Separate integrated in-session review,no blocking product finding,no independent-model approval. Published as `aadd88204de154cfcf5c1638310831a0a316dd86`: live/fetched refs and17 exact owned blobs read back; actual final integrated10 repeats Rust3089/0/176,Web1739,Chromium82/all10 commands0 after preserving the owner story-only update. Original final wrapper old-vs-new Git-stamped binary hash refusal retained; current release version/hash independently archived. Scheme10 remains refused and KL153/AR06P/Alpha open.

2026-10-04 AR06U research checkpoint: bounded scheme10 public/source investigation
started after delivered exact23/hygiene c17b0f36. KNX format-family references are
not namespace grammar proof; candidate mirror cover/body is unverified,
full schema10 specification/XSD not recovered. Both package
master admission and dedicated master-language evidence omit10. See
[scheme10 research](docs/PRODUCT_SCHEME_10_RESEARCH.md). No production change,
new private census/import result, namespace admission or compatibility claim.
KL153/AR06P stay IN_PROGRESS; bounded structural probe/ownership evidence pending.

**2026-10-05 KL-153 closed (ADR-0083).** Vocabulary census over 146 scheme-10
packages: 0 element kinds, 0 element/attribute pairs and 0 parent/child pairs
outside scheme 11; no foreign namespace or qualified attribute. Exact scheme 10
admitted through the strict 21/23 member validation; `tests/scheme10.rs`
(install, master translations and language evidence, replay, strict refusals,
late rollback, look-alike namespaces) with three compiled mutants. Release:
146/146 scheme-10 files install (1 with the large profile) and verify clean.

**Exit evidence:** RED/GREEN regression tests per item, unchanged atomic refusal
for anything still unsupported, a re-run of the 853-file measurement with a
before/after table, and updated KNOWN_LIMITATIONS entries. The crawled files are
private, unpinned evidence and never a committed fixture or CI gate.

At 2026-10-03 22:35 CEST KL150 candidate a2aa4b7 has full offline853
original-filename CLI evidence, independently reconciled: baseline687 installed,
candidate688; exactly one database constraint refusal→installed, all other
categories unchanged, no installed→refused regression. Original manifest/all
inputs rehashed unchanged, successful retained package bytes/source blob hashes
verified, refusal tables empty, private temp copies/DBs removed; aggregate-only
receipts. Fresh complete public16 (Rust3009/0/166, Web1702, Chromium82,
bindings17) and same-profile Release12 (baseline named RED/candidate storage7
GREEN/CLI5 controls each) pass. Fresh upstream is still cb5781c7, source706
exact. Integrated acceptance, final metadata/publication/readback and own cleanup
remain; KL150 is still unchecked and R-MODULE-04 runtime remains separate.

### AR07 — Research and validate supported parameter semantics, never execute unknown vendor logic

**Sources:** `KL-3`, `KL-146`, `PDB-01`, `PDB-02`, `PDB-03`, `PDB-05`, `R-DYNAMIC-01`, `R-MODULE-03`, `R-MODULE-04`.
**Dependencies:** AR05/AR06. **Mode:** bounded offline research first.

**Status:** `DONE_SCOPED` 2026-10-05 (closing note after the checklist). History: `IN_PROGRESS`; fresh `alpha-parameter-semantics` from published
AR06 receipt `0c3d6a8a`, 2026-10-02 17:14 CEST. Primary Condition_t constraint and actual
controller-kind resolver traced in [parameter boundary](docs/PARAMETER_SEMANTICS_BOUNDARY.md).
Baseline independently reconciled at 417/0/6 after correcting a source-string
ignore-count error. Stored-controller RED/GREEN and ADR-0061 now bound comparison
to Number/Restriction, preserve None policy and explicitly refuse other known
kinds. Controller gate proc_b1e47be31700 is 16/16 green: public ProductDB/server
layers 1,118/0/57, six compiled behavioral mutants, exact source restoration,
Clippy/build/format. Corrected broad proc_ed20715c68e5 independently accepted
13/13: workspace 2,924/0/164, Web 1,357, six selected private Dynamic tests
6/0/0, no genuine skips, all 103 original archive hashes unchanged. No private
raw logs, 596 source/config inputs frozen and 17 shadow bindings equal.
Published U15/theme ancestry fe02deeb integrated at 03f18c95. Integrated
proc_dea67da354fd independently reconciled at 17/17: workspace 2,924/0/164,
Web 1,559, Chromium fixtures 61, selected private 6/0/0; 606 inputs frozen
and 17 shadow bindings equal. Controller checkpoint published/read back at
2d9aaeb8; local/live/fetched refs equal and all owned document bytes confirmed.

Additional bounded Float declaration guard candidate: actual RED101 before fix,
Float6/0/0 and HTTP34/0/0 (ten lower/upper metadata cases, nonempty project
equality, retained source, independent sibling writes). Corrected public
proc_e87d7afdd30d8/8, workspace2926/0/164, all615 source/config hashes frozen,
17 shadow bindings equal; failed prerequisite attempt archived/rejected. Both
compiled min/max guard mutants caught by unit + HTTP (4 observations), all615
hashes restored. Final owned proc_536a6eb1634218/18 independently reconciled:
workspace2926/0/164, Web1559, existing intercepted Chromium61, private6/0/0
with zero genuine/unknown skips and all103 original identities/hashes unchanged.
No private raw logs, 615 frozen inputs and 17 shadow bindings equal. Later
upstream commissioning c9f77d7b changes code outside this guard; actual merged
bc5999c1/proc_d4c3a0b0b43f independently passes20/20: workspace2931/0/164,
Web1559/Chromium61, private6/0/0 + offline SimTunnel HTTP13/0/0 without skips,
all108 originals unchanged (including103 product archives),615 frozen inputs
and17 equal bindings. Published/read backda3bc947, local/live/fetched refs
equal0/0 and eight exact owned artifacts; completed owned builds/shadows
cleaned, shared root/U16 untouched. Broader AR07 stays
open. Fresh alpha-parameter-budget from published be88e7b9 now reproduces the
general-diagnostic gap: compiled public RED101/0-1-0, 2,000,896 warnings above
the proposed 2,000,000 ceiling in1.89s, without refs/labels or exhausted expansion
quota. Production source unchanged; no timeout/OOM/private/live/vendor run.
ADR-0062 records proposed shared work admission and incomplete-evaluation write
refusal. Sibling REDs subsequently compiled and reproduced the defect; public
Core GREEN proc_ce7c8cb5fef3 passed nine stages (Library354/0/0,
DynamicTree62/0/6, eight small admission-boundary units, strict Clippy/fmt/
whitespace/source freeze). Four targeted repeats are subsets. A second compiled
HTTP RED proved prefix write authority incorrectly returned200 instead of400.
The server now clears all write targets after resource truncation; public
backend GREEN proc_61273ee26c21 passed six stages: HTTP35/0/0 and server204/0/2,
strict ProductDB/server Clippy, atomic nonempty-project refusal, retained source
bytes and source freeze. Separate in-session bounded source/security review has
no blocking finding, not an independent-model approval. First mutation attempt
proc_f134efced013 failed shared-lock acquisition before any compiler/source
mutation; its timeout remains rejected, not behavioral evidence. Retry
proc_43b041004d63 independently accepted five compiled behavioral mutants:
walk/skipped-descendant/binding/diagnostic admission and prefix write authority.
Each compiled0 and produced intended Rust101/0-1-0 assertion failure; canonical
evaluator/server bytes and full source hashes restored. Actual candidate
proc_cd67fb854490 exited0, independently20/20: workspace2944/0/164,
Web1559/Chromium61, selected private Dynamic6/0/0 + offline SimTunnel HTTP13/0/0,
103/108 originals unchanged,616 frozen inputs/17 equal bindings and strict
build/lints/nonempty intended-root audits. Restored candidate GREEN is not
latest-upstream acceptance: integrate fresh0889c102 U16/U17 with all owner
artifacts intact, gate the combined tree, then publish/read back.
Actual integration subsequently committed7ae116a1, both complete owner histories
and all producer source artifacts retained. proc_096e63a3429e exited0 and
independently20/20: workspace2953/0/164 over148 blocks, Web1665/Chromium61,
explicit private Dynamic6/0/0 + offline SimTunnel HTTP13/0/0 without genuine
skips or private raw logs,103/108 originals unchanged,624 frozen inputs equal
exact committed blobs,17 bindings equal and strict build/lints/nonempty root
audits. Five compiled mutants remain restored. This accepts ADR-0062's bounded
work policy, not broader AR07. Acceptance metadata/publication/readback pending;
preceding integration-pending words describe the earlier candidate checkpoint.
Bounded policy source8f47c13b/actual-gated7ae116a1 subsequently published/read back
through2704f8e29b6ca2d53c022c468f116b526109c678: local/live/fetched refs equal0/0,
all14 owned artifacts and624 actual-gated inputs exact remotely; doc-only
acceptance gates pass, code delta zero. Own build/script/raw-log scaffolding
removed, minimal aggregate receipts and source-only remaining audit retained.
Final receipt metadata publication/readback and clean checkout/branch removal
remain closing steps, not new semantics or full-package AR07/Alpha acceptance.
Broader AR07 and
work/byte/semantics acceptance remain open; no completed UI/native/ETS claim.
`unsupportedControlKind` warning/English fallback is on the backend wire; manual
Web kind/catalogue adoption stays with UI. Selected-private/broad controller
gates and separate in-session review pass, and the controller checkpoint is
accepted. Broader budget/module-identity/provenance/vendor-inert audit remains
open, distinct from the scoped controller and Float fixes. Channel
label data/UI half is already delivered; unknown manufacturer logic stays inert.

2026-10-03 module-provenance subpackage: fresh `alpha-module-provenance` from
published `e9707794`. Public compiled nameless-nesting RED101/0-1-0 shows distinct
Core/server paths collapsing to identical HTTP scope identities. ADR-0063 adds
response-local `nodeChain` through the existing accessor only; legacy fields,
write authority, source data and storage schemas unchanged. Public HTTP38/0/0,
server205/0/2 and selected Core identity72/0/6 pass, strict Clippy/fmt/whitespace
and624 frozen inputs; three compiled omission/inner-only/reversal mutants caught
and restored, separate in-session review without blocking finding. Integration
and delivery pending. UI manual scope matching remains owner work, not completed
by an additive wire field; genuine nested manufacturer and broader AR07 remain
open. The prior budget final receipt/cleanup is verified, not a pending rerun.

2026-10-03 05:38 actual module-provenance acceptance: reviewed source d51dd6c7,
conventional integration3711c4f7 on published e9707794. proc_f27315f3cd8c's20/20
independently accepted: ordinary Rust2957/0/164 over148 blocks, Web1665,
intercepted Chromium61, selected private Dynamic6/0/0 and in-memory SimTunnel
HTTP13/0/0; no genuine/unknown skips or private raw logs. All624 committed inputs,
17 bindings and420 original files (including103 product archives) unchanged.
Hashing420 files is not parsing420 files. Strict Clippy/build/fmt/dependency and
four nonempty intended-root audits pass. First `Checking`-only receipt assertion
rejected; actual fresh Clippy says `Compiling`, corrected without source change
or gate replay. ADR-0063 bounded backend accepted; publication/readback/cleanup
pending, UI scope adoption and whole AR07/Alpha remain open.

2026-10-03 05:48 module-provenance backend leaf delivered: source/acceptance
3a8b66f422bda73c9999fb214f2459c79ecea76b published and read back with0/0
local/live/fetched refs at that checkpoint, all11 own artifacts and624 actual
inputs exact remotely. Actual3711c4f7's20/20 and three compiled mutants/restoration
retain their scope; post-gate Markdown delta has four nonempty intended-root
audits/whitespace green. Completed owned targets/raw scaffolding removed after
process checks; minimal aggregates retained. Final receipt-only publication and
checkout/two ancestor-confirmed branch cleanup follow separately. Continue
earliest-ready AR07 external substitution/variable-output audit on fresh upstream;
UI chain adoption, genuine nested products and whole AR07/Alpha remain open.

2026-10-03 06:56 AR07 continuation: final module-provenance receipt14e2eb9a
published/read back; actual clean checkout/two ancestor-confirmed branches and
owned scaffolding removed, dirty root unchanged. Fresh alpha-text-output on14e2eb9a
audits binding/label scalar copies. Initial ancestor-cost hypothesis rejected:
ModuleScope::argument is deliberately local, already charged correctly; no
inheritance/ancestor fee is authorized. ADR-0065 is proposed scalar admission,
not new KNX semantics or old byte guarantee. proc_49808cde0743 compile0, three
copy assertions101/0-1-0 independently verified; fourth inheritance expectation
rejected and converted to positive non-inheritance regression (old-production1/0/0).
Exact UTF-8-cost unit101/0-1-0 verified before core fix. Candidate core copy admission
implemented, Core proc_c73947a5b170 independently5/5:355/0/0 library and66/0/6
DynamicTree plus strict Clippy/fmt/whitespace/freeze, no private ignored cases
executed. New HTTP cause test shares existing whole-prefix atomic assertions;
proc_8ee608d8a643 independently5/5, named1/0/0/full39/0/0, strict lint/fmt/
whitespace/freeze and complete-prefix atomic authority/source invariants.
Omission/restoration proc_98b4f3f512f1 independently verifies3 compiled mutants,
4 caught assertions, evaluator/all9 scoped hashes exact. Candidate-only broad
public proc_2b136248f7f9 rejected at header158>157 after workspace2963/0/164.
Required blank Rust doc separator fixed, unchanged ceiling; fix gate410/157/17
passes, first rejected logs/receipt retained. Retry proc_230c3f7e4db9 independently
passed13/13, workspace2963/0/164 across148 blocks, Web1665,615 frozen inputs and
17 unchanged shadow bindings; copy policy renumbered0065 to preserve upstream0064;
no integrated
delivery yet. No consumer/UI/binding source edit
or acceptance claimed. Outside-walk String-only projection and general allocation/
RSS/latency remain explicit boundaries; whole AR07 and Alpha still open.

## AR07 scalar-copy actual integration accepted — 2026-10-03 09:52 CEST

Reviewed source2e7a41c3 conventionally merged with published c07e6403 at
9f512ab3e302014d1b4c3d2af33cd33712a052ad; upstream ADR0064/owner histories
preserved, own scalar policy ADR0065. Actual proc_3ea5d4e7afa4 exit1 is retained:
private Dynamic Rust6/0/0 exit0 but closed skip classifier rejected two output
signals. Private raw lines were not persisted; do not claim recovered text.
Source-backed public regression proves bool-valued PackageInstallReport.skipped
and registered libtest-prefix cases; corrected classifier5 positives/7 negatives
pass, unknown or missing-data signals still rejected. No production change.
Continuation proc_8dd254bb5c75 exit0 independently accepted20 stages: five public
commands reused from the exact same committed617-input tree (not a wholly fresh
20-command run),13 commands and2 final checks newly executed. Workspace2984/0/165
over149 blocks, Web1665, intercepted Chromium61; newly selected private Dynamic
6/0/0 and in-memory offline SimTunnel Download14/0/0, zero unknown skip signals.
All617 current/committed source/config hashes and17 shadow bindings equal;
420 originals including103 product archives unchanged across the complete new
private window. Hashing420 files does not mean parsing420 files. Strict
Clippy/build/fmt/dependency and four nonempty intended-root audits pass.
Earlier candidate/header failure and three compiled omission mutants/restoration
remain scoped to their actual runs, not relabelled. Separate in-session review,
not an independent-model verdict. Acceptance-doc gates/publication/readback
pending. Broader AR07/Alpha, external String-only ISSUE-08 projections,
UI diagnostics/identity adoption, native/ETS and general allocation/RSS/latency
remain open. No live bus/vendor code or private raw logs; UI owner untouched.

## AR07 bounded scalar-copy policy delivered — 2026-10-03 09:59 CEST

Source/actual acceptance published and fetched/live read back at
cbc6b0b238fbeef2da41c4d38208850574c68760: owned HEAD/fetched/live main equal,
divergence0/0; all617 actual-gated code/config inputs and7 acceptance documents
exact remotely. Canonical dirty root/main/statistics unchanged. Actual9f512ab3
continuation remains20 accepted stages with5 verified public commands reused,
13 commands+2 checks new, workspace2984/0/165, Web1665/Chromium61, selected
private Dynamic6/0/0 and offline SimTunnel14/0/0;17 equal bindings and420
unchanged originals including103 archives. Four fresh-target nonempty
intended-root acceptance-document audits and whitespace passed; zero code delta.
ADR0065 accepted only for this bounded Core/HTTP scalar-content admission,
not external String-only projections or full AR07/ETS/Alpha semantics. Initial
header/classifier rejections and compiled mutant/restoration receipts retained.
Six completed owned build targets actually removed after process checks;
compact machine-readable evidence retained. Closing receipt-only metadata
gates/publication and clean owned checkout/branch/scaffolding removal follow.
Whole AR07/Alpha, ISSUE-08 checked-result consumer contract, UI diagnostics/
identity adoption, native/hardware/vendor/general allocation/RSS remain open.

### AR07 checked outside-walk subpackage accepted — 2026-10-03 13:32 CEST

Scoped source a065ad94 / actual0369a56a, proc_39eb1bd6a2f3 exits0/22 commands:
16 fresh public,3 compiled selected inventories,3 private executions (6+2+1).
Workspace2995/0/165/150 blocks, Web1702/Chromium72,17 equal bindings,
699 exact committed inputs,420 unchanged originals, no private raw logs/link.
Five real compiled behavior mutants and separate in-session review accepted.
ADR0066 is bounded backend text-overlay policy, not complete response memory,
legacy SDK/UI-specific/native/ETS semantics. Acceptance documents, publication,
readback and own cleanup pending. e2a40268's Markdown-only AR06P plan/findings
integrated7bb0ee72 with complete owner histories and zero gated-code delta.
Fresh queue now prioritizes AR06P KL-149 then coordinated KL-150; actual nested
manufacturer evidence is a newly available prerequisite, not already validated.
Existing broader AR07 checkboxes/matrix and whole Alpha remain open.

- [x] Compare current handling with primary, available schema/specification evidence and existing packages; separate supported behavior, unexpanded Repeat, raw allocator/calculation data and unknown activation from actual correctness defects.
- [x] Fix only semantics demonstrably established by those sources, with an ADR before any new domain/storage contract. Preserve opaque forms and warn on unsupported constructs; no guessed RepeatIndex, placement formula, DPT or visibility.
- [x] Verify activation-budget/refusal behavior, nested/duplicate module identities and parameter validation; retain safe read-only behavior when evidence is insufficient.
- [x] Improve data/projection provenance for channel names and indeterminate activation without inventing missing manufacturer texts. Hand presentation changes to UI.

**Exit evidence:** research decisions and supported-semantics regressions or exact external blockers. ParameterCalculation scripts, Button handlers, DLLs and unknown vendor code are not executed; a complete Repeat/Allocator engine is not authorized merely because it appears in the inventory.

**AR07 closed `DONE_SCOPED` 2026-10-05.** Census of 3,599 distinct programs
and the ADR-0080 write-authority package
([boundary](docs/PARAMETER_SEMANTICS_BOUNDARY.md#ar07-corpus-census-and-write-authority--2026-10-05),
[ADR-0080](docs/adr/0080-parameter-write-authority.md)) complete the four
items together with the earlier ADR-0061/0062/0063/0065/0066, Float-guard and
`KL-150` receipts. Rows: `KL-3`, `KL-146`, `PDB-02`, `PDB-03`, `PDB-05`
`ACCEPTED_BOUNDARY`; `PDB-01`, `R-DYNAMIC-01` `BLOCKED_EXTERNAL` (missing
normative Repeat/AP-XSD evidence); `R-MODULE-03/04` stay `BLOCKED_EXTERNAL`
with corrected missing input (rule, not sample). New `KL-156` (unreported
`Parameter`/`ParameterRef` attributes) is the next alpha package. UI owner:
adopt the three new diagnostic tokens.

**AR07 follow-up `KL-156` delivered 2026-10-05**
([ADR-0081](docs/adr/0081-parameter-attributes-are-reported.md)):
`Parameter`/`ParameterRef` report every attribute they do not store; schema
v21 backfills `ingest_unknown`, measured install reports and
`package.unknown_count`. Fixture tests plus six mutants, workspace
3,289/0/177, a 102-package corpus probe (migrated equals fresh in every
unknown/report table) and a matrix re-pin whose five changed aggregates an
independent recount predicts exactly. `KL-156` `DONE`.

### AR08 — Complete the safe data/application half of supported password import

**Source:** `KL-13`. **Dependencies:** AR06 and existing secret-handling conventions.

- [x] Trace the existing ZipCrypto reader into application and CLI entry points; add only missing approved plumbing, not a second cipher implementation.
- [x] Test correct/wrong/missing password, corrupted data, decompression/size bounds, nested archives, failed-import atomicity and native roundtrip. Passwords must not enter persistent project data, process arguments, logs or compatibility reports.
- [x] Hand any new password dialog to UI rather than editing it here. Record whether the entry path is library-, CLI-, application- or end-to-end verified.
- [x] Leave AES project decryption sample-gated and encrypted manufacturer packages unsupported. Do not enable AES or make a real-ETS claim from synthetic success alone.

**Exit evidence:** supported entry-path tests and redaction checks, with genuine ETS/AES evidence gaps still visible. Secret transport must follow the repository's verified mechanism; if none exists for an intended surface, document/design it first.

**State 2026-10-04:** library, application, CLI and server entry paths verified; Web dialog handed to the UI owner; evidence in [ALPHA_READINESS](docs/ALPHA_READINESS.md#ar08-password-import-entry-paths-2026-10-04). Status: ledger row `KL-13`.

### AR09 — Verify DPT/model fidelity without changing wire rulings by guesswork

**Source:** `KL-61`. **Dependencies:** AR06/AR07 for imported type provenance.

- [x] Audit implemented types and observable encoding rulings against current primary evidence. Fix proven format defects only; keep format validation distinct from unverified subtype tables.
- [x] Investigate explicitly declared group-address DPT versus linked-object inference in supported project schemas. If modeling/resolution changes are justified, write an ADR and migration/roundtrip contract first.
- [x] Test explicit, inferred, conflicting, missing and unsupported types, retained raw payload and error propagation. Never silently override a conflict or discard a source declaration.

**Exit evidence:** model/import/native roundtrip and codec tests, or a documented prerequisite. No LTE/200-series scope expansion, scene-display change in UI or real group write.

**Status 2026-10-05: DONE.** Width audit `f3b4fd34` (17.001 fixed), range pass `f7050d6a`, ADR 0078 + implementation `5a2249d3`, integrated `b7d7927e` (Rust 3269/0/177, Web 2013, Chromium 132). Audits in `docs/spec-audits/`. Web display of declared-vs-linked detail handed to the UI owner.

### AR10 — Finish evidence-backed backend localization paths

**Sources:** `KL-14`, `KL-37`, `KL-64`, `KL-66`.
**Dependencies:** AR06/AR07 and established language-pack contracts.
**Status:** `DONE_SCOPED` 2026-10-05 — backend complete; the UI portion (`KL-37`: showing the slice-2b markers) delivered by the UI owner the same day (`ba190b6e`), `KL-37` `ACCEPTED_BOUNDARY`. History: `TODO` → slices 1, 2a, 2b, 3 → UI markers.

- [x] Trace source/default language, manufacturer/master translations and backend diagnostics into their current readers and projection.
- [x] Use a source-backed language when available; otherwise expose the fallback rather than inferring one from unrelated UI settings or installation names.
- [x] Add language-aware lookups/structured diagnostics only on established paths, preserving original text, missing-translation behavior and raw diagnostic details.
- [x] Test both installed languages, absent/ambiguous translations, existing native migrations and fallback reproducibility. Send required new catalogue/rendering work to UI under its ownership contract.

**Exit evidence:** data/application/CLI localization regressions and clear boundary wording, not a “fully localized” claim. A missing frontend consumer keeps that portion waiting on its owner.

**AR10 slice 1, 2026-10-05:** trace in
[research/backend-localization-paths.md](docs/research/backend-localization-paths.md)
(first checkbox's evidence). `KL-14` `ACCEPTED_BOUNDARY`: no project
language exists in the schema; `DeviceInstance/@InitialValueLanguage` is
reported, never promoted; the placeholder is never consulted
(`knx-etsproj/tests/project_language.rs`). Next slice: expose the overlay
fallback (which stored language answered, or the package's declared
`DefaultLanguage`) on the parameter, catalogue, device-product and master
surfaces — `KL-37`/`KL-64`.

**AR10 slice 2a, 2026-10-05:** parameter panel done — `textLanguage`,
`nameLanguage`, `enumOptions[].language` and `sourceLanguage` expose which
stored language answered or that the package's own text was kept
(`parameter_views_name_the_language_that_answered_and_expose_the_fallback`,
`parameter_panel_exposes_the_answering_language_and_the_fallback`). Catalogue,
device-product and master surfaces remain (slice 2b). UI owner: decide how
the markers are shown. **Delivered by the UI owner 2026-10-05 (`f5494094`):** "Untranslated
(en-US)" / "Options untranslated" badges in the parameter panel.

**AR10 slice 2b, 2026-10-05:** catalogue, device-product block and master
readers expose the answering language and the fallback
(`device_product_names_the_language_that_answered_and_the_fallback`,
`the_dpt_text_names_the_language_that_answered`, catalogue assertions).
`KL-64` ACCEPTED_BOUNDARY. `KL-37` stays open for the report's
communication-object text; `KL-66` next.

**AR10 slice 3 and closure, 2026-10-05:** the report's communication-object
text follows the device detail's translation rule
(`knx_app::com_object_language`, `documentation_composition.rs`
`com_object_text::*`); the server now calls the same function. Ledger:
`KL-14`, `KL-64`, `KL-66` `ACCEPTED_BOUNDARY`; `KL-37` `WAITING_OWNER` (UI
markers for catalogue, product block and DPT text). Not a "fully localized"
claim: report labels stay §48, server error bodies stay §66.

**UI portion delivered, 2026-10-05 (UI owner, recorded 2026-10-06 at the
user's request):** `f5494094` shows the slice-2a markers in the parameter panel;
`ba190b6e` adds one shared rule and badge (`apps/knx-web/src/languageFallback.tsx`)
for the catalogue (name, description), the device product block (product text,
catalogue name, application name, each with its declared source language) and
the communication-object DPT text (no source declared on that wire). Markers
appear only with a product language selected. Ledger `KL-37`
`ACCEPTED_BOUNDARY`; [KNOWN_LIMITATIONS §37](docs/KNOWN_LIMITATIONS.md#37-translations-reach-selected-surfaces-not-every-imported-text-or-ui-output)
"the markers are shown". Evidence: RED-first Vitest, 10/10 and 11/11 mutants,
gates in `.ai/logs/2026-10-05_claude_ui-ar10-language-markers.md` and
`…_ui-kl37-language-markers.md`. Owner self-report; no independent review of the
UI half was requested.

### AR11 — Resolve remaining CSV/report decisions and verify the selected backend scope

**Sources:** `KL-38`, `KL-40`, `KL-44`, `KL-47`, `KL-51`, `KL-60`.
**Dependencies:** AR06; AR07 for supported parameter output.

- [x] Reproduce current behavior and provide a bounded decision for the unaccepted §40 and §60 residues. Recommended default: derived counts stay read-only; add source-backed Description/Comment roundtrip only under an approved schema/CSV contract; retain explicit paging unless the UI owner adopts a measured alternative.
- [x] Obtain the reserved scope decision before calling either residue accepted or implementing a behavior change. Continue other packages while awaiting it.
- [x] Implement and test only the approved backend slice: malformed/duplicate CSV, address conflicts, atomic preview/apply, undo/redo, native roundtrip and no hidden lossy columns.
- [x] Keep raw module/AllocatorRef output explicitly warned where semantics are unknown. Document HTML/diff/custom-CSV scope without claiming ETS, ESF/OPC or layout parity.

**Exit evidence:** recorded decisions, approved slice tests and a UI handoff for any new display. DIN-26 CSV ranges, native PDF, prose catalog and diff application/correlation exclusions stay accepted, not reopened.

**State 2026-10-04:** user decisions recorded: §40 accepted as is for the Alpha (no schema change), §60 handed to the UI owner (virtualisation plus search/filter before the Alpha). §38/§44/§47/§51 verified and accepted; no backend slice was approved, so none was built. Evidence: [ALPHA_READINESS](docs/ALPHA_READINESS.md#ar11-csvreport-decisions-2026-10-04); status in the ledger rows.

### AR12 — Resolve package-version policy before implementing selection

**Source:** `KL-135`. **Dependencies:** AR05/AR06. **Initial status:** `WAITING_DECISION`.

- [ ] Enumerate actual stored package/application versions and conflicts, current winner policy and effects on existing project references.
- [ ] Propose the smallest explicit version/pinning choice; record a user-approved policy or continued deferral. Do not automatically replace project applications or reinterpret raw ReplacesVersions.
- [ ] If approved, implement only its versioned data/application contract, with compatibility/conflict/rollback/migration tests and no manufacturer hard-coding.
- [ ] Hand user selection UI to its owner. A backend selector without its agreed consumer remains partial, not a delivered whole feature.

**Exit evidence:** explicit policy and tests/owner receipt where activated. No speculative online catalog or firmware/device update.

**State 2026-10-04:** user decision: continued deferral. First-installed winner with disclosed candidates is the Alpha boundary; nothing implemented. Ledger row `KL-135`.

### AR13 — Verify privacy and the actual deployment-security boundary

**Sources:** `KL-106`, `KL-22`, `KL-65`.
**Dependencies:** AR01; can run independently of UI/commissioning acceptance.

- [x] Audit debug/report export against bounded privacy-pattern classes and synthetic sensitive fixtures, not one remembered address. Ensure newly handled fields are redacted or explicitly warned without claiming perfect anonymization.
- [x] Verify existing server authentication/bind refusal and documented reverse-proxy/TLS requirements offline. Keep a shared password distinct from roles, multi-user isolation and public-Internet safety.
- [x] Make artifact version/provenance accurately describe its built revision; test any adopted dirty-build marker rather than claiming version output proves a clean source tree.

**AR13 delivered, Claude session, 2026-10-04:** dossier and deployment/privacy
checklist in [ALPHA_READINESS](docs/ALPHA_READINESS.md#ar13-privacy-and-deployment-security-dossier).
Debug report now names every kept class and the telegram file's values and
timestamps; all 97 declared route/method pairs checked for the guard;
`KNX_REQUIRE_CLEAN_TREE=1` release builds refuse a modified tree. 12 new
tests (1 privacy fixture, 1 exhaustive route test, 10 build-stamp tests),
9 behavioural mutants caught plus a build-level experiment now pinned by a
test. KL-65 `DONE`; KL-22 and KL-106 `ACCEPTED_BOUNDARY`. AR17 must build
with `KNX_REQUIRE_CLEAN_TREE=1`; the dialog wording is handed to the
Web-lock holder.

**Exit evidence:** privacy/authentication/provenance regressions and deployment/privacy checklist. No automatic TLS service, user-role system, new license regime, credential disclosure or host configuration change.

### AR14 — Verify existing non-commissioning bus/CLI contracts offline

**Sources:** `KL-29`, `KL-31`, `KL-62`, `KL-72`, `KL-73`, `KL-74`, `KL-75`, `KL-76`, `KL-77`, `KL-78`, `KL-126`, `KL-102`.
**Dependencies:** AR09 for DPT changes; otherwise independent.

- [x] Verify supported CLI address/name formatting and existing routing/scan/monitor contracts with fakes and local adapters. If a case is already assigned to U13/ISSUE-12 or another owner, consume its test instead of rebuilding it here.
- [x] Test truthful busy/vacant/unknown results, timeout boundaries, exclusion enforcement and reconciliation preserving project identity; occupancy is not product identity.
- [x] Maintain bounded one-tunnel/one-line scope, uncertain retries and custom multicast evidence gaps. Real gateway/custom-group evidence is external and belongs to an authorized owner, not this goal.
- [x] For an unreachable public error branch, document that reachability boundary and test only genuinely reachable behavior; do not fabricate a wire event to tick a coverage box.

**AR14 delivered, Claude session, 2026-10-04:** offline dossier in
[ALPHA_READINESS](docs/ALPHA_READINESS.md#ar14-offline-buscli-contract-dossier).
Three defects fixed (CLI project style, multi-installation names in CLI and
server monitor, Linux cross-group routing delivery), 17 new tests plus a
three-level case in an existing round trip, 16/16 behavioural mutants caught. KL-29 `DONE`, KL-31 `BLOCKED_EXTERNAL` (real
custom-group run), the other ten `ACCEPTED_BOUNDARY` as retained documented
boundaries. No bus run.

**Exit evidence:** offline contract/regression dossier or retained documented boundaries. No real bus run, multi-tunnel redesign, automatic coupler traversal, scan speed promise or new hardware support. Monitor UI and discovery acceptance remain external owner work.

### AR14B — Reach the bus from a bridge-network container

**Sources (post-snapshot, outside the 180-ID ledger):** `KL-155`.
**Dependencies:** none; independent of UI and commissioning acceptance.
**Origin:** user report and decision 2026-10-05 ("ja mit in die alpha goal"): the Docker server could not reach the bus while the CLI on the host could.

- [x] Measure the cause read-only (`DESCRIPTION_REQUEST` only, no tunnel, no bus frame) and correct every document that promised tunnelling through the bridge. **Done 2026-10-05:** see KNOWN_LIMITATIONS §155; installation guide, Linux setup, web/Docker guide, README, Dockerfile comment, §79 and GAP E5 corrected.
- [x] Add an opt-in UDP Route Back mode for tunnelling (`[D]` Core v01.06.02 AS §8.6.2.2, §8.4.3.4.3): control and data endpoint of `CONNECT_REQUEST`, and the `CONNECTIONSTATE_REQUEST`/`DISCONNECT_REQUEST` HPAIs, all zero. Default stays the own address, so hosts and gateways that work today see no change. RED loopback tests assert every HPAI the client sends in both modes; mutants for each site.
- [x] Wire it to the server (`KNX_TUNNEL_ROUTE_BACK=1`) and document it for bridge containers. Discovery stays host-network only (multicast).
- [ ] Optional, only with the user's go: one read-only tunnel connect/disconnect from a bridge container to a real gateway (no bus frame, gateway free of other tunnels).

**Exit evidence:** offline HPAI regressions and mutants, server env wiring test, updated docs. No claim that every gateway supports Route Back; one measured gateway is not compatibility evidence.

**AR14B delivered, iaw session, 2026-10-05:** `1fd1664a`. `knx_net::TunnelReturnPath`
(`LocalAddress` default, `RouteBack`); the HPAI is fixed per connection and used
by CONNECT (both endpoints), CONNECTIONSTATE and DISCONNECT. `knx-server` reads
`KNX_TUNNEL_ROUTE_BACK`. Tests: `client::tests::route_back_tunnel_sends_all_zero_hpais_for_its_whole_lifetime`,
`default_tunnel_keeps_its_own_address_in_every_hpai`,
`bus::tests::a_route_back_connector_asks_the_gateway_to_answer_the_packet_source`,
`tests::route_back_is_opt_in_and_reads_like_every_other_flag`; RED first, 7/7
compiled mutants killed with sources restored. Gate: web build, fmt, Clippy
`-D warnings`, workspace 3,273/0/177 in 182 blocks (loopback-only namespace),
five xtask checks, whitespace. `KL-155` `DONE` at this scope; the optional live
check waits for the user's go.

### AR14D — Consolidate status tracking before AR15

**Sources:** none in the 180-ID ledger; documentation hygiene requested by the user on 2026-10-04 (“what is redundant and worth merging, and what should be split for readability”).
**Dependencies:** none for D1. D2–D5 need the *status-docs lock* below and run before AR15, so that AR15 reconciles one ledger instead of six.
**Status:** `DONE` — D1–D5 delivered (D5's commissioning part 2026-10-06 by its owner). Status-docs lock released 2026-10-04.

**Why.** The same per-ID status is kept in up to six places: the ledger in §7–§8 of this file, the per-ID ledger in [ALPHA_READINESS](docs/ALPHA_READINESS.md), [COMMISSIONING_ALPHA_LEDGER](docs/COMMISSIONING_ALPHA_LEDGER.md) (42 rows), [UI_ALPHA_READINESS](docs/UI_ALPHA_READINESS.md) (24 rows), [LIMITATION_TRIAGE](docs/LIMITATION_TRIAGE.md) (priorities) and the frozen [OFFENE_PUNKTE](docs/archive/OFFENE_PUNKTE.md) snapshot. They already drift apart: on 2026-10-04 `KL-149` was `TODO` in §8 while the AR06P text called it a scoped candidate. Three very large files also make the current state hard to find: `IMPLEMENTATION_STATUS` (15.2k lines), `KNOWN_LIMITATIONS` (7.6k) and `RESEARCH` (6.8k).

**Status-docs lock.** D2–D5 rewrite rows that other sessions also update. A session takes the lock with a handover-only commit pushed to `main` at once, carrying the line `Status-docs lock: taken by <session> for AR14D-<n>`, and releases it with the same line saying `released` in its merge entry. While another session holds it, do not edit ledger rows. Write the status change in your own handover entry under the heading *Ledger updates for AR14D* instead; the holder applies it and says so. Prefer a quiet window: no other session in the middle of a ledger-touching delivery.

*Frozen while the lock is held* (only the holder edits them): §7–§8 of this file, [ALPHA_READINESS](docs/ALPHA_READINESS.md), [COMMISSIONING_ALPHA_LEDGER](docs/COMMISSIONING_ALPHA_LEDGER.md), [UI_ALPHA_READINESS](docs/UI_ALPHA_READINESS.md), [LIMITATION_TRIAGE](docs/LIMITATION_TRIAGE.md), [KNOWN_LIMITATIONS](docs/KNOWN_LIMITATIONS.md) and [RESEARCH](docs/RESEARCH.md). *Not frozen:* source code and tests, the Web lock and its packages, the top of `IMPLEMENTATION_STATUS`, the handover, `.ai/logs/`, ADRs, and each session's own goal-file text outside status/ledger tables. A delivery during the freeze may merge as usual; whatever it would have written into a frozen file (a status change, a new or changed `KL` entry, a new research finding) goes complete and verbatim into its handover entry under *Ledger updates for AR14D*, and the holder carries it over before releasing the lock.

**User decision 2026-10-04:** the lock is held for the whole D2–D5 rebuild, not one step at a time; other sessions keep the freeze until the holder writes `Status-docs lock: released`. Pausing code work is not required (decided as the less disruptive option).

- [x] **D1 — Archive and slim the logs** (Claude session, 2026-10-04). `goal.md`, `OFFENE_PUNKTE.md` and `PROJECT_ANALYSIS_2026-09-15.md` moved verbatim to [docs/archive](docs/archive/README.md). The September tail of `IMPLEMENTATION_STATUS` (12.5k lines) moved verbatim to [docs/history](docs/history/IMPLEMENTATION_STATUS_2026-09.md). The handover `.ai/CURRENT_STATE.md` keeps its newest entries (77 at the cut); the 470 older ones moved verbatim to `.ai/archive/`. Only relative links changed; check-anchors 393 links / 262 files, none dead. `IDEA.md` and the empty `docs/Issues.md` stay: the latter is the user's inbox for observations.
- [x] **D2 — One source-ID ledger.** ADR first: which file is the status of record, its columns (ID, priority, route/owner, status, evidence link, last change) and who may change which rows (owners keep authority over their rows; only the location changes). Then create it, e.g. `docs/status/ALPHA_LEDGER.md` as one Markdown table, and replace §7–§8 here, the per-ID tables in ALPHA_READINESS, COMMISSIONING_ALPHA_LEDGER and UI_ALPHA_READINESS by links. Their evidence prose stays where it is. Before moving, reconcile every row whose status differs between the sources, with evidence, and list each correction; never pick the more flattering one silently. **Delivered 2026-10-04:** [ADR-0076](docs/adr/0076-one-ledger-is-the-status-of-record.md) and [docs/status/LEDGER.md](docs/status/LEDGER.md), 180 + 5 rows. §7–§8 here, the two tables and the stale count line in ALPHA_READINESS, the priority/disposition columns of COMMISSIONING_ALPHA_LEDGER and the owner checkpoint tables were moved there; a script proved every moved cell is present. 24 rows differed between the goal and ALPHA_READINESS, six statuses were corrected on evidence (`DATA-02`, `KL-42`, `KL-149`, `KL-150`, `KL-152` → `DONE`, `KL-151` → `IN_PROGRESS`) and three stale routes fixed; every case is listed in the ledger's reconciliation record.
- [x] **D3 — A ledger check in `xtask`.** Unique IDs, only the §2.2 status words, 180 snapshot rows plus the post-snapshot rows, every `KL-n` exists as a heading in `KNOWN_LIMITATIONS`, no second per-ID status table anywhere in `docs/` or the goal files. Test with positive and negative fixtures and a mutation of each rule. Add it to the doc gates in §5. **Delivered 2026-10-04:** `cargo run -p xtask -- check-ledger` (`xtask/src/ledger.rs`), also in CI and the gate tables. Ten unit tests written RED first against a stub (9 failed, the exemption case passed trivially), then GREEN; 7/7 behavioural mutants caught; a real-repo negative control (one changed status plus a stray table row) failed with both problems named. Also checks the count lines, which are now part of the file.
- [x] **D4 — Split `RESEARCH.md` by topic** into `docs/research/<topic>.md` (for example project import, product database, KNXnet/IP, DPT, device procedures). `RESEARCH.md` stays as the index; section numbers (§25 and so on) stay stable. Rewrite every inbound anchor link across `docs/`, the goal files, code comments and ADRs, keep `<a id>` aliases where a slug changes, and prove by script that the union of the new files contains every original line. **Delivered 2026-10-04:** five topic files under [docs/research](docs/RESEARCH.md#where-the-research-lives) (project format 297 lines, product data 933, KNXnet/IP 2,610, commissioning 1,893, features and UI 923); `RESEARCH.md` keeps the tag legend, a section table, §1 and §9–§12 and the sources (181 lines). 14 inbound anchor links rewritten; a script proved that every original line is in exactly one file (only link targets changed).
- [x] **D5 — Separate resolved from open in `KNOWN_LIMITATIONS`.** Move the body of resolved or withdrawn entries to `docs/history/KNOWN_LIMITATIONS_resolved.md`. The heading stays in place as a one-line stub with the resolution and a link, so numbering and the 130+ inbound links keep working. Recount with the LIMITATION_TRIAGE command and record the counts before and after. Then replace the *Where things stand* sections of `goal-ui.md` and `goal-commission.md`, and their `goal.md` ownership rows, with links to the ledger; this needs each owner session's agreement in the handover, because those files are theirs. **Delivered 2026-10-04 except the commissioning goal file:** the five entries whose own status line says resolved (§18, §23, §24, §42, §130 gate) keep heading and status line as a stub; their historical bodies (163 lines) moved verbatim to [docs/history/KNOWN_LIMITATIONS_resolved.md](docs/history/KNOWN_LIMITATIONS_resolved.md). §90 and §95 stay: they are pointers, not resolved defects. Heading count unchanged (115). `goal-ui.md`: with the owner's agreement its status narrative moved verbatim to [UI_ALPHA_READINESS](docs/UI_ALPHA_READINESS.md#owner-status-history) and the `goal.md` ownership row points to this file; all five IDs it named have ledger rows. **Commissioning part delivered 2026-10-06 by its owner:** the `goal-commission.md` status narrative (as of 2026-10-01) moved verbatim to [COMMISSIONING_ALPHA_LEDGER](docs/COMMISSIONING_ALPHA_LEDGER.md#owner-status-history); the goal file now links the ledger. `goal.md` itself is archived ([docs/archive/goal.md](docs/archive/goal.md)), so it has no ownership row left to change.

**Exit evidence:** one ledger with a green `xtask` check, no duplicate per-ID status table, inbound links intact (check-anchors plus a plain-file link check), a line-preservation proof for every move, and counts recorded before and after. No status changes without evidence; this package moves and reconciles, it does not close items.

### AR15 — Reconcile release documentation and limitations on the finished scope

**Sources:** `DOC-03`, `KL-9`, `KL-16`, `KL-46`, plus all earlier results and accepted/later routes.
**Dependencies:** completed/explicitly blocked AR00–AR14, AR06P, AR14B and AR14D (one consolidated ledger); newest owner receipts.
**Status:** `DONE` 2026-10-06 — dossier complete ([ALPHA_SCOPE_MATRIX](docs/ALPHA_SCOPE_MATRIX.md)); final acceptance stays with AR18/AR19. Slice 1 done: the four source IDs are
disposed with fresh evidence — `KL-9`, `KL-16`, `KL-46` `ACCEPTED_BOUNDARY`
(Git external-diff recipe run and documented; GTK3/advisories rechecked; report
names rechecked against tests), `DOC-03` `DONE` (stale commissioning,
`.knxproj` hand-over and suggestion statements corrected across README and
manual; counts verified). A storage finding from the same check became
`KL-157` (atomic upgrade, fixed `b247268a`).
Slice 2 (2026-10-06): items 1–2 done as listed under each item. Slice 3
(2026-10-06): items 3–4 in [ALPHA_SCOPE_MATRIX](docs/ALPHA_SCOPE_MATRIX.md)
(capabilities by evidence level, deployment/import/hardware boundaries, ledger
totals, a release disposition for every not-yet-accepted row, risks).

- [x] Reconcile `IMPLEMENTATION_STATUS`, `KNOWN_LIMITATIONS`, `LIMITATION_TRIAGE`, `ROADMAP`, `GAP_ANALYSIS_ETS`, `COMPATIBILITY`, `IMPORT_EXPORT` and the relevant model/architecture docs with actual source/tests.
  - Slice 2 checked claims against code constants and tests: product schemes
    (10–14, 20, exact 21/23) in COMPATIBILITY, GAP_ANALYSIS_ETS, ROADMAP, KL §11,
    PRODUCT_DATABASE_CORPUS and the manual; store/product schema versions (10/21)
    in the architecture tour, implementation status and DATA_MODEL;
    commissioning status in ARCHITECTURE §8 and COMPATIBILITY (now pointing to
    KL §7); migration atomicity (KL-157). ROADMAP/GAP_ANALYSIS commissioning
    text was already current (owner). Not every sentence of these documents was
    re-proven; historical sections stay as dated history.
- [x] Recount limitations and derived tables programmatically; preserve duplicate-ID disambiguation, historical anchors and new entries published by either other track.
  - Slice 2: script over KNOWN_LIMITATIONS and LIMITATION_TRIAGE — 119
    heading lines, 118 numbers (§130 twice), 108 limitations + 11 signposts,
    107 rated (K1 5, K2 30, K3 58, K4 14), §105 unrated; no triage row without
    a heading. New rows §151/153/154/155/157, §149/150/152/156 to signposts,
    §29 to K4, six reasons refreshed.
- [x] Document native SQLite versus text-diff limits, product-dependent report names, GTK3 platform dependency and actual deployment/import/hardware boundaries. Withdraw resolved prose, not remaining evidence gaps.
- [x] Produce the alpha scope/risk/decision matrix: supported and verified, simulator-only, externally blocked, accepted boundary and later work. Every unaccepted blocker gets a release disposition, not a hidden waiver.

**Exit evidence:** mutually consistent release claims, full source-ID disposition and doc gates. This dossier can advance while a final owner receipt is pending; final acceptance cannot.

### AR16 — Accept the existing manual after UI closure

**Source:** `RELEASE-03`. **Dependencies:** AR15 and integrated U13 closure receipt.
**Status:** `DONE` 2026-10-06 — UI closure receipt `84bc32c3` verified, manual accepted at its tested scope ([MANUAL_ACCEPTANCE](docs/MANUAL_ACCEPTANCE.md)). Location/screenshot policy decided by the
user on 2026-10-06: the manual stays on GitHub (`docs/manual/`, read in the
repository, not bundled — consistent with ADR-0024) **with screenshots**.
Screenshots must show the finished application at its tested scope and use
fictional data only.
Slice 1 (2026-10-06): all 21 screenshots regenerated from the real
application with a fictional project (`tools/manual_sample_project.py`,
`apps/knx-web/e2e/manual-screenshots.shots.ts`); every screenshot passage
re-read and corrected; stale "Discover gateways" wording fixed in 7 places.
Dated checklist: [MANUAL_ACCEPTANCE](docs/MANUAL_ACCEPTANCE.md). Open: the
UI owner's closure receipt (two residues reopened 2026-10-06: `UI-04`, `KL-61`
wording), then acceptance. Slice 2 (2026-10-06): claim-by-claim pass done by
script and live probes (CLI 95 invocations, labels, routes, versions, "not yet"
sentences); twelve groups of stale claims corrected, the style hint handed to
the UI owner.

- [x] Verify the current UI owner's closure, actual tested surfaces and any native/accessibility exceptions; do not substitute a self-review or an old unsuccessful invocation.
- [x] Resolve the reserved manual location/screenshot decision consistently with ADR-0024: maintainer `docs/` does not silently become a bundled in-app help system.
- [x] Review the existing manual claim by claim against the finished application, update workflows/screenshots at their actual tested scope, and remove stale “never written”, browser-export and count statements without broadening hardware claims.
- [x] Record explicit manual acceptance and unresolved user-owned exceptions.

**Exit evidence:** dated manual checklist and approved location/screenshot policy. Missing UI/native/live evidence remains marked, not filled with invented screenshots or observations.

### AR17 — Build and inspect the local Linux alpha candidate

**Sources:** packaging part of `RELEASE-04`; no new inventory task implied.
**Dependencies:** AR15/AR16 and the integrated code candidate.
**Status:** `DONE` 2026-10-06 — started before AR16 with the user's permission
(2026-10-06). Candidate `KNXBench_0.1.0-alpha.4_amd64.AppImage` from
`6b9b6818`, SHA-256 `4c778104…80ef2`, `check-appimage` ok; offline smoke under
X11 (private Xvfb) and native Wayland; packaging clean. Findings: version
drift (fixed in `6b9b6818`), `KL-158` (AppImage needs an X server), build
paths in the binary (privacy checklist item 6). Record:
[ALPHA_CANDIDATE](docs/ALPHA_CANDIDATE.md). A later code change needs a
rebuild before AR18.

- [x] Follow the repository's existing AppImage workflow/ADR-0021, using the recorded toolchain and current build commands rather than guessed installation steps.
- [x] Build an actual Linux x86_64 artifact, run `xtask check-appimage` with its real artifact directory and record the built revision, version, digest and result.
- [x] Perform authorized **offline** smoke checks: launch, create/open/save/reopen, clear failure messages and no backend/device auto-contact. Native UI checks themselves remain coordinated with the UI owner; consume its verified result rather than claiming them from headless Chromium.
- [x] Check packaging for private corpus, secrets and accidental maintainer-doc bundling. Record exactly the Linux environment tested.

**Exit evidence:** actual local artifact, validator/smoke output and narrow packaging manifest. No tag, upload or claim of all-distribution/platform support; this is a candidate, not a release.

### AR18 — Final integrated gates and independent whole-product review

**Sources:** `RELEASE-01`, `RELEASE-02`.
**Dependencies:** AR15–AR17, integrated U13 receipt, commissioning's dated scope/safety receipt, and decisions for every unresolved alpha-owned blocker.
**Decisions recorded 2026-10-06 (user):** the independent whole-product review
is done by a fresh Claude session without this history, started by the user
from a written brief; the nine externally blocked or pending rows (`KL-1`,
`KL-11`, `KL-125`, `KL-31`, `PDB-01`, `R-DYNAMIC-01`, `R-MODULE-03`,
`R-MODULE-04`, `KL-158`) are accepted as disclosed Alpha boundaries; the
`KL-61` declared-versus-linked DPT display is built before AR18 (then the
candidate and gates rerun).

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

### AR20 — Publish the read-only, session-bound telegram-flow contract

**Backend delivered 2026-10-04 (Claude); vertical feature open until U20/U21/AR21.** Source: new
approved Alpha feature, [flow research](docs/TELEGRAM_FLOW_VISUALIZATION.md),
[ADR-0077](docs/adr/0077-session-local-telegram-flow-view.md).
**Dependencies:** U19's exact contract handoff and current monitor/project
contracts. Ready independently of unrelated import/commissioning packages;
required before U20 and final AR15–AR18 acceptance. No Web implementation here.

- [x] Trace current capture/rows/context/projection and all consumers against
  directly read KNX clauses; reuse application/service resolution and existing
  associations. Distinguish observed source from configured participant, flag
  handling and unresolved/ambiguous mappings; Send is not a receiver exclusion.
- [x] Implement the smallest pure read-only participant snapshot and service
  adapter: typed raw destination/source identity, per-object membership/evidence,
  existing decoded/raw states and reliable event age. Keep domain/core independent
  of UI and format; no new core graph, coordinates, project/product migration.
- [x] Bind evidence to server incarnation, monitor session and full flow context.
  Extend exact comparison to relevant device/link/flag/activation facts;
  preserve atomic snapshots, cursor/drops/restarts and old-consumer compatibility.
  Do not remap retained events against an edited project or round unsafe counters.
- [x] Specify bounded response/model/age admission and visible unavailable,
  historical, stale and overflow states. Capture repeats with distinct sequences
  are observed frames; duplicate poll delivery is not another event. No new
  tunnel, active probe, persistence, decryption, write API or UI-only inference.
- [x] Add RED/GREEN semantic/service regressions for Write/Read/Response,
  no/multiple recipients, Send-associated receiving candidates, unknown flags,
  inactive/dangling objects, duplicate addresses/IDs, multiple installations,
  DPT conflict/error, individual/opaque services, link-only edits, delayed rows,
  session/server restart, invalid counters, bounded refusal and capture gaps.
- [x] Run actual owning-crate/integration/full gates and restored guard mutations;
  document exact final wire fields/routes, revision and named tests in the U20
  handover. Coordinate bindings with the UI owner/Web lock; no alpha edits under
  the Web tree by stealth. A data-only receipt does not close the vertical feature.

**Exit evidence:** integrated, version-compatible, read-only semantic contract
and named regressions, consumed next by U20. No hardware or recipient-state proof.

**State 2026-10-04:** contract, wire names and named tests in [TELEGRAM_FLOW_VISUALIZATION §10](docs/TELEGRAM_FLOW_VISUALIZATION.md#10-ar20-delivered-contract-alpha-2026-10-04); handed to U20 in the handover. Status: ledger row `FLOW-01`.

### AR21 — Accept the integrated telegram-flow Alpha feature

**Status:** `DONE` 2026-10-05 — accepted for the Alpha on a recorded envelope
(motion for small/medium maps, Motion Off for several hundred nodes or the §7
load; Chromium only), all seven findings closed
([§22](docs/TELEGRAM_FLOW_VISUALIZATION.md#22-ar21-rerun-of-findings-6-and-7-and-acceptance-alpha-2026-10-05)).
Receipt for AR18: §13–§22 and `.ai/logs/2026-10-05_claude_ar21-acceptance.md`.

**Review 2026-10-05: returned to the UI owner with findings.** Receipt
`9d432d17` + `deb6813a` + `fb40a99a` verified and gates rerun; two §9.3 binding
requirements are neither met nor recorded as deviations, and the §7 starting
load saturates the main thread with motion on
([§13](docs/TELEGRAM_FLOW_VISUALIZATION.md#13-ar21-acceptance-review-alpha-2026-10-05)).
`FLOW-01` stays `IN_PROGRESS`.

**Rerun 2026-10-05: findings 1–3 closed, returned again with finding 4**
(the reduced-rendering note counts recipient paths as telegrams), own §7-load
measurement recorded
([§15](docs/TELEGRAM_FLOW_VISUALIZATION.md#15-ar21-acceptance-rerun-alpha-2026-10-05)).

**Rerun of finding 4, 2026-10-05:** fixed for telegrams with a line; returned
with finding 5 (telegrams without a line at the node limit counted as drawn)
([§17](docs/TELEGRAM_FLOW_VISUALIZATION.md#17-ar21-rerun-of-finding-4-alpha-2026-10-05)).

**Rerun of finding 5 and recheck, 2026-10-05:** finding 5 fixed; returned with
finding 6 (partially refused or sender-refused telegrams missing from the note,
[§19](docs/TELEGRAM_FLOW_VISUALIZATION.md#19-ar21-rerun-of-finding-5-alpha-2026-10-05))
and, on the user's recheck request, finding 7 (event-ring overflow counted but
never shown,
[§20](docs/TELEGRAM_FLOW_VISUALIZATION.md#20-ar21-recheck-on-request-alpha-2026-10-05)).
Gates rerun green on the recheck revision.

Dependencies: integrated AR20 and U21's exact owner receipt, with U19/U20
covered. Required before the finished AR15–AR18 acceptance and AR19 decision.
The alpha session does not reimplement the UI or take its lock for bookkeeping.

- [x] Consume and verify both owners' exact integrated revisions/contracts/tests;
  confirm productive use of the existing monitor feed, not an isolated study or
  second connection. Check snapshot/sequence/age/flag semantics across layers.
- [x] Exercise the contract's synthetic end-to-end cases: immediate same-slot
  values/7-second expiry, three badges plus Inspector, current-source leader,
  layout-only freeze, bidirectional/group-labelled pulses, persistent quiet
  edges, high-load coalescing versus actual loss, stale/restart and no-project
  fallback. Use intercepted/fake adapters only; never open a real KNX socket.
- [x] Reconcile actual renderer/load/long-session measurements, limits and
  leak/motion/theme/keyboard evidence with the supported Alpha envelope.
  Existing accepted native/Orca/live boundaries remain disclosed, not silently
  reopened or certified from Chromium. Do not report unexecuted benchmark targets.
- [x] Update manual, implementation status, architecture/research, limitations,
  roadmap and AR15–AR18 scope/artifact/review inputs. Required flow behavior is
  not waived by old optional/spatial dispositions; record any new user exception
  explicitly. Keep the canonical source-ID ledger's historical rows intact.
- [x] Run actual integrated affected/full gates and a separate feature review;
  hand the named flow acceptance receipt to AR18's independent whole-product
  review. Any feature/fix after AR18 reruns affected final acceptance.

**Exit evidence:** source-bound whole-feature receipt and accurate Alpha
scope/artifact/review, not a release tag or KNX/ETS compatibility claim.

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

Run `git diff --check`, the fresh-worktree-built `xtask check-anchors` and `xtask check-ledger` (the 180-ID/owner/table validation, ADR-0076). No full Rust/Web product suite or hardware run is claimed for merely writing a plan.

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
| Status ledger | `cargo run -p xtask -- check-ledger`; 185 rows today (ADR-0076) |
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

**Commissioning scope decision, user, 2026-10-04:** new real-hardware,
power-loss, vendor and ETS validation is accepted out of the commissioning
goal, not a request waiting for the operator and not a commissioning completion
blocker. Preserve the missing evidence as
[KNXBench user notices](docs/manual/known-issues.md#commissioning-validation-boundary).
This is not proof of compatibility or recovery and does not bypass runtime
backup/authorization/refusal gates. Caller coverage, Web/client adoption,
offline recovery contracts and owner admission remain required under
[the commissioning goal](goal-commission.md). SAFE-03/AUDIT-01 are not marked
done by this scope decision; unrelated Alpha acceptance conditions are unchanged.

Distinguish these terminal states:

1. **Offline work exhausted, readiness blocked:** all ready alpha-owned packages are delivered; named owner/external/user decisions remain. Report exact blockers and continue only when they resolve. This is **not** alpha readiness.
2. **Alpha-ready, not published:** AR21 has adopted the integrated Telegram-flow receipt and AR18 is green for the actual candidate, both tracks' relevant receipts/accepted boundaries and manual/release exceptions are settled, and the independent whole-product review has no blocking findings. AR19 may still wait for publication approval.
3. **Published alpha:** the user explicitly approved the exact release action and its target/artifact have been read back and verified. Do not claim this from a local AppImage or a version string.

Do not use a completion percentage for unequal tasks. Report completed packages, current verification, owner/external blockers, accepted limitations and the next ready package or exact unblock action. Once this goal is explicitly started, a normal green package boundary is not a stop boundary.

## 7. Complete source-ID routing ledger

Status, owner and route of every source ID live in the
[source-ID ledger](docs/status/LEDGER.md) ([ADR-0076](docs/adr/0076-one-ledger-is-the-status-of-record.md),
AR14D D2). The routing table, the owner checkpoints and their counts that
stood here moved there on 2026-10-04. Update the ledger row, not this file.

**UI owner handoff — 2026-10-04 11:27 CEST (Claude, `goal-ui.md` owner session):** all
backend halves of the open UI rows are published (`48d1cd2e`, `74dbd1a9`,
`68f18755`, `8b075952`, `acbda83b`). The remaining work is web-only, and the
Web lock is held by the commissioning session (`codex-commission-continuation`).
**By user decision the web halves are handed over to that session**, which
owns them from now on. Each row stays `IN_PROGRESS` until its web half is
published with RED/GREEN, gates and browser evidence; the receiving session
updates the rows. The `goal-ui.md` owner keeps the backend contracts and
answers questions about them.

| Row | Web task for the Web-lock holder | Backend contract (published) | Acceptance |
| --- | --- | --- | --- |
| `MODEL-03` | Let the individual-address editor submit device octet `0`; show the server's refusal text when the product is no evidenced coupler | `POST /api/individual-address` (`deviceId`, `address`) — the server uses `SetCouplerIndividualAddress` only when the installed product has `Hardware/@IsCoupler="true"` (RESEARCH §25) | `.0` accepted for a coupler, refused with message otherwise; undo restores |
| `DATA-03` | Generate one `requestId` (1–128 chars `[A-Za-z0-9_-]`) per catalog submit; after a lost/ambiguous response offer **retry with the same id**; treat `replayed: true` as success without a second batch | `POST /api/devices` `requestId`; identical resend → `replayed: true`; same id with other content → 400 (ADR-0069) | Retry after simulated network loss creates the batch once |
| `MODEL-04` | Two opt-in toggles in the catalog dialog: *allocate addresses* (only with a selected line) and *unique names* | `POST /api/devices` `allocateAddresses` (needs `lineId`), `uniqueNames`; both default `false` and are part of the replay fingerprint | Allocated addresses skip `.0`, used and excluded addresses; short supply refused as a whole |
| `MODEL-01` | Installation rename control; installation choice for new areas, main ranges, root building parts, range-less group addresses and the CSV import/export buttons; later-installation targets in dropdowns | `PATCH /api/installations/{id}` (`name`); optional `installationId` on `POST /api/areas`, `/api/group-ranges`, `/api/building-parts`, `/api/group-addresses` and on `/api/group-addresses/csv-import` / `csv-export`; cross-installation moves → 400 "separate infrastructures" (ADR-0070) | Edit and create in installation 2; one undo per action |
| `MODEL-02` | Where the Inspector shows a placement/line-owner ambiguity, offer "keep this placement" per current slot | `POST /api/repair/device-placement` (`deviceId` + exactly one of `keepLineId` / `keepUnassignedInstallationId`), `POST /api/repair/line-owner` (`lineId`, `keepAreaId`); save refuses ambiguous topology (`AmbiguousTopology`, ADR-0071) | Repair enables ordinary editing; undo restores the exact imported state; save works after repair |
| `UX-01` | Drag a group address onto a communication object; the existing keyboard/select path stays | `POST /api/group-links` (`comObjectId`, `gaId`, `direction`) — unchanged contract | Drop links once; invalid drop shows the server refusal |

**User decision 2026-10-04 15:41:** the six web halves above and the AR13
`debugReport.privacyTelegrams` text move from the commissioning session back to
the `goal-ui.md` owner session. That session starts on them only after the
commissioning session releases the Web lock, then takes the lock through the
`goal-ui.md` §3 procedure. Until then the commissioning session keeps the lock
for its own package and owes nothing on these rows. As of this decision, no web
half had been started anywhere.

Entries spanning supported and blocked subcases (notably `KL-13`, `KL-11`, `KL-61` and parameter semantics) require a subcase disposition in their AR package. `TODO` authorizes verification/planning within the stated boundaries, not guessing the missing semantics or claiming implementation is absent.

## 8. Post-snapshot findings (outside the 180-ID count)

These entries were found after the inventory snapshot. They have their own
identities and do not change the 180-entry count above. Priority follows the
`OFFENE_PUNKTE.md` scale.

Their rows are in the [ledger](docs/status/LEDGER.md#post-snapshot-ids).

## Checked outside-walk scoped delivery receipt — 2026-10-03 13:54 CEST

Scoped delivery 0b8ec935362d06642a81bafcbbb74824236c39b8 was pushed/fetched/live-read back at the recorded
checkpoint: refs/trees equal0/0,699 actual-gated inputs and7 acceptance documents
exact. Actual0369a56a's22 accepted commands and final five nonempty/root-explicit
doc/whitespace gates retain their run scope; later delta is Markdown only.
Twelve completed own build/snapshot/shadow/XDG directories removed after process
checks; originals/foreign/root unchanged. Closing metadata gates/readback and
clean checkout/branch/scaffolding removal remain, not new policy acceptance.
AR06P KL-149 is the next ready package; broader AR07/Alpha/UI-native/ETS remain
open. Published statistics-owner artifact preserved, not a local statistics refresh.
