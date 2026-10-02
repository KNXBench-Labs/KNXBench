# Alpha-readiness evidence and dispositions

## AR00 reconciliation baseline

User-started offline alpha queue, 2026-10-01. Inspected source/maintained docs
at `307a5970ad5147437dffce4a86047e85e3319186`; the dated input inventory
[OFFENE_PUNKTE](../OFFENE_PUNKTE.md) is unchanged. This is dispatch/provenance
reconciliation, not a fresh full implementation, security or compatibility audit.
The execution owner is [alpha-release-goal](../alpha-release-goal.md); historical
[goal](../goal.md) is not a second executor. UI/commissioning remain their owners.

### Evidence adopted, not relabelled

- UI source `36e6b6af`, closure `dfa0cc79`, receipt `8a51b74d` are ancestors
  of the baseline. U0–U13 and both ISSUE-12 boxes are closed. Original
  independent GPT-6.1-Sol review: **changes required**, user-approved reviewer
  substitution; subsequent fixes and separate in-session review are not a
  second independent verdict. See `.ai/logs/2026-10-01_codex_ui-u13-fixes.md`
  and [implementation status](IMPLEMENTATION_STATUS.md).
- Native WebKitGTK/real screen-reader/discovery multicast are still unverified.
  A `DONE` UI acceptance inventory row closes that dated U13/checkbox task,
  not those separate platform boundaries or optional owner follow-ups.
- Commissioning cleanup/worker/directory hardening receipts are adopted at
  their actual offline scope. Complete per-device recovery and new exact-write
  authorization remain missing; no hardware operation is authorized here.
- Startup worktree/process inventory found no other active alpha/old-goal
  Cargo or CLI executor. The existing commissioning/review checkouts are
  foreign and untouched. This observation is not a permanent concurrency lock.
- Root research/statistics/handover and unpublished telemetry/shutdown files
  are foreign. They are not copied, edited or committed. Paperclip's old
  `7b64496` observation is historical, not current dispatch authority.
- Statistics refresh is blocked on the canonical root's foreign modified
  `stats.md` and integrated-history synchronization by its owner. Safe fallback:
  retain the report untouched, do not fabricate a refresh; unblock only when
  the owner provides a current canonical checkout and safe report disposition.

### Stale assertions verified from implementation and existing tests

| Former task | Actual contract / evidence inspected |
| --- | --- |
| KL-18 | `domain::open_project` passes `None` to `replace_project_state`; replacement transaction atomically sets `store_path`. Desktop delegates to this domain. `http_project_routes::importing_replaces_a_dirty_project_with_a_clean_baseline` asserts no imported store path. |
| KL-23 | `fs_routes::stream_temp_file` uses bounded `ServeFile` frames and body-owned temporary path. Both `fs_routes` cleanup/stream tests and `http_fs_routes` unsaved-project download regression exist; SQLite serialization still creates a temporary file. |
| KL-24 | `FsPicker.tsx` has `multiple`, native file drop and sequential `uploadFiles`. `FsPicker.test.tsx` tests sequential upload, protected-mode drop, conflicts and partial failure. Opening a project remains singular. |
| ISSUE-04 | Issue-plan rows are ticked; `http_project_routes` saved-edit/save-failure/undo-baseline tests and `App.test.tsx` locale/success-only timestamp test match the actual contract. |

These are source/test inspections plus adopted historical owner runs, **not new
product-test executions**. AR00's own doc/ledger checks are recorded in
`.ai/logs/2026-10-01_codex_alpha-queue.md`. Old headings and links are retained.

## AR01 verification delivery

The executable uses the selected runtime workspace, validates its identity,
pins metadata to that manifest and refuses missing/empty scan scopes. All
checked layering roots must be actual workspace members. The target and
coverage are emitted; see [verification targets](VERIFICATION.md).

Executed: old removed-worktree zero-file success; new deleted-target refusal;
75 unit and 12 CLI integration tests, strict xtask Clippy, fmt and all four
repository scan gates. Five guard mutations fail behaviorally and sources are
restored. Scope: 448 graph packages, 370 valid / 160 absent headers (ceiling
160), 17 generated files excluded, 323 corpus-lint Rust sources. This is not
a full product/workspace test or a real-corpus execution. In-session review
found and fixed a transitive-only policy-root gap and the missing touched-file
header; the mutation tests cover that gap after fixing the fixture exclusions.
Details: `.ai/logs/2026-10-01_codex_alpha-gate-scope.md`.

## AR02 general ID exhaustion

All nine project-local allocators now refuse exhaustion with a typed error.
The final representable ID and its entities roundtrip at native schema v9;
failed CSV/reconciliation batches apply nothing, and existing parameter edits
remain possible. Tests assert unchanged project/counters, maximum-ID
undo/redo/save/reopen, CLI exit 2 and HTTP refusal, not just a failure label.
Mapper seeded-boundary tests abort detached construction and preserve input.
See [model contract](DATA_MODEL.md) and
`.ai/logs/2026-10-01_codex_alpha-id-exhaustion.md`.

Final offline gates, 2026-10-01 UTC receipt: 141 Rust result blocks, 2,855 passed,
zero failed, 161 ignored; 77 private corpus/roundtrip tests explicitly run,
zero failure/ignored/skip markers; Web 1,312; strict Clippy, dependency policy,
typecheck, build, headers/anchors/layering/corpus policy and patch checks pass.
Three compiled behavioral mutants fail and original source is restored.
Native/ETS interoperability, actual hardware and whole-product release
acceptance are not inferred from those tests. The catalog scope and numbered
limitation counts below remain unchanged; ADR-0039 phases 3–5 still require
explicit activation (prompt unanswered, not approved).

