# 2026-10-04 — MODEL-01 web half, part 1: create and rename in every installation

Agent: Claude, goal-ui.md owner session. Web lock taken for MODEL-01
(`babdbdc1`), held across part 1 and part 2. The status-docs lock (AR14D,
`6cec9fbb`) was taken while this package was in progress and released at 18:45
before it was published. The package was rebased onto the consolidation, and
its ledger change went straight into `docs/status/LEDGER.md` (MODEL-01 stays
`IN_PROGRESS`).

## Contract (ADR-0070, server half already published)

An id-addressed command acts in the one installation that holds the entity;
an id occurring more than once is refused. Root creates take an optional
`installationId` (u8) on `POST /api/areas`, `/api/group-ranges`,
`/api/building-parts` and `/api/group-addresses`; absent means the first
installation. `PATCH /api/installations/{id}` takes `{name}`. Cross-installation
moves are refused. The catalog route has no installation field: a device on a
line follows the line, an unassigned one goes to the first installation.

## Change

- `api.ts`: optional `installationId` on the four root creates;
  `renameInstallation`.
- `treeUtils.ts`: `owningInstallation(tree, kind, id)`,
  `deviceInstallations(tree)` (single pass), `deviceInstallation(tree, id)`.
- `ProjectExplorer.tsx`: the `isFirst` gates are gone from areas, lines,
  buildings and ranges; root rows carry `installationId`. Drag eligibility is
  computed per installation, and the drop handler checks that source and
  target share an installation. `isFirst` remains only for the unassigned
  catalog row.
- `StructureWorkspace.tsx`: root creates per installation section; child rows
  only in the parent's owning installation.
- `Inspector.tsx`: `NameField` extracted from `TopologyNameField` and reused
  for one name field per installation on the project node.
- Messages en/de: `inspector.installations`, `inspector.installationName`.

## Evidence

- RED: 17 failing cases (api 2, treeUtils 4, Explorer 5, StructureWorkspace
  3, Inspector 3) for the intended reasons (missing functions, rows, fields).
  The guard "never offers a first-installation line to a later-installation
  device" already held and protects the refusal.
- First GREEN attempt: the 3 old cross-installation guard tests failed. A drop
  event arrives without an accepted dragover, and the root drop handler had
  relied on `isFirst`. Fixed in the handler (same-installation check). The
  old "only first-installation drag sources" test encoded the replaced rule
  and was adapted, adding the case of a device placed in two installations.
- Review (in-session, before the gate): IMPORTANT, three misindented JSX
  lines left by a scripted prop removal (fixed). IMPORTANT, device ownership
  was computed O(n²) per Explorer render; replaced by one linear pass shared
  with `deviceInstallation`.
- Controls: `e2e/installations.e2e.ts` fails against the old Explorer.
  Mutants caught: owner takes first of several; drop skips the installation
  check; area row drops `installationId`; rename failure keeps the typed name;
  ambiguous device kept; unassigned not counted (the last two re-run after the
  single-pass rewrite). One equivalent mutant removed by simplification.
- Harness lesson: Vitest output captured from a non-TTY can carry colour
  codes, and a count regex then misses `Tests N failed`. Read the raw output
  before believing a "survived", and set `NO_COLOR=1`.
- Full gate, attempt 2: web build, fmt, clippy -D warnings, workspace tests 3,145 passed / 0 failed / 177 ignored in 169 blocks with 0 skip markers, four repository gates (headers 464 ok, ceiling 157; anchors 393 ok), tsc, Vitest 1,790/100 files, complete intercepted Chromium suite 103/103, whitespace; source frozen. Attempt 1 failed only in Chromium: `site.e2e.ts` (4 cases) pinned the old root-create body `{name, kind}`; the explicit `installationId: 1` is the intended contract, so the expectation was adapted with a comment and the whole gate re-run. The failed attempt is kept, not relabelled.
