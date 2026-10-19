# WP-20: System column in the Devices list (Claude, 2026-10-19)

## Request

Owner request 3 (WP-20): a sortable, filterable *System* column in Devices,
showing System 1, 2, 7 or B, other profiles by their own names, and
*Unknown (mask XXXXh)* for an unlisted mask. Source: the mask version the
application program declares in the product database, never a mask read from a
device. First deliverable: one cited table in the core, not in the UI.

## Change

- `crates/knx-core/src/system_family.rs`: `MASK_FAMILIES` (37 rows) plus
  `declared_system()` with the states no mask, unreadable mask, unknown mask
  and family. It reuses the commissioning `MaskVersion` by import; commissioning
  files are unchanged (UP258 owner).
- `apps/knx-server/src/device_catalog.rs`: `GET /api/devices` resolves the
  device's program link and adds a closed `system` state per row.
- Web: `deviceSystem.ts` (strict admission and labels), `deviceList.ts`
  (sort/filter by label and mask), `DevicesWorkspace.tsx` (column), EN/DE
  strings.

## Controller decisions applied (resume)

- `noMaskVersion` and `invalidMask` kept; single-source rows are named in the
  code comment and in the research note's table.
- A program link to other hardware is now `applicationNotOfProduct` ("Program
  does not belong to this product"), not "No application program". The same
  honesty applies to a named but uninstalled link or program, which is now
  `applicationNotInDatabase` ("Program not installed"). `noApplicationProgram`
  remains only for an empty program reference.
- `DiagnosticsCompanion` import-graph inventory gains the pure `deviceSystem.ts`.
- Table-width totals re-measured in `table-column-widths.e2e.ts` with the
  production table-columns config: devices EN 919 → 1000 px, DE 955 → 1037 px
  (System column renders 81 / 82 px); text columns alone 880 / 917 → 961 /
  999 px. The intrinsic oracle's device-table identity changed from 12 to 13
  columns.

## Verification of the table

Every row re-checked against rendered original pages (Resources Table 7 printed
pp25–26, Profiles Table 1 p13, mask 2705h/27B0h p6, mask 2920h p4) and the
printed page footers confirmed per page with `pdftotext`. Rendered pages and
extracts stay in profile scratch.

## Notes

- The first table-lookup mutant (0300h → System 2) did not run at first: its
  pattern also matched the test's own expected table, so the guard assertion
  stopped it. It was repeated with a first-occurrence replacement and turned
  two tests red.
