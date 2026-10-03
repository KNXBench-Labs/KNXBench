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
**Status:** `IN_PROGRESS`. `KL-149` has a scoped candidate with independently
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
`KL-150` storage candidate is verified at 2026-10-03 20:56 CEST: exact synthetic
UNIQUE-constraint RED, nine public focused tests/ProductDB625/0/25, three compiled
scope mutants/five named failures and an exact committed pre-fix baseline RED.
One authorized offline private nested package is baseline RED/candidate GREEN;
independent lexical/stored scope counts, original/retained member bytes and
retry agree, originals unchanged, no raw/private item logs. All706 candidate
source/config inputs exact. Separate in-session review is not independent-model
approval. Full853 CLI measurement, fresh broad/current-upstream acceptance,
publication/readback and own cleanup remain; KL150 stays unchecked. No new
R-MODULE-04 runtime semantics, migration/schema, limits, UI or bus behavior.
`KL-151`/`KL-152` need measurement before
any limit changes. `KL-153` is bounded research.

Ordered by value per effort:

- [x] `KL-149` (P2): compare `.knxprod`/`.vd*` extensions case-insensitively in `knx products ingest`, matching `install_package`. Add a CLI regression test with an upper-case name and a negative control showing that `.KNXPROJ` still routes to the project importer. Verified effect from the same853 original-filename release measurement: Hager/Berker installs go from 2 to 45 of 48.
- [ ] `KL-150` (P1): reproduce with a **synthetic** nested-`ModuleDef` fixture (RED), fix the `dynamic_node` key scoping so inner and outer definitions keep their own `(program_id, module_def_id, node_id)`, and keep the atomic rollback. Verify against the local MDT `RF-TAL55Bx0x-01S` package as private evidence only. Do not commit it. Then hand the nested sample to AR07 as R-MODULE-04 input; this package does not claim nested-module semantics.
- [ ] `KL-151` (P2): measure peak RSS, ingest time and database growth for the eight over-limit bundles and five over-limit members, using a temporary raised limit in a scratch build only. Then decide on a documented bound (or streaming) with a hostile-ZIP regression test. Do not just remove the limit. Siemens' only current download (1,006 MiB expanded) is the reference case.
- [ ] `KL-152` (P2): measure the actual evidence-item maxima of the two refused packages. Then either size the budget with a hostile-input test, or change evidence collection to a counted summary that stays explicit and loss-reporting.
- [ ] `KL-153` (P2): collect grammar evidence for scheme 23 first (current ETS6 product downloads, 2 files) the same way schemes 12–14/21 were admitted. Scheme 10 (146 ABB ETS4-era files) follows only if its grammar differences are bounded. Any scheme without evidence stays an explicit refusal.

**Exit evidence:** RED/GREEN regression tests per item, unchanged atomic refusal
for anything still unsupported, a re-run of the 853-file measurement with a
before/after table, and updated KNOWN_LIMITATIONS entries. The crawled files are
private, unpinned evidence and never a committed fixture or CI gate.

### AR07 — Research and validate supported parameter semantics, never execute unknown vendor logic

**Sources:** `KL-3`, `KL-146`, `PDB-01`, `PDB-02`, `PDB-03`, `PDB-05`, `R-DYNAMIC-01`, `R-MODULE-03`, `R-MODULE-04`.
**Dependencies:** AR05/AR06. **Mode:** bounded offline research first.

**Status:** `IN_PROGRESS`; fresh `alpha-parameter-semantics` from published
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
**Dependencies:** completed/explicitly blocked AR00–AR14 and AR06P; newest owner receipts.

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
| `PDB-09` | P1 | AR05 | DONE |
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
| `KL-87` | P2 | AR05 | DONE |
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
| `KL-86` | P2 | AR05 | DONE |
| `PDB-01` | P2 | AR07 | TODO |
| `PDB-02` | P2 | AR07 | TODO |
| `PDB-05` | P2 | AR07 | TODO |
| `PDB-06` | P2 | AR05 | DONE |
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
| `PDB-07` | P3 | AR05 | DONE |
| `PDB-11` | P3 | AR05 | DONE |
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

## 8. Post-snapshot findings (outside the 180-ID count)

These entries were found after the inventory snapshot. They have their own
identities and do not change the 180-entry count above. Priority follows the
`OFFENE_PUNKTE.md` scale.

| Source ID | Priority | Primary route | Current status |
| --- | --- | --- | --- |
| `KL-150` | P1 | AR06P | TODO |
| `KL-149` | P2 | AR06P | TODO |
| `KL-151` | P2 | AR06P | TODO |
| `KL-152` | P2 | AR06P | TODO |
| `KL-153` | P2 | AR06P | TODO |


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
