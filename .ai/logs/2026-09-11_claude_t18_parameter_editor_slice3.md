# T18 slice 3 — the parameter editor (2026-09-11)

Worktree `.worktrees/t18-parameter-editor`, branch `t18-parameter-editor`,
BASE `d44af72`. Design: `docs/superpowers/specs/2026-09-11-parameter-editor-design.md`
(D20-D26). Five tasks, each reviewed before the next started.

## What the four code tasks changed

1. **Product-database read model** (`crates/knx-productdb/src/query.rs`).
   Added `ParameterView` and two query functions:
   - `parameter_views(conn, program_id) -> Vec<ParameterView>` — one row
     per declared `ParameterRef` in an application program, carrying
     `id`, `display_order`, `tag`, `name`, `text`, `text_layer`, `kind`,
     `access`, `min_inclusive`, `max_inclusive`, `enum_options`.
   - `parameter_ref_ids(conn, program_id) -> HashSet<String>` — the set
     of declared ids, used to classify a stored `ParameterInstance` as
     "matches a declared parameter" vs "stale" (no corresponding
     `ParameterRef` in the currently-loaded program).
   No schema change. This is a read model over existing tables.

2. **`Command` pair** (`crates/knx-core/src/command.rs`,
   `crates/knx-store/src/command_sync.rs`). Added
   `Command::SetParameterValue { id, device, ets_id, raw: String }` and
   `Command::RestoreParameterValue { id, device, ets_id, raw: Option<String> }`
   for undo/redo. Wired into the existing command-log/undo-stack
   machinery, no new storage table.

3. **HTTP endpoints** (`apps/knx-server`). `GET`/`POST
   /api/device/{id}/parameters`, returning/accepting a `ParameterPanelDto`
   (`ParameterSectionDto`, `ModuleScopeDto`, `ParameterFieldDto`,
   `EnumOptionDto`, `StaleParameterDto`, `ParameterDiagnosticDto`). POST
   validates against the field's `kind`/enum/min/max before dispatching
   `Command::SetParameterValue`, then re-runs the evaluator and returns
   the same shape as GET so the client sees the updated activation set
   in the same response. An unresolvable program (no application program
   id on the device, or program not present in the product database)
   returns 200 with `programId: null` and an empty section list, not an
   error — `stale` entries (raw stored values with no matching declared
   parameter) are still reported in that case.

4. **Web panel** (`apps/knx-web/src/ParameterPanel.tsx` +
   `ParameterPanel.test.tsx`). Renders sections/fields from the DTO,
   dispatches POST on blur-commit, reverts the input and shows the
   server's rejection text on a 4xx, and disables module-scoped fields
   with the caption "Shared across every instantiation of this module;
   read-only in this release." A short fix round (commit `b25eb6d`)
   added two pinning tests for already-correct behaviour (rejected-edit
   revert with exact message text; `programId: null` rendering) with no
   production change.

## Decisions that moved during implementation (vs. the design doc's plan)

- **D22: `display_order` became `Option<i64>`.** The design's draft had
  it as a plain `i64`. In practice not every package declares an
  explicit order (observed: all 543 rows for one demo program's
  `M-0083_A-0317-31-7DC6` application omit it), so the field had to be
  optional; `None` means "the package declared no order," and the
  panel falls back to declaration order for those rows.
- **D24: the undo/redo payload is `Option<String>`, not `Override<T>`.**
  The design sketch initially reached for the existing `Override<T>`
  enum (used elsewhere in `knx-core` for tri-state override/inherit/
  absent fields), but `Override<T>` has no "value present but this is a
  restore-to-previous-raw-or-none" variant that fits `RestoreParameterValue`
  cleanly, and `ParameterInstance.raw` itself is a plain `String`, not
  an `Override<String>`. Settled on `raw: String` for `SetParameterValue`
  (a set always supplies a value) and `raw: Option<String>` for
  `RestoreParameterValue` (undo may restore to "no stored instance
  existed before this edit," i.e. `None`).
- **D22 gained `access`.** Not in the original read-model sketch; added
  so the panel can display ETS's declared access level (read-only /
  read-write) for operator context. Display only — `access` does not
  gate whether the write endpoint accepts a POST; the write gate is
  purely "is this id top-level, not module-scoped" (D25).

## What stayed out of scope (unchanged from the design's Non-goals)

- **Module-scoped (per-channel) parameter editing.** `ParameterInstance`
  is still keyed by `(device, ets_id)` only; a module-instantiation's
  mangled id (`<Module/@Id>_MI-<k>_<declared suffix>`) is a distinct key
  per instantiation, but the evaluator's `ValueMap` is a flat
  `HashMap<String, String>` keyed by the *declared* id with no scope
  slot (`evaluate.rs:797`, `:348`), and MI-above-1 semantics are
  unresearched. Module-scoped values are read and displayed correctly
  (per-instantiation, via `ModuleScope`) but the write path only accepts
  top-level ids. This is D25, unchanged from the design.
- **Deep format validation.** The write path checks kind/enum/min/max
  shape, not full ETS semantic rules.
- **`evaluate`/`resolve_values`** were not touched; this slice is a
  consumer only. D16 (module-instantiation-shared-values limitation,
  from slice 2) stays true in the sense the docs pass restates: a
  `choose` controlled by a module-scoped parameter evaluates against the
  same program-default value in every channel, so the active field set
  can differ from ETS even where the *displayed* stored value is
  correct per-instantiation.
- No schema change in either database, in either direction.

## Task 5 (this entry's task) — docs reconciliation

Docs-only pass, no production logic change: `docs/IMPLEMENTATION_STATUS.md`,
`docs/KNOWN_LIMITATIONS.md` (§3, §12), `docs/GAP_ANALYSIS_ETS.md` (A3, B8,
Tier 5 T18), `docs/DATA_MODEL.md` (§10), `docs/ARCHITECTURE.md` (v1-target
table, one row), `docs/RESEARCH.md` (§4.4 addendum, id-shape format facts
re-derived from the three demo projects and confirmed against the design
doc's own Evidence section). Full detail in the task's own report at
`/home/knxbench/.claude/jobs/8098e9e6/tmp/t18s3-task5-report.md`.