## AR03 enforcement audit and reserved decision

[Pinned source audit](ADR0039_ENFORCEMENT_AUDIT.md) at published AR02
`e691bc1318d0785289f8132378a0f26c9a829b27`: phases 1–2 are already implemented;
the live surface has changed since the ADR's historical count. A bounded
three-package proposal is documented, not activated. `KL-129`/AR03 remain
`WAITING_DECISION`; no user answer is not approval or accepted continued
deferral. This package runs doc/ledger gates only, not new product tests.
AR02 remote/ref/artifact readback succeeded and its owned checkout, branch,
targets and scratch were removed. AR03's docs-only audit was published as
`4f47059c`; its reserved activation remains unanswered.

## AR04 storage command contract — verified; publication pending

[Explicit full-save fallback](STORAGE_COMMAND_CONTRACT.md) replaces the exported
helper's successful no-op arms without changing current production save paths.
The source audit routes all 50 command variants through one complete snapshot
write; this is not a claim of 50 individually executed variant tests.
Behavioral parameter RED/GREEN, nine focused scalar/provenance regressions,
three file-backed reopen/history/order/late-failure tests and three rejected
compiled mutants are measured. Source hashes match after restoration.
Complete coordinated gates exited 0: 142 Rust result blocks, 2,859 passed,
zero failed, 161 ignored, zero skip markers; two explicitly executed private
store roundtrips, zero ignored/failure; Web 1,312. Strict Clippy, typecheck,
build, semantic bindings, dependency and repository gates pass. All 574 guarded
source files are unchanged across gates; changed store code compilation was
verified. Publication/final receipt remains pending; dispatch rows retain
IN_PROGRESS until verified remote readback.
No parked ADR phase, U12 control, schema or commissioning gate is changed.

## Stable limitation identity

There are **110 numbered headings**, **109 distinct numbers**, two meanings of
130, and no 94. Seven are resolved/clarification-only: 18, 23, 24, 42 (AR04),
90, 95, 130-GATE (AR01). Therefore **103 numbered residual boundaries** remain:
K1=5, K2=30, K3=54, K4=13 (102 triaged); 105 is wire-evidence-only and unclassified.
No heading or legacy fragment is destructively renumbered.
`KL-8` also routes 26 (Secure); `KL-130-GATE` and `KL-130-ZOOM` identify the
separate headings exactly. All other numbered IDs use `KL-<number>`.

Unnumbered aliases below identify source sections, not new inventory tasks.
Multiple input IDs can describe separate aspects of the same section.

| Stable section ID | Exact KNOWN_LIMITATIONS heading | Input IDs / owner |
| --- | --- | --- |
| UB-BACKUP-DIRECTORIES | Backup directory synchronization is not a disk-loss or confinement proof | SAFE-03; commissioning |
| UB-BUS-ACTIVITY | Partial commissioning bus-activity snapshot (ADR-0055) | AUDIT-01, UI-04; commissioning |
| UB-STRUCTURE-EDITOR | U12 structure editor scope (ISSUE-05) | DATA-02, MODEL-01, MODEL-02, KL-127; mixed layers retain primary routes |
| UB-DEVICE-EDITOR | U11 device editor scope (ISSUE-09) | MODEL-01, MODEL-02, MODEL-03; UI |
| UB-CATALOG-BATCH | U11 catalog batch scope (ISSUE-07) | DATA-01, DATA-03, MODEL-04; mixed layers retain primary routes |
| UB-READINESS-BACKUP | Commissioning readiness and pre-write backup boundary (ADR-0049) | SAFE-02, UI-03; commissioning / UI acceptance |
| UB-PDB-DYNAMIC | PDB-9 parameter and Dynamic coverage boundary | PDB-01/02/03/04/05/06/07, R-MODULE-03; data/owner routes below |
| UB-PDB-REPORT | PDB-3 report history and coverage boundary | PDB-07/08/09/11; alpha |
| UB-PDB-METADATA | PDB-7 catalogue metadata are source strings, not capabilities | PDB-10; alpha |

The `Closed entries and historical links` section is not a tenth unnumbered
limitation. UI-03's device-checks boundary is also retained in the readiness
section, not silently invented as a numbered issue.

## Decision and external-gate contracts

