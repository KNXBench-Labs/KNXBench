# 2026-09-29 — Claude — ISSUE-08 data half, P1: schema-≥21 IsActive

## What
The schema-≥21 ETS mapper (`map_com_object_v21`) used `required_bool` for
`ComObjectInstanceRef/@IsActive`. Project Schema23 §1.2.5.13 declares no
such attribute on `ComObjectInstanceRef_t`, and neither schema-≥21 sample
writes one (0/691, 0/26). Every overridden object imported as inactive.

## Evidence
- Throwaway probe (not delivered, kept in the scratch dir only) measured
  the project→device-detail path on three corpus projects and found the
  inactive flood as the first real defect.
- Corpus pin: ETS 6.3.0 867/867 active, KV 75/75, ETS4 (schema 11
  control) 907/907.
- Mutations: `required_bool` restored → `(176, 867)`; `unwrap_or(true)` →
  malformed-value unit test fails.

## Scope boundary
The download planner evaluates the product `Dynamic` tree itself; no
exporter writes `IsActive`. The fix changes the model and the HTML report
only.

## Measured for P2 (not changed)
- `NoBranchMatched`: ETS4 1,016, ETS6 978, KV 64. Of these, 997/959/64
  are legal enum values without a covering `when`; 19 per ETS project
  have a non-enum control.
- Channel ownership: every `ComObjectRefRef` in the three projects'
  programs sits under exactly one `Channel`.
