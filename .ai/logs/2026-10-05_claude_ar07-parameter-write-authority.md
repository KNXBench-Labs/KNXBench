# 2026-10-05 — Claude (alpha): AR07 parameter write authority (ADR-0080)

## Why

AR07's first checkbox asks to compare current parameter handling with primary
evidence and the existing packages. A read-only census of 3,599 distinct
application programs (OriginalData + the 2026-10-03 public crawler download)
and the licensed *Project Schema23* turned up two places where KNXBench let
the user write values the manufacturer does not let the user write:

1. `Access_t` (§1.1.2.1) is "the rights for the ETS user to view and modify
   parameters". 35 % of offered refs have effective access `None`/`Read`;
   all were writable. `ParameterRef/@Access` was not even stored (nor
   reported).
2. `ParameterCalculation` (116,799 in 809 programs) derives right-hand values
   from left-hand ones by vendor script. Editing the left side without running
   the script leaves derived values stale; KNXBench never runs vendor code.

## What changed

- ProductDB schema v20: `parameter_ref.access`, `parameter_calculation_ref`,
  `application_program.write_authority_recorded`; one pass
  (`parse/write_authority.rs`) shared by ingest and the v19→v20 backfill;
  failed backfill → program unrecorded → read-only (fail closed).
- `query::write_authority`, `ParameterView::ref_access`.
- Server: `write_refusal` / `effective_access` / `push_write_refusals`;
  three new diagnostic kinds; DTO `access` is now the effective access.
- Test rewinds learn to drop v20 objects (`tests/v20_rewind`).
- Corpus matrix: v20 table excluded from the v16 projection (must stay equal),
  baseline commitment re-pinned with the measured value.

## Found, not fixed here

- `KL-156`: `Parameter`/`ParameterRef` attributes the parser does not store
  are not reported either (probe-confirmed). Next package.

## Handover to the UI owner

Adopt `parameterAccessReadOnly`, `manufacturerCalculation`,
`writeAuthorityUnavailable` in the manual kind union and both catalogues;
decide whether `access: "None"` fields are hidden or collapsed.