| Gate / rows | Missing input and responsible owner | Safe fallback | Exact unblock condition |
| --- | --- | --- | --- |
| KL-70, KL-88, KL-134 | Maintainer-selected behavior/safety rules exist, but an explicit user **release-scope waiver** was not found in the inspected decision sources | Keep active-parameter refusal, current master-name policy and inert baggage; no new feature or execution | User accepts these precise residual boundaries for alpha, or requests scoped work; do not label technical rulings user consent |
| KL-129 / AR03 | User activation of parked ADR-0039 phases 3–5 | Keep phases 1–2 guards; AR02 handles exhaustion independently | Recorded activation scope or explicit continued-deferral decision |
| KL-40 / KL-60 / AR11 | User CSV/paging scope choice | Existing validated read-only columns and disclosed paging | User-approved schema/backend scope or explicit boundary acceptance; UI changes require owner receipt |
| KL-135 / AR12 | User version/pinning policy | Existing first-winner policy with candidates disclosed | Approved versioned selection policy or explicit deferral; frontend acceptance remains owner-dependent |
| KL-1 | Independent missing-schema/modular project samples; operator/sample owner | Preserve bytes/report unknowns; no broader compatibility claim | Authorized independent fixtures and executed corresponding import/native tests |
| R-MODULE-03/04 | Verified AllocatorRef/nested-module source/sample; research owner | Preserve opaque/raw values and bounded/read-only handling | Primary semantics plus suitable regression fixture, not a guessed formula |
| KL-13 subcases | Genuine protected ETS export and password; operator/sample owner | Current ZipCrypto library tests; AES refusal | Safe entry-path plumbing tests for AR08; real ETS sample for interoperability; real AES sample plus separate implementation evidence for AES |
| UI rows | Owner receipt at exact stated scope; GUI/accessibility evidence where disclosed | Adopt completed U13, retain platform/optional residues and existing consumers | Relevant owner's new evidence or explicit alpha exception; no duplicated U packages |
| Commissioning rows | Device-specific complete recovery, exact identity, hardware/spec evidence and separate go; commissioning owner | Fail-closed actions and disclosed simulator/one-device scope | Owner's scoped receipt and explicit release disposition; this session never opens a real socket |
| RELEASE-03 | User manual location/screenshots/acceptance | Existing maintainer manual, ADR-0024 no bundled docs | Recorded user policy and dated finished-product manual checklist |
| RELEASE-01/02 | Finished candidate/artifact, integrated gates, chosen independent whole-product reviewer | Not ready; owner UI review is not the whole-product review | AR18 prerequisites and actual independent verdict with blocking findings resolved |
| RELEASE-04 | Exact release/tag/publication approval; user | No tag/upload | Explicit AR19 approval of the actual reviewed candidate |
| LATER rows | Separate scoped decision/ADR; future/tooling owner | No alpha implementation prerequisite | New explicit scope; telemetry/private/history artifacts remain unversioned unless separately authorized |

`ACCEPTED_BOUNDARY` below requires the named dated decision or accepted ADR;
it is not inferred from "no task exists". FUTURE-05 is merely unscheduled and
is `LATER`, not an accepted release exception. The three disputed behavior rows
remain `WAITING_DECISION`. No wait blocks unrelated ready offline packages.

## Complete per-ID execution ledger

Each row retains its one primary route and original priority. The evidence
column points to the **current retained source**, not a claim that its entire
implementation was freshly tested. Open alpha code contracts are verified in
their named AR package. Owner rows have the shared fallback/unblock contract
above and are not new tasks assigned to an already closed owner queue.

