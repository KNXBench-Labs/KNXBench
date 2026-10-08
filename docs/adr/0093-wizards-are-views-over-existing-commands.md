# ADR 0093: Wizards are views over existing commands; a new project's structure is seeded atomically

Date: 2026-10-08
Status: Accepted (both wizards implemented)
Session: Post-alpha UX (wizards)

## Context

The user asked for a new-project wizard and an add-device wizard. Facts
checked in the code on 2026-10-08 before deciding:

- `NewProjectDialog.tsx` was a single form (name, installation, project
  language, group-address style). `POST /api/project/new`
  (`domain::new_project_impl`) built one installation with an **empty**
  topology, no building parts and no group ranges, and replaced the open
  project in one transaction behind the unsaved-changes guard. It leaves
  no undo history.
- Every structure element the explorer can create already has a core command
  (`CreateArea`, `CreateLine`, `CreateBuildingPart`, `CreateGroupRange`). These
  commands check duplicate area/line numbers, range ordering, nesting and
  overlap. Over REST each create is its own request and its own undo step;
  there is no generic batch route.
- `knx_core::IndividualAddress::new` bounds area and line to 0–15;
  `GroupAddress::parse` bounds main 0–31, middle 0–7 (three-level) and main
  0–31 / sub 0–2047 (two-level). A `GroupRange` nests two levels deep at most.
- The catalog workspace (`CatalogBrowser.tsx`) is non-modal and creates 1–32
  devices per request with ADR-0069 replay. `POST /api/devices` has no
  installation or building-part field (KNOWN_LIMITATIONS U12).
- Web client and server always ship as one package (desktop bundle, or
  `knx-server` serving its own bundle). The user confirmed that a newer client
  never talks to an older server, so version-skew handling is not required.

The user settled the shape in an interview (receipt
`.ai/logs/2026-10-08_claude_wizards-grilling.md`, Q1–Q18).

## Decision

1. **Wizards add no domain logic.** They are UI flows over the commands and
   routes that already exist. Every rule stays in the core, and the server
   re-checks everything the client checks.

2. **New-project wizard replaces the dialog.** Steps: details (as before),
   topology, building, group structure (skipped for free style), review.
   **Create project** works from every step and Enter on the first step still
   creates at once, so the old fast path survives. Only area 1 with line 1.1
   is pre-filled; building and group structure start empty. After success the
   wizard shows "created, not yet saved" with **Add devices now**. There is no
   save step.

3. **The starting structure is seeded atomically on the server.**
   `POST /api/project/new` takes an optional `seed` (`areas`, `buildings`,
   `groupRanges`; unknown fields refused). `knx_app::project_seed` applies it
   to the not-yet-installed replacement through the ordinary core commands, on
   a private copy, before the unsaved-changes guard runs. A refused seed
   (`422`, kind `projectSeedInvalid`, message prefixed with the node's wire
   path such as `areas[0].lines[1]`) replaces nothing. An accepted seed arrives
   together with the project, without undo entries. Bounds: the KNX address
   formats above, a blank-name/medium refusal, and at most **2,000** structure
   nodes per seed. The cap is a request-size guard, not a KNX limit. The wizard
   offers four building-part kinds only (Building, Floor, Room,
   DistributionBoard).

4. **Group-structure presets are data.** Versioned JSON files under
   `apps/knx-web/presets/group-structure/` say which axis (functions or the
   building step's floors) is the main and which the middle level, and which
   functions appear in which order. Admission refuses unknown fields, ids,
   functions and versions, and labels come from the message catalogue. A
   preset only fills the editor; nothing is sent until **Create project**.
   Main groups are numbered from 1, middle groups from 0. An axis that does
   not fit its level refuses the whole preset; it is never truncated.

5. **Add-device wizard.** A modal stepper beside the catalog workspace. It
   reuses the catalog's API, replay and diagnostics logic (the install report
   and diagnostic wording moved to `CatalogInstallReport.tsx`, shared by both). Steps: product (search or install), placement
   (installation, line, room), name/quantity/addresses, review, result.
   Parameters and group links are not part of it. `POST /api/devices` gains
   optional `installationId` and `buildingPartId`, applied in the same `Batch`
   and part of the replay fingerprint. A new read-only
   `POST /api/devices/preview` computes names and addresses without mutating
   or reserving IDs. The create request carries the previewed values
   (`expected`), and the server answers `409` (kind `catalogPreviewStale`)
   when it would now allocate different ones. Line, installation and building
   part must agree on one installation (`domain::resolve_catalog_installation`);
   a disagreement or an unknown id is a `400` from preview and create alike.
   Entry points: explorer rows under each line, each installation's
   Unassigned bucket and each room; **Add device…** in the command palette;
   **Add devices now** after a new project; **Add with wizard…** in the
   catalog, which starts at the placement step.

## Alternatives considered

- **Client chains the existing REST creates after `project/new`.** Rejected: a
  failure halfway leaves a half-built project, and a project that was just
  created would carry N undo steps of its own creation.
- **A new core "seed" command.** Rejected: it would duplicate the create
  commands' rules. Applying the existing commands to the replacement before it
  is installed gives the same atomicity with no second rule set.
- **Presets as TypeScript code.** Rejected: manufacturer-independent but still
  data; files admitted like theme packs keep the structure separable and
  testable.
- **`dryRun` flag on `POST /api/devices`.** Rejected in favour of a separate
  read-only route, so the mutating and non-mutating operations stay distinct.

## Consequences

- A new project can start with a usable topology, building tree and range
  skeleton in one step. `new_project_impl` without a seed behaves exactly as
  before.
- The server's projection of a seeded project is indistinguishable from one
  built by hand (`KB-<Kind>-<id>` provenance, `Editing` completion).
- Tests: `knx-app` seed unit tests (atomic rollback including allocators, every
  bound, the cap, style rules), `knx-server` HTTP tests
  (`tests/http_project_seed.rs`), web unit tests for draft checks, preset
  admission and every wizard step, and intercepted Chromium runs at 1440 px and
  400 px in English and German.
- Add-device tests: placement agreement across two installations (domain
  unit test), `tests/http_device_wizard.rs` (preview mutates nothing and
  reserves nothing, create equals preview in one undo step, stale `409`,
  line-less placement, unknown targets, replay fingerprint, malformed
  expectation), web unit tests for placement defaults and the wizard, and
  intercepted Chromium runs in English and German at 1440 px and 400 px.
- What the stale check does not cover is listed in KNOWN_LIMITATIONS §167.
- Floor names in the quick fill are generic and numbered (`Floor 1`); the
  wizard does not guess regional floor naming.
