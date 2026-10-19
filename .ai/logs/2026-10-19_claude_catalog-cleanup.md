# Device catalog: identity lines and placement cleanup

Owner request (19 October 2026, German): devices appear twice in the Device catalog
and it is not visible why; add a cleanup option that removes old, unneeded devices
from the catalogue.

Claim `eab8661a` (its entry and the pause entry were mistakenly signed "codex";
this session is Claude). Branch `feat/catalog-cleanup-20261019`. The owner paused
at 08:35 for a reboot and resumed with "go".

## Findings

- The catalog lists `catalog_item` rows = placements. Several can point at one
  product + Hardware2Program pair; a newer application version is installed beside
  the old one on purpose (catalogue-update contract, Project bindings).
- Rows showed only name, number, description, so different versions/sources
  looked identical.
- Devices store `product_ref` + `program_ref`, never a placement ID; programs
  resolve without placements. **But** `query::device_product` joins a placement by
  the exact product + Hardware2Program pair for the inspector's catalogue item
  name/number and the documentation export's product label. Removing the last
  placement of a pair therefore changes what those surfaces show (no data loss,
  devices keep working). Found in the controller review of the worker's backend.

## Decision

ADR-0145: explain first (additive `details`), remove selected placements only,
token-bound preview with per-row impact (last placement of the pair; open-project
device count), apply = IMMEDIATE transaction → recompute token → full backup →
delete. Manual selection; one suggestion (all but one placement per exact
manufacturer + product + Hardware2Program triple); never version ordering. No schema
change (the product-DB migration queue belongs to the Alpha 15 claim).

## Work

- Backend by a delegated worker (interrupted at the owner's pause before its
  commit; its 4 cleanup tests were green), reviewed by the controller. Added the
  `bindings_without_placement` preview fact (RED → GREEN), server routes and DTO
  `details`, 4 HTTP tests (RED → GREEN).
- Web: `CatalogItemIdentity`, `CatalogCleanupDialog`, `catalogCleanup` helper,
  EN/DE copy (du-form), sticky action row, content-sized dialog (same pattern as
  the unsaved-changes dialog), 4 browser cases in the catalogue production config.
- Screenshot review found and fixed: tall empty dialog (overlay stretch), danger
  button below the fold at 400 px (double scroll), missing filter placeholder,
  right-aligned tool buttons. Remaining: the fixture's "welcome back" toast covers
  the bottom of every dialog at 400 px; the button is still clickable.

## Pitfalls

- A test's `vi.mock("./api")` must re-implement new predicates
  (`isCatalogueCleanupStale`); a missing one throws inside `catch` and the alert
  never renders.
- `getByRole("option")` in the catalogue also matches the manufacturer select's
  options; scope to the results listbox.
- `tsc` (build) catches mock typing that Vitest does not.

Gate results and publication: see `.ai/CURRENT_STATE.md`.
