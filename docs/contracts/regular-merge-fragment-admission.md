# Regular-planner MergeId fragment admission

Status: source contract, 25 October 2026. Pure-planner behaviour only: nothing
here was tried on a device or a bus, and no hardware compatibility is claimed.

## Decision

The regular planner (`plan_memory_download`, `plan_memory_download_with_inferences`)
refuses a `ProductProcedure` program that declares a `LoadProcedure` with a
`MergeId` beside its one regular procedure. ADR-0044 requires unmodelled product
content to fail closed. The planner previously dropped every `MergeId` procedure
before looking at its steps, so a fragment that carried work was ignored silently.

This is a **conservative narrowing of an unsupported shape**. It is not a KNX
format prohibition and not a proven incorrect device effect. The documented
`MergedProcedure` splice mechanics are not claimed unknown here; what a `MergeId`
declaration means under `LoadProcedureStyle="ProductProcedure"` is unestablished,
and KNXBench does not invent merged semantics for it.

## Behaviour

New `DownloadPlanError::MergeFragment { position, merge_id }`:

- `position` is 1-based in the full `LoadProcedures` source list; the first
  fragment in source order is named.
- `merge_id` is the raw attribute text, empty, whitespace and unknown ids
  included. Any `Some` counts as a declaration.
- The fragment's steps are neither inspected nor shown. `Display` names the
  position and the debug-escaped id only.

Order of refusals: `check_program_kind` (mask, then style), manufacturer and
application identity, `ProcedureCount`, fragment check, `DoesNotConnectFirst`,
step translation. `ProcedureCount(0)` and `ProcedureCount(2)` are unchanged and
win over a fragment; a fragment wins over a main procedure that would fail later.
`check_program_kind` is untouched, so its consumers see no change.

`knx_app::download_support::plan_category` maps the new variant to
`UnsupportedCategory::ProcedureShape`, so offline download, comparison source
checks, coverage and backup restore, which share the planner, refuse alike.

## Compatibility tradeoff

`source_still_plans` becomes false for this shape, and the shared read-only
comparison, coverage and backup-restore paths refuse it the same way. Programs
that planned before because the fragment was ignored now report a procedure-shape
refusal. This is accepted as the price of failing closed; the freshness and
recovery barriers around those paths are unchanged.

## Deliberate contract change

An earlier unit test asserted that a `MergeId` procedure "is not a second
procedure" and still planned. That positive contract is withdrawn on purpose and
the test now expects the refusal.

## Unchanged

The parser, image builder and storage are untouched. The product source remains
retained verbatim; the parsed program keeps every fragment, its id and its steps, and a test checks
the stored source bytes after ingest, after the image and after the refusal.
A program without fragments plans byte for byte as before.

## Tests

`knx-productdb` `regular_merge_fragment` (synthetic mask-0701 sources through
`ingest_file`, `build_download_image` and both planner entry points) and
`knx-app` `regular_merge_fragment_offline` (real `offline_download`).
