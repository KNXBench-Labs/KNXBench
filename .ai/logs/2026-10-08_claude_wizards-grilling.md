# 2026-10-08 — Claude — grill-me: New-Project-Wizard & Add-Device-Wizard

Interview only. No product code, tests, build, commit, deployment or bus contact.

## Facts checked in the repository (2026-10-08 07:39)

- No wizard exists anywhere (`grep -i wizard` → only ADR-0084, research, a spec).
- `apps/knx-web/src/NewProjectDialog.tsx` (282 lines): single-form `Overlay`
  dialog — name, installation name, project text language (BCP-47), GA style;
  409 unsaved-changes prompt with Save-and-create / Discard. Calls
  `POST /api/project/new`.
- `domain::new_project_impl` (`apps/knx-server/src/domain.rs:811`) builds a
  `Project` with exactly one installation and **empty** topology, buildings
  and group ranges, then replaces state transactionally (no undo entry).
- `apps/knx-web/src/CatalogBrowser.tsx` (611 lines): non-modal centre
  workspace. Install `.knxprod` with report, manufacturer filter, search
  combobox, name, quantity 1–32, `uniqueNames`, `allocateAddresses` (needs a
  line), ADR-0069 `requestId` replay/retry, per-device diagnostics.
- `POST /api/devices` body: `lineId?`, `catalogItemId`, `name`, `quantity`,
  `requestId?`, `allocateAddresses`, `uniqueNames`. **No** `installationId`,
  **no** `buildingPartId` (KNOWN_LIMITATIONS U12: unassigned catalog device
  lands in the first installation).
- Core has `Command::Batch`, `CreateArea/Line/BuildingPart/GroupRange`,
  `MoveDeviceToBuildingPart`; server has one REST route per structure create
  (each = its own undo step). No generic batch route.
- Entry points today: welcome card, toolbar, command palette (`newProject`),
  workbench "Catalog" button (pre-fills a selected line).

## Round 1 (asked)

Q1 replace vs. add · Q2 project seed scope · Q3 atomic server-side seeding ·
Q4 device-wizard step scope · Q5 extend `POST /api/devices` with
installation/building part · Q6 save at wizard end.

Answers: pending.

## Round 1 answers (2026-10-08)

User: "1-6" — all recommendations accepted:
Q1 project wizard replaces NewProjectDialog (fast path kept); device wizard added beside catalog, shared API/logic.
Q2 seed topology + buildings + GA structure, every step skippable; only Area 1 / Line 1.1 prefilled.
Q3 atomic server-side seed via optional `seed` on `POST /api/project/new`, built with core commands.
Q4 device steps: product → placement → name/qty/addresses → review → result; no parameters/GA links.
Q5 additive `installationId`/`buildingPartId` on `POST /api/devices` in the same Batch/replay fingerprint; client verifies returned placement.
Q6 no save step; "not yet saved" notice + "add devices now".

Self-decided: en+de complete, bar/tlh via existing en fallback; one ADR.
Extra facts: BuildingPartType list in knx-core/src/building.rs; GroupRange two levels; line medium_ref opaque, UI default "MT-0"; no explorer context menu exists.

## Round 2 (asked)
Q7 modal stepper · Q8 entry points · Q9 topology editor · Q10 building editor/types · Q11 GA presets as data · Q12 dryRun preview.

## Round 2 answers (2026-10-08)

User: "7-12" — accepted: Q7 modal Overlay stepper, shared install-report component; Q8 explorer line/unassigned rows open wizard, new room-node entry, palette, project-wizard end, catalog "with wizard"; no general context menu. Q9 area/line editor, default 1/1.1, MT-0 free text. Q10 Building/Floor/Room/DistributionBoard tree, N-floor quick fill, no language-specific floor names. Q11 editor + data-file presets, floor-derived option, style-aware, no individual GAs. Q12 real server preview (refined in Q14 to a separate route).