| Source ID | Priority | Primary route | Status | Evidence / remaining disposition |
| --- | --- | --- | --- | --- |
| `KL-116` | P0 | `goal-commission.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §116; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `KL-139` | P0 | `goal-commission.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §139; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `KL-140` | P0 | `goal-commission.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §140; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `SAFE-01` | P0 | `goal-commission.md` — owner only | WAITING_OWNER | goal-commission.md status / docs/RESEARCH.md §22–24; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `KL-79` | P1 | `goal-ui.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §79; U13 implementation closed; retain this source's platform/optional residue; UI owner contract above |
| `KL-61` | P1 | AR09 | TODO | docs/KNOWN_LIMITATIONS.md §61; Retained boundary; AR09 verifies subcases before changing status |
| `KL-99` | P1 | `goal-commission.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §99; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `KL-112` | P1 | `goal-commission.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §112; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `KL-136` | P1 | `goal-commission.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §136; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `KL-138` | P1 | `goal-commission.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §138; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `KL-141` | P1 | `goal-commission.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §141; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `KL-142` | P1 | `goal-commission.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §142; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `KL-7` | P1 | `goal-commission.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §7; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `KL-92` | P1 | `goal-commission.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §92; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `DEBUG-01` | P1 | `goal-commission.md` — owner only | WAITING_OWNER | goal-commission.md status / ADR-0051; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `SAFE-02` | P1 | `goal-commission.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md: Commissioning readiness / ADR-0049; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `SAFE-03` | P1 | `goal-commission.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §7 / goal-commission.md §3; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `DATA-01` | P1 | AR02 | DONE | Nine checked allocators; synthetic maximum-ID/native/CSV/CLI/HTTP/mapper and rollback regressions; three behavioral mutants; final offline gate receipt .ai/logs/2026-10-01_codex_alpha-id-exhaustion.md. Parked mutation enforcement and catalog UI scope remain separate. |
| `KL-129` | P1 | AR03 | WAITING_DECISION | docs/KNOWN_LIMITATIONS.md §129; Reserved user decision; see decision contract above |
| `KL-106` | P1 | AR13 | TODO | docs/KNOWN_LIMITATIONS.md §106; Retained boundary; AR13 verifies subcases before changing status |
| `DOC-01` | P1 | AR00 | DONE | goal.md §3 / §12.2 / docs/LIMITATION_TRIAGE.md / apps/knx-server/src/domain.rs; AR00 source/test and provenance reconciliation above; doc/ledger gate receipt in alpha-queue log |
| `KL-1` | P1 | AR06 | BLOCKED_EXTERNAL | docs/KNOWN_LIMITATIONS.md §1; Missing independent sample/source; exact fallback/unblock contract above; no invented semantics |
| `KL-13` | P1 | AR08 | TODO | docs/KNOWN_LIMITATIONS.md §13; Retained boundary; AR08 verifies subcases before changing status |
| `PDB-09` | P1 | AR05 | TODO | docs/KNOWN_LIMITATIONS.md: PDB-3 report history and coverage boundary; Retained boundary; AR05 verifies subcases before changing status |
| `R-MODULE-01` | P1 | `goal-commission.md` — owner only | WAITING_OWNER | docs/RESEARCH.md §19.11 / goal-commission.md; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `KL-130-GATE` | P1 | AR01 | DONE | docs/KNOWN_LIMITATIONS.md §130 (Gate); AR01 runtime-root/coverage CLI and scan regressions, old removed-tree reproduction, five behavioral mutants; verification delivery above |
| `RELEASE-01` | P1 | AR18 | WAITING_OWNER | goal.md §9–10; Named final acceptance prerequisites above; not ready on historical receipts alone |
| `RELEASE-02` | P1 | AR18 | WAITING_OWNER | goal.md §1 / §10; Named final acceptance prerequisites above; not ready on historical receipts alone |
| `KL-22` | P1 | AR13 | TODO | docs/KNOWN_LIMITATIONS.md §22; Retained boundary; AR13 verifies subcases before changing status |
| `KL-63` | P1 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY | docs/KNOWN_LIMITATIONS.md §63; goal.md §6: T22 explicitly parked outside v1 must-haves |
| `KL-8` | P1 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY | docs/KNOWN_LIMITATIONS.md §8/26; goal.md §6: Secure deferred 2026-09-11; not Secure support |
| `UI-01` | P1 | `goal-ui.md` — owner only | DONE | goal-ui.md §0 / docs/IMPLEMENTATION_STATUS.md: U13; U13 closure dfa0cc79 / receipt 8a51b74d; source 36e6b6af; native/multicast boundaries retained |
| `UI-02` | P1 | `goal-ui.md` — owner only | DONE | goal-ui.md §0 / docs/superpowers/plans/2026-09-21-user-reported-issues.md; U13 closure dfa0cc79 / receipt 8a51b74d; source 36e6b6af; native/multicast boundaries retained |
| `KL-126` | P2 | AR14 | TODO | docs/KNOWN_LIMITATIONS.md §126; Retained boundary; AR14 verifies subcases before changing status |
| `KL-29` | P2 | AR14 | TODO | docs/KNOWN_LIMITATIONS.md §29; Retained boundary; AR14 verifies subcases before changing status |
| `KL-31` | P2 | AR14 | TODO | docs/KNOWN_LIMITATIONS.md §31; Retained boundary; AR14 verifies subcases before changing status |
| `KL-62` | P2 | AR14 | TODO | docs/KNOWN_LIMITATIONS.md §62; Retained boundary; AR14 verifies subcases before changing status |
| `KL-72` | P2 | AR14 | TODO | docs/KNOWN_LIMITATIONS.md §72; Retained boundary; AR14 verifies subcases before changing status |
| `KL-73` | P2 | AR14 | TODO | docs/KNOWN_LIMITATIONS.md §73; Retained boundary; AR14 verifies subcases before changing status |
| `KL-74` | P2 | AR14 | TODO | docs/KNOWN_LIMITATIONS.md §74; Retained boundary; AR14 verifies subcases before changing status |
| `KL-75` | P2 | AR14 | TODO | docs/KNOWN_LIMITATIONS.md §75; Retained boundary; AR14 verifies subcases before changing status |
| `KL-77` | P2 | AR14 | TODO | docs/KNOWN_LIMITATIONS.md §77; Retained boundary; AR14 verifies subcases before changing status |
| `KL-78` | P2 | AR14 | TODO | docs/KNOWN_LIMITATIONS.md §78; Retained boundary; AR14 verifies subcases before changing status |
| `KL-105` | P2 | `goal-commission.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §105; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `KL-108` | P2 | `goal-commission.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §108; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `KL-101` | P2 | `goal-commission.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §101; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `KL-104` | P2 | `goal-commission.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §104; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `KL-109` | P2 | `goal-commission.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §109; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `KL-111` | P2 | `goal-commission.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §111; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `KL-113` | P2 | `goal-commission.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §113; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `KL-114` | P2 | `goal-commission.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §114; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `KL-143` | P2 | `goal-commission.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §143; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `KL-144` | P2 | `goal-commission.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §144; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `KL-145` | P2 | `goal-commission.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §145; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `KL-93` | P2 | `goal-commission.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §93; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `GAP-T30-01` | P2 | `goal-commission.md` — owner only | WAITING_OWNER | docs/RESEARCH.md §8.7.15 / docs/superpowers/specs/2026-09-13-commissioning-download-design.md §12; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `GAP-T30-02` | P2 | `goal-commission.md` — owner only | WAITING_OWNER | docs/RESEARCH.md §8.7.15 / docs/superpowers/specs/2026-09-13-commissioning-download-design.md §12; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `GAP-T30-03` | P2 | `goal-commission.md` — owner only | WAITING_OWNER | docs/RESEARCH.md §8.6.7 / §8.7.15; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `GAP-T30-07` | P2 | `goal-commission.md` — owner only | WAITING_OWNER | docs/RESEARCH.md §8.7.15 / docs/superpowers/specs/2026-09-13-commissioning-download-design.md §12; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `GAP-T30-08` | P2 | `goal-commission.md` — owner only | WAITING_OWNER | docs/RESEARCH.md §8.7.15 / docs/superpowers/specs/2026-09-13-commissioning-download-design.md §12; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `GAP-T30-09` | P2 | `goal-commission.md` — owner only | WAITING_OWNER | docs/RESEARCH.md §8.7.14–15 / docs/superpowers/specs/2026-09-13-commissioning-download-design.md §12; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `R-DL-01` | P2 | `goal-commission.md` — owner only | WAITING_OWNER | docs/RESEARCH.md §8.6.7; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `R-DL-02` | P2 | `goal-commission.md` — owner only | WAITING_OWNER | docs/RESEARCH.md §8.7.15 / commissioning-download-design.md R11; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `DATA-02` | P2 | AR04 | IN_PROGRESS | STORAGE_COMMAND_CONTRACT.md; full-save fallback/native failure-history tests verified; integrated gates/publication pending; U12 editor scope is not lifted |
| `DATA-03` | P2 | `goal-ui.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md: U11 catalog batch scope; U13 implementation closed; retain this source's platform/optional residue; UI owner contract above |
| `KL-87` | P2 | AR05 | TODO | docs/KNOWN_LIMITATIONS.md §87; Retained boundary; AR05 verifies subcases before changing status |
| `AUDIT-01` | P2 | `goal-commission.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md: Partial commissioning bus-activity snapshot / ADR-0055/0056; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `KL-137` | P2 | `goal-ui.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §137; U13 implementation closed; retain this source's platform/optional residue; UI owner contract above |
| `KL-36` | P2 | `goal-ui.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §36; U13 implementation closed; retain this source's platform/optional residue; UI owner contract above |
| `KL-82` | P2 | `goal-ui.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §82; U13 implementation closed; retain this source's platform/optional residue; UI owner contract above |
| `DOC-03` | P2 | AR15 | TODO | docs/manual/known-issues.md / docs/manual/ideas-and-roadmap.md; Retained boundary; AR15 verifies subcases before changing status |
| `RELEASE-03` | P2 | AR16 | WAITING_OWNER | goal.md §5 / docs/manual/README.md / ADR-0024; Named final acceptance prerequisites above; not ready on historical receipts alone |
| `RELEASE-04` | P2 | AR19 | WAITING_DECISION | goal.md §5 / docs/ROADMAP.md Session 7; Reserved user decision; see decision contract above |
| `KL-127` | P2 | `goal-ui.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §127; U13 implementation closed; retain this source's platform/optional residue; UI owner contract above |
| `MODEL-01` | P2 | `goal-ui.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md: U11 device editor scope / U12 structure editor scope; U13 implementation closed; retain this source's platform/optional residue; UI owner contract above |
| `MODEL-02` | P2 | `goal-ui.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md: U11 device editor scope / U12 structure editor scope; U13 implementation closed; retain this source's platform/optional residue; UI owner contract above |
| `MODEL-03` | P2 | `goal-ui.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md: U11 device editor scope; U13 implementation closed; retain this source's platform/optional residue; UI owner contract above |
| `IMPORT-05` | P2 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY | docs/manual/ideas-and-roadmap.md / docs/COMPATIBILITY.md; manual ideas-and-roadmap / COMPATIBILITY: encrypted product packages explicitly excluded |
| `IMPORT-06` | P2 | AR06 | TODO | docs/manual/known-issues.md / docs/KNOWN_LIMITATIONS.md §2; Retained boundary; AR06 verifies subcases before changing status |
| `KL-11` | P2 | AR06 | TODO | docs/KNOWN_LIMITATIONS.md §11; Retained boundary; AR06 verifies subcases before changing status |
| `KL-125` | P2 | AR06 | TODO | docs/KNOWN_LIMITATIONS.md §125; Retained boundary; AR06 verifies subcases before changing status |
| `KL-128` | P2 | AR06 | TODO | docs/KNOWN_LIMITATIONS.md §128; Retained boundary; AR06 verifies subcases before changing status |
| `KL-15` | P2 | AR06 | TODO | docs/KNOWN_LIMITATIONS.md §15; Retained boundary; AR06 verifies subcases before changing status |
| `KL-2` | P2 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY | docs/KNOWN_LIMITATIONS.md §2; DIN-26 decision, 2026-09-27 (all twelve accepted; ambiguous-DPT residue only for KL-12) |
| `KL-14` | P2 | AR10 | TODO | docs/KNOWN_LIMITATIONS.md §14; Retained boundary; AR10 verifies subcases before changing status |
| `KL-37` | P2 | AR10 | TODO | docs/KNOWN_LIMITATIONS.md §37; Retained boundary; AR10 verifies subcases before changing status |
| `KL-64` | P2 | AR10 | TODO | docs/KNOWN_LIMITATIONS.md §64; Retained boundary; AR10 verifies subcases before changing status |
| `KL-66` | P2 | AR10 | TODO | docs/KNOWN_LIMITATIONS.md §66; Retained boundary; AR10 verifies subcases before changing status |
| `KL-12` | P2 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY | docs/KNOWN_LIMITATIONS.md §12; DIN-26 decision, 2026-09-27 (all twelve accepted; ambiguous-DPT residue only for KL-12) |
| `KL-135` | P2 | AR12 | WAITING_DECISION | docs/KNOWN_LIMITATIONS.md §135; Reserved user decision; see decision contract above |
| `KL-146` | P2 | AR07 | TODO | docs/KNOWN_LIMITATIONS.md §146; Retained boundary; AR07 verifies subcases before changing status |
| `KL-3` | P2 | AR07 | TODO | docs/KNOWN_LIMITATIONS.md §3; Retained boundary; AR07 verifies subcases before changing status |
| `KL-6` | P2 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY | docs/KNOWN_LIMITATIONS.md §6; goal.md §6: vendor binaries not executed |
| `KL-68` | P2 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY | docs/KNOWN_LIMITATIONS.md §68 / docs/RESEARCH.md §4.4 (RepeatIndex); goal.md §6: dated 2026-09-20 module boundary decision |
| `KL-69` | P2 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY | docs/KNOWN_LIMITATIONS.md §69; goal.md §6: dated 2026-09-20 module boundary decision |
| `KL-71` | P2 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY | docs/KNOWN_LIMITATIONS.md §71; goal.md §6: dated 2026-09-20 module boundary decision |
| `KL-85` | P2 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY | docs/KNOWN_LIMITATIONS.md §85; DIN-26 decision, 2026-09-27 (all twelve accepted; ambiguous-DPT residue only for KL-12) |
| `KL-86` | P2 | AR05 | TODO | docs/KNOWN_LIMITATIONS.md §86; Retained boundary; AR05 verifies subcases before changing status |
| `PDB-01` | P2 | AR07 | TODO | docs/KNOWN_LIMITATIONS.md: PDB-9 parameter and Dynamic coverage boundary; Retained boundary; AR07 verifies subcases before changing status |
| `PDB-02` | P2 | AR07 | TODO | docs/KNOWN_LIMITATIONS.md: PDB-9 parameter and Dynamic coverage boundary; Retained boundary; AR07 verifies subcases before changing status |
| `PDB-05` | P2 | AR07 | TODO | docs/KNOWN_LIMITATIONS.md: PDB-9 parameter and Dynamic coverage boundary; Retained boundary; AR07 verifies subcases before changing status |
| `PDB-06` | P2 | AR05 | TODO | docs/KNOWN_LIMITATIONS.md: PDB-9 parameter and Dynamic coverage boundary; Retained boundary; AR05 verifies subcases before changing status |
| `PDB-08` | P2 | AR06 | TODO | docs/KNOWN_LIMITATIONS.md: PDB-3 report history and coverage boundary; Retained boundary; AR06 verifies subcases before changing status |
| `PDB-10` | P2 | AR06 | TODO | docs/KNOWN_LIMITATIONS.md: PDB-7 catalogue metadata are source strings; Retained boundary; AR06 verifies subcases before changing status |
| `R-DYNAMIC-01` | P2 | AR07 | TODO | docs/RESEARCH.md §4.3 / Open questions; Retained boundary; AR07 verifies subcases before changing status |
| `R-MODULE-03` | P2 | AR07 | BLOCKED_EXTERNAL | docs/RESEARCH.md §4.4 / docs/KNOWN_LIMITATIONS.md: PDB-9; Missing independent sample/source; exact fallback/unblock contract above; no invented semantics |
| `R-MODULE-04` | P2 | AR07 | BLOCKED_EXTERNAL | docs/RESEARCH.md §4.4 / docs/GAP_ANALYSIS_ETS.md A3; Missing independent sample/source; exact fallback/unblock contract above; no invented semantics |
| `KL-133` | P2 | `goal-ui.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §133; U13 implementation closed; retain this source's platform/optional residue; UI owner contract above |
| `KL-38` | P2 | AR11 | TODO | docs/KNOWN_LIMITATIONS.md §38; Retained boundary; AR11 verifies subcases before changing status |
| `KL-39` | P2 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY | docs/KNOWN_LIMITATIONS.md §39; DIN-26 decision, 2026-09-27 (all twelve accepted; ambiguous-DPT residue only for KL-12) |
| `KL-40` | P2 | AR11 | TODO | docs/KNOWN_LIMITATIONS.md §40; Retained boundary; AR11 verifies subcases before changing status |
| `KL-44` | P2 | AR11 | TODO | docs/KNOWN_LIMITATIONS.md §44; Retained boundary; AR11 verifies subcases before changing status |
| `KL-47` | P2 | AR11 | TODO | docs/KNOWN_LIMITATIONS.md §47; Retained boundary; AR11 verifies subcases before changing status |
| `KL-51` | P2 | AR11 | TODO | docs/KNOWN_LIMITATIONS.md §51; Retained boundary; AR11 verifies subcases before changing status |
| `KL-60` | P2 | AR11 | TODO | docs/KNOWN_LIMITATIONS.md §60; Retained boundary; AR11 verifies subcases before changing status |
| `R-SEC-01` | P2 | Later / separate scope — not an alpha task | LATER | docs/RESEARCH.md §9 / Open questions; Separate future/tooling scope; not an authorized alpha implementation or release waiver |
| `UI-03` | P2 | `goal-ui.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md: Device-checks UI boundary; U13 implementation closed; retain this source's platform/optional residue; UI owner contract above |
| `UI-04` | P2 | `goal-commission.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md: Partial commissioning bus-activity snapshot / ADR-0055/0056; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `KL-130-ZOOM` | P2 | `goal-ui.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §130 (Zoom); U13 implementation closed; retain this source's platform/optional residue; UI owner contract above |
| `KL-20` | P2 | `goal-ui.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §20; U13 implementation closed; retain this source's platform/optional residue; UI owner contract above |
| `KL-124` | P3 | `goal-ui.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §124; U13 implementation closed; retain this source's platform/optional residue; UI owner contract above |
| `KL-76` | P3 | AR14 | TODO | docs/KNOWN_LIMITATIONS.md §76; Retained boundary; AR14 verifies subcases before changing status |
| `KL-102` | P3 | AR14 | TODO | docs/KNOWN_LIMITATIONS.md §102; Retained boundary; AR14 verifies subcases before changing status |
| `KL-110` | P3 | `goal-commission.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §110; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `KL-115` | P3 | `goal-commission.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §115; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `GAP-T30-04` | P3 | `goal-commission.md` — owner only | WAITING_OWNER | docs/RESEARCH.md §8.7.15 / docs/spec-audits/2026-09-19-cp-3_5_3-partial-download.md; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `HISTORY-01` | P3 | Later / separate scope — not an alpha task | LATER | docs/GAP_ANALYSIS_ETS.md B11; Separate future/tooling scope; not an authorized alpha implementation or release waiver |
| `HISTORY-02` | P3 | Later / separate scope — not an alpha task | LATER | docs/GAP_ANALYSIS_ETS.md C1 / D8; Separate future/tooling scope; not an authorized alpha implementation or release waiver |
| `DOC-02` | P3 | AR00 | DONE | docs/KNOWN_LIMITATIONS.md / docs/LIMITATION_TRIAGE.md; AR00 source/test and provenance reconciliation above; doc/ledger gate receipt in alpha-queue log |
| `MODEL-04` | P3 | `goal-ui.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md: U11 catalog batch scope; U13 implementation closed; retain this source's platform/optional residue; UI owner contract above |
| `MODEL-05` | P3 | Later / separate scope — not an alpha task | LATER | docs/RESEARCH.md §17.3; Separate future/tooling scope; not an authorized alpha implementation or release waiver |
| `MODEL-06` | P3 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY | goal.md §6 / ADR-0019; accepted ADR-0019 / goal.md §6: topological model, no spatial canvas |
| `MODEL-07` | P3 | Later / separate scope — not an alpha task | LATER | docs/RESEARCH.md §15 / docs/ROADMAP.md: Functions; Separate future/tooling scope; not an authorized alpha implementation or release waiver |
| `IMPORT-01` | P3 | Later / separate scope — not an alpha task | LATER | docs/GAP_ANALYSIS_ETS.md C3; Separate future/tooling scope; not an authorized alpha implementation or release waiver |
| `IMPORT-02` | P3 | Later / separate scope — not an alpha task | LATER | docs/RESEARCH.md: local installation data (uncommitted addition); Separate future/tooling scope; not an authorized alpha implementation or release waiver |
| `IMPORT-03` | P3 | `goal-commission.md` — owner only | WAITING_OWNER | docs/RESEARCH.md: telegram-capture audit (uncommitted addition) / goal-commission.md K19; Adopt scoped owner evidence, retain safety/spec/hardware residue; commissioning gate contract above |
| `IMPORT-04` | P3 | Later / separate scope — not an alpha task | LATER | docs/RESEARCH.md: local installation data (uncommitted addition); Separate future/tooling scope; not an authorized alpha implementation or release waiver |
| `KL-100` | P3 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY | docs/KNOWN_LIMITATIONS.md §100; accepted ADR-0024 §2: catalogue paragraphs, not a second help store |
| `KL-48` | P3 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY | docs/KNOWN_LIMITATIONS.md §48; DIN-26 decision, 2026-09-27 (all twelve accepted; ambiguous-DPT residue only for KL-12) |
| `KL-134` | P3 | Recorded boundary — AR00 provenance / AR15 claims | WAITING_DECISION | docs/KNOWN_LIMITATIONS.md §134; Technical ruling is documented; explicit user release waiver missing; see decision contract above |
| `KL-70` | P3 | Recorded boundary — AR00 provenance / AR15 claims | WAITING_DECISION | docs/KNOWN_LIMITATIONS.md §70; Technical ruling is documented; explicit user release waiver missing; see decision contract above |
| `KL-88` | P3 | Recorded boundary — AR00 provenance / AR15 claims | WAITING_DECISION | docs/KNOWN_LIMITATIONS.md §88; Technical ruling is documented; explicit user release waiver missing; see decision contract above |
| `PDB-03` | P3 | AR07 | TODO | docs/KNOWN_LIMITATIONS.md: PDB-9 parameter and Dynamic coverage boundary; Retained boundary; AR07 verifies subcases before changing status |
| `PDB-04` | P3 | Later / separate scope — not an alpha task | LATER | docs/KNOWN_LIMITATIONS.md: PDB-9 parameter and Dynamic coverage boundary; Separate future/tooling scope; not an authorized alpha implementation or release waiver |
| `PDB-07` | P3 | AR05 | TODO | docs/KNOWN_LIMITATIONS.md: PDB-9 / PDB-3 report history and coverage boundary; Retained boundary; AR05 verifies subcases before changing status |
| `PDB-11` | P3 | AR05 | TODO | docs/KNOWN_LIMITATIONS.md: PDB-3 report history and coverage boundary; Retained boundary; AR05 verifies subcases before changing status |
| `FUTURE-01` | P3 | Later / separate scope — not an alpha task | LATER | docs/RESEARCH.md §13 / goal.md §7; Separate future/tooling scope; not an authorized alpha implementation or release waiver |
| `FUTURE-02` | P3 | Later / separate scope — not an alpha task | LATER | docs/RESEARCH.md §14 / goal.md §7; Separate future/tooling scope; not an authorized alpha implementation or release waiver |
| `FUTURE-03` | P3 | Later / separate scope — not an alpha task | LATER | ADR-0031 / goal.md §7 / docs/ROADMAP.md: In-application help; Separate future/tooling scope; not an authorized alpha implementation or release waiver |
| `FUTURE-04` | P3 | Later / separate scope — not an alpha task | LATER | docs/RESEARCH.md §16 / goal.md §7; Separate future/tooling scope; not an authorized alpha implementation or release waiver |
| `FUTURE-05` | P3 | Recorded boundary — AR00 provenance / AR15 claims | LATER | goal.md §6 / docs/GAP_ANALYSIS_ETS.md C6; Separate future/tooling scope; not an authorized alpha implementation or release waiver |
| `FUTURE-06` | P3 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY | goal.md §6; goal.md §6: Linux-first platform exclusion |
| `FUTURE-07` | P3 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY | goal.md §6; goal.md §6: logo remains user-owned |
| `KL-107` | P3 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY | docs/KNOWN_LIMITATIONS.md §107; accepted ADR-0025 and goal.md §6: data extension, no code plug-in API |
| `KL-16` | P3 | AR15 | TODO | docs/KNOWN_LIMITATIONS.md §16; Retained boundary; AR15 verifies subcases before changing status |
| `KL-42` | P3 | AR04 | IN_PROGRESS | docs/KNOWN_LIMITATIONS.md §42; no-op/doc defect corrected with native/mutation evidence; package gates/publication pending |
| `KL-65` | P3 | AR13 | TODO | docs/KNOWN_LIMITATIONS.md §65; Retained boundary; AR13 verifies subcases before changing status |
| `KL-41` | P3 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY | docs/KNOWN_LIMITATIONS.md §41; DIN-26 decision, 2026-09-27 (all twelve accepted; ambiguous-DPT residue only for KL-12) |
| `KL-45` | P3 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY | docs/KNOWN_LIMITATIONS.md §45; DIN-26 decision, 2026-09-27 (all twelve accepted; ambiguous-DPT residue only for KL-12) |
| `KL-46` | P3 | AR15 | TODO | docs/KNOWN_LIMITATIONS.md §46; Retained boundary; AR15 verifies subcases before changing status |
| `KL-52` | P3 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY | docs/KNOWN_LIMITATIONS.md §52; DIN-26 decision, 2026-09-27 (all twelve accepted; ambiguous-DPT residue only for KL-12) |
| `KL-53` | P3 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY | docs/KNOWN_LIMITATIONS.md §53; DIN-26 decision, 2026-09-27 (all twelve accepted; ambiguous-DPT residue only for KL-12) |
| `KL-54` | P3 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY | docs/KNOWN_LIMITATIONS.md §54; DIN-26 decision, 2026-09-27 (all twelve accepted; ambiguous-DPT residue only for KL-12) |
| `KL-55` | P3 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY | docs/KNOWN_LIMITATIONS.md §55; DIN-26 decision, 2026-09-27 (all twelve accepted; ambiguous-DPT residue only for KL-12) |
| `KL-56` | P3 | Recorded boundary — AR00 provenance / AR15 claims | ACCEPTED_BOUNDARY | docs/KNOWN_LIMITATIONS.md §56; DIN-26 decision, 2026-09-27 (all twelve accepted; ambiguous-DPT residue only for KL-12) |
| `KL-9` | P3 | AR15 | TODO | docs/KNOWN_LIMITATIONS.md §9; Retained boundary; AR15 verifies subcases before changing status |
| `TOOLS-06` | P3 | AR00 | DONE | docs/paperclip-shutdown/STATUS.md / goal.md §12; AR00 source/test and provenance reconciliation above; doc/ledger gate receipt in alpha-queue log |
| `TOOLS-01` | P3 | Later / separate scope — not an alpha task | LATER | .ai/CURRENT_STATE.md (local statistics handover) / docs/AI_STATS_TELEMETRY_PLAN.md; Separate future/tooling scope; not an authorized alpha implementation or release waiver |
| `TOOLS-02` | P3 | Later / separate scope — not an alpha task | LATER | docs/AI_STATS_TELEMETRY_PLAN.md; Separate future/tooling scope; not an authorized alpha implementation or release waiver |
| `TOOLS-03` | P3 | Later / separate scope — not an alpha task | LATER | docs/AI_STATS_TELEMETRY_PLAN.md / .ai/CURRENT_STATE.md (local); Separate future/tooling scope; not an authorized alpha implementation or release waiver |
| `TOOLS-04` | P3 | Later / separate scope — not an alpha task | LATER | docs/AI_STATS_TELEMETRY_PLAN.md / .ai/CURRENT_STATE.md (local); Separate future/tooling scope; not an authorized alpha implementation or release waiver |
| `TOOLS-05` | P3 | Later / separate scope — not an alpha task | LATER | docs/AI_STATS_TELEMETRY_PLAN.md; Separate future/tooling scope; not an authorized alpha implementation or release waiver |
| `FUTURE-08` | P3 | Later / separate scope — not an alpha task | LATER | docs/manual/ideas-and-roadmap.md; Separate future/tooling scope; not an authorized alpha implementation or release waiver |
| `KL-121` | P3 | `goal-ui.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §121; U13 implementation closed; retain this source's platform/optional residue; UI owner contract above |
| `KL-43` | P3 | `goal-ui.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §43; U13 implementation closed; retain this source's platform/optional residue; UI owner contract above |
| `KL-97` | P3 | `goal-ui.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §97; U13 implementation closed; retain this source's platform/optional residue; UI owner contract above |
| `KL-98` | P3 | `goal-ui.md` — owner only | WAITING_OWNER | docs/KNOWN_LIMITATIONS.md §98; U13 implementation closed; retain this source's platform/optional residue; UI owner contract above |
| `UX-01` | P3 | `goal-ui.md` — owner only | WAITING_OWNER | docs/GAP_ANALYSIS_ETS.md B10 / docs/IMPLEMENTATION_STATUS.md T11; U13 implementation closed; retain this source's platform/optional residue; UI owner contract above |
| `UX-02` | P3 | `goal-ui.md` — owner only | WAITING_OWNER | apps/knx-web/src/CatalogBrowser.tsx / docs/manual/known-issues.md; U13 implementation closed; retain this source's platform/optional residue; UI owner contract above |
| `UX-03` | P3 | `goal-ui.md` — owner only | WAITING_OWNER | apps/knx-web/src/NewProjectDialog.tsx / apps/knx-web/src/messages/en.ts; U13 implementation closed; retain this source's platform/optional residue; UI owner contract above |

**Mechanically counted execution statuses:** ACCEPTED_BOUNDARY=24, BLOCKED_EXTERNAL=3, DONE=6, LATER=20, TODO=54, WAITING_DECISION=6, WAITING_OWNER=67; total=180.
