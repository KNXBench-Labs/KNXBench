# 2026-10-12 — Claude — Four group-address presentations

## What changed

The Group addresses workspace moved from one flat table to four presentations over
one shared read-only model:

- `groupAddressPresentation.ts`: occurrence rows (`range:<id>:<n>`, `address:<id>:<n>`,
  `unassigned`, `unresolved`) from stored GroupRange parent/membership only.
  Iterative cycle detection, linear counts, issue list (duplicate IDs, missing /
  ambiguous parent or membership, cycles). Ambiguous IDs: visible, not selectable.
- `groupAddressMatrix.ts`: columns per exact `device:<d>:object:<o>`, every
  directional link occurrence kept, conflicting object ownership reported.
- `GroupAddressTable.tsx` is now the controller (query, data-state filter, scope,
  render window, arrow keys); renderers: `GroupAddressTreeTable`, `GroupAddressColumns`,
  `GroupAddressAtlas`, `GroupAddressMatrix`; shared `GroupAddressLinks`,
  `GroupAddressWindowNotice`.
- Preference `groupAddressView` lives in app settings, never project data.
- `StructureWorkspace` keys the workspace by server/project incarnation + installation.

## Why the render window

CDP profile on a 5,000-address synthetic project: 6–7.5 s per view switch, dominated
by React commit and DOM insertion. A 500-row window with explicit "show more / all"
brought it to 0.6–1.5 s. No virtualization dependency was added.

## Ownership

Built the independent foundation while the DPT owner had uncommitted overlapping
UI edits; integrated after their publish `fe09a41a` with a fast-forward only.

## Evidence

Receipts (command, exit, input hashes) in the session scratch `attempts/`; summarized
in `docs/IMPLEMENTATION_STATUS.md` (2026-10-12 entry). Self-review only.