Facts: `snapshot_revision` is response ordering, not content revision. `NewProjectBody` uses serde default without deny_unknown_fields → old server silently ignores `seed`; a `dryRun` flag on POST /api/devices would APPLY on an old server → must be a separate route.

## Round 3 (asked)
Q13 expected-values 409 · Q14 old-server capability gating / preview route · Q15 multi-installation placement · Q16 cancel/back/package side effect · Q17 seed caps & refuse-nothing-replaced · Q18 two packages + tests.

## Round 3 answers (2026-10-08)

User: "13-18" accepted, with correction to Q14: web client and server always ship as one package and are never separated → mixed-version/old-server handling is irrelevant. Consequences: no capability flag, no post-create seed comparison, no client-side placement verification for old servers (Q5 note dropped). Preview stays a separate read-only route `POST /api/devices/preview` purely for clean semantics (non-mutating vs mutating), not for compatibility.

Frontier empty → synthesis presented; awaiting explicit go.

## Synthesis accepted (2026-10-08, "passt")

User approved the synthesis without changes. Delivery split: P1 new-project wizard + atomic seed, then P2 add-device wizard + preview + placement.

## P1 delivered (2026-10-08 09:10)

- Worktree `KNXBench.worktrees/project-wizard-20261008`, branch `feature/project-wizard-20261008` off `f5aff6d5`.
- Core/app: `crates/knx-app/src/project_seed.rs` (seed types, validation, atomic apply via core commands on a clone, 2,000-node cap, 13 unit tests).
- Server: `new_seeded_project_impl` + `NewProjectSpec`/`NewProjectError` in `domain.rs`; `ProjectSeedDto` (deny_unknown_fields) in `routes.rs`; 422 kind `projectSeedInvalid`; `tests/http_project_seed.rs` (5 tests).
- Web: wizard shell `NewProjectDialog.tsx`, steps `ProjectWizard{Topology,Building,Groups,Review,Issues}.tsx`, draft logic `projectSeed.ts`, presets `groupStructurePresets.ts` + `presets/group-structure/*.json`, en/de messages, CSS; App keeps the dialog open on the "created" page, `onAddDevices` → `openCatalog` (P2 rewires to the device wizard).
- Gate (frozen inputs): Vitest 2,355/140 files, Chromium 162 + repeat 21, Rust 3,564 passed/0 failed/178 ignored, fmt, five xtask gates, diff check. Clippy: first attempt exit 255 with empty log (unexplained, no OOM in kernel log); rerun twice on identical inputs exit 0, second run re-checked knx-app/knx-server/knx-desktop.
- Self-review findings fixed before gate: Cancel vs Escape semantics (Cancel always asks when touched; Escape toggles the question); remove buttons styled subtle; per-kind add labels.
- Manual screenshot retaken against release server via a temporary tolerant copy of the manual spec (original spec times out in `dismissToasts` before the new-project shot — pre-existing harness flake, not touched).

## P2 delivered (10:00)

- Feature commit `60e16158`: add-device wizard over `POST /api/devices` (+ `installationId`, `buildingPartId`, `expected`) and the new read-only `POST /api/devices/preview`; 409 `catalogPreviewStale` on a changed project; placement in the same `Batch`.
- Web: `DeviceWizard.tsx`, `DeviceWizardProduct.tsx`, `deviceWizardPlacement.ts`, shared `CatalogInstallReport.tsx`; explorer rows (lines, every Unassigned, rooms), palette `add-device`, catalog **Add with wizard…**, project wizard's **Add devices now**.
- Gate: Vitest 2,370/142, Chromium 170 + repeat 39, Rust 3,571/0/178 ignored, clippy, fmt, xtask (headers refused two long first lines → shortened, rerun ok), `inputs_frozen=1`.
- Self-review fixes before gate: hover scale clipping product rows (CSS), stale/placement tests per installation, explorer counts asserted exactly.
