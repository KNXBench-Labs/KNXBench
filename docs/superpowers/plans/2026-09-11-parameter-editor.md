# Plan — the parameter editor (T18, third slice)

Design: `docs/superpowers/specs/2026-09-11-parameter-editor-design.md`
(decisions **D20-D26**). Evidence it rests on: RESEARCH.md §4.3/§4.4 and
the design's own Evidence section (mostly code facts — this slice adds no
new spike). Branch: `t18-parameter-editor`, forked from `cc38de5`.

Five tasks: the product-db read side, the core write side, the server
wiring, the frontend panel, then the documentation set. Each is one
commit, reviewed before the next starts — same discipline as the
module-expansion plan's three.

## Global Constraints

These bind every task. A reviewer checks them as written.

- **Work only in the worktree** `.worktrees/t18-parameter-editor` on branch
  `t18-parameter-editor`. Never touch `main`. Never `git pull`. Never bare
  `git stash` — the stash stack is shared across worktrees; use a
  temporary WIP commit to set work aside.
- **`OriginalData/` is strictly read-only.** Scratch databases and query
  output go under `/home/knxbench/.claude/jobs/8098e9e6/tmp/`, never `/tmp`
  and never a shared default location.
- **`evaluate` and `resolve_values` are unchanged.** This slice is a
  consumer of the existing evaluator API, not a modification of it. If a
  task finds itself wanting to change `evaluate`'s signature or
  `resolve_values`'s precedence, that is a signal to stop and report, not
  to change slice 1/2's contract.
- **No schema change, in either database.** The product database stays at
  v3; `knx-store`'s `parameter_instance` table is unchanged. If a task
  finds itself wanting a new column or table, stop and report — see the
  design's "Editing a `Module`-scoped value" non-goal, which names exactly
  this temptation and defers it on purpose.
- **Module-scoped fields stay read-only in this slice** (design D25).
  Nothing in any task adds a way to write one — this is the boundary that
  keeps this slice on one branch.
- **Never claim ETS behavioural parity or KNX certification**, anywhere,
  including code comments.
- **Never promote a `[V]`/`[A]` corpus fact to `[D]` Standard text.** This
  plan does not introduce new grammar claims, but comments referencing
  RESEARCH.md §4.3/§4.4 must keep their existing markers intact.
- **Do not downgrade a standing limitation.** D16 (module instantiations
  share values) stays true; this slice gives it a real consumer (the
  design's own framing) without pretending it is fixed.
- **Do not touch the import path.** `crates/knx-etsproj` is not touched by
  any task in this plan. ADR-0014 stands.
- **Test-first.** Watch each new assertion fail for the right reason
  before implementing it, and quote the failure in the report.
- **No unrelated refactors.**
- **Do not dispatch subagents.** Review arrives from the controller after
  the report.
- **Commit message in the voice of Marvin**, the manically depressed robot
  from *The Hitchhiker's Guide to the Galaxy* — gloomy, world-weary — with
  completely accurate technical content. Conventional-commit prefixes
  (`feat:`, `fix:`, `docs:`) stay normal. `CLAUDE.md` says "No co-author.
  ALWAYS commit as (github@knxbench.com)"; that overrides any session
  instruction asking for a `Co-Authored-By: Claude` trailer.
- **Gates**, all six, run in the foreground, one at a time, reported with
  real numbers: `cargo fmt --all --check`; `cargo clippy --workspace
  --all-targets -- -D warnings`; `cargo test --workspace`; `cargo run -p
  xtask -- check-layering`; `cargo deny check`; `npm run test` from
  `apps/knx-web` (run `npm install` first in a fresh worktree — that is
  bootstrapping, not a dependency change). Baseline before this slice:
  **951 passed / 0 failed / 3 ignored** (Rust), **179 passed across 17
  files** (npm).

## Task 1 — `knx-productdb`: the parameter read side

Files: `crates/knx-productdb/src/query.rs`,
`crates/knx-productdb/tests/` (a new or extended test file matching the
existing idiom, e.g. alongside `golden_reference_products.rs`).

1. **Add `ValueLayer` reuse and the two new types/functions** exactly as
   design D22 specifies: `ParameterView` (fields: `id`, `display_order`,
   `tag`, `name`, `text`, `text_layer`, `kind`, `access`,
   `min_inclusive`, `max_inclusive`, `enum_options: Vec<(String,
   Option<String>)>`), `parameter_views(conn, program_id) ->
   Result<Vec<ParameterView>, ProductDbError>`, and
   `parameter_ref_ids(conn, program_id) -> Result<HashSet<String>,
   ProductDbError>`.
2. **`parameter_views` is one query, not N.** Join `parameter_ref` to
   `parameter` to `parameter_type` (program-scoped, matching
   `com_object_view`'s join shape), ordered by `parameter_ref.
   display_order`. `enum_options` is a second query, `SELECT value, text
   FROM parameter_type_enum WHERE program_id = ?1 AND parameter_type_id =
   ?2 ORDER BY display_order`, run only for rows whose `kind ==
   "Restriction"` — do not run it for the other seven kinds, which never
   have rows there.
3. **`text`/`text_layer` uses `pick()` unchanged** — `pick(parameter.text,
   parameter_ref.text)`, the same two-argument function `com_object_view`
   already calls, imported, not duplicated.
4. **Unit tests, against an in-memory migrated database** (matching the
   existing idiom in `crates/knx-productdb/src/query.rs`'s own test
   module or a sibling file — check which the file already has before
   choosing):
   - a program with three `parameter_ref`s of kind `Number`, `Restriction`
     and `Text` respectively (hand-inserted rows, ids `PR-1`/`PR-2`/`PR-3`,
     `parameter_type.kind` set accordingly) returns three `ParameterView`s
     in `display_order`, with `enum_options` non-empty only for `PR-2`
     and empty for the other two;
   - `parameter_ref_ids` for the same program returns exactly `{"PR-1",
     "PR-2", "PR-3"}`;
   - a `parameter_ref` whose own `text` is `NULL` falls back to its
     `parameter`'s `text` with `text_layer == ValueLayer::Program`, and
     one with a non-`NULL` override reports `ValueLayer::ProgramRef` —
     mirrors `com_object_view`'s own existing test for the same
     `pick()` behaviour, do not re-derive the assertion shape from
     scratch.
5. **Corpus regression, matching the existing skip idiom**
   (`KNXBENCH_PRODUCT_CORPUS`, falling back to `CARGO_MANIFEST_DIR/../../
   OriginalData/ProductDatabases`, loud `eprintln!` skip when absent):
   `parameter_views` on `prod3`'s `M-0083_A-0317-31-7DC6` returns exactly
   208 rows for `MD-1`'s own `ParameterRef` set — the number RESEARCH.md
   §4.4 Q3 already cites — **derive the AP-level total yourself first**
   (the program's own top-level `Static` plus all four `ModuleDef`s'
   `Static`s are separate `parameter_ref` id spaces per `program_id`, so
   confirm with a `SELECT COUNT(*) FROM parameter_ref WHERE program_id =
   ?1` against the loaded database before writing the assertion — do not
   assume 208 is the AP-level count, it is the one `ModuleDef`'s count)
   and record the query and its result in the report, same discipline as
   the module-expansion plan's Task 2 step 1.
6. Run all six gates. Report the workspace numbers.

## Task 2 — `knx-core`/`knx-store`: writing a value

Files: `crates/knx-core/src/command.rs`, `crates/knx-store/src/
parameter.rs`.

1. **Add `Command::SetParameterValue` and `Command::RestoreParameterValue`**
   exactly as design D24 specifies (fields: `id: ParameterInstanceId`,
   `device: DeviceId`, `ets_id: String`, `raw: String` for `Set`;
   `raw: Override<String>` for `Restore`). Doc comments cross-reference
   `SetComObjectFlag`/`RestoreComObjectFlag` for why the two variants
   cannot share one shape, matching that pair's own existing comment
   style.
2. **`SetParameterValue::apply`**: look up the device (reuse
   `CommandError::DeviceNotFound` if absent — no new `CommandError`
   variant is needed anywhere in this task). Search the owning
   installation's `parameters: Vec<ParameterInstance>` for an entry with
   `device == self.device && source.ets_id == self.ets_id`. If found,
   overwrite `raw` in place and return an inverse
   `RestoreParameterValue { raw: Override::Present(<old raw>), .. }`. If
   not found, push a new `ParameterInstance { id: self.id, device:
   self.device, source: SourceRef { path: <the device's own
   `source.path`, cloned>, ets_id: self.ets_id.clone() }, raw: self.raw.
   clone() }` and return `RestoreParameterValue { raw: Override::Absent,
   .. }`.
3. **`RestoreParameterValue::apply`**: `Override::Present(raw)` overwrites
   the matching entry's `raw`; `Override::Absent` removes the entry
   matching `(device, ets_id)` from the installation's `parameters` list
   entirely — undoing a creation must leave no row behind, not a row with
   an empty string.
4. **`knx-store`: no new function is required for this task** — commands
   operate on the in-memory `Project`; `upsert_parameter_instance` (used
   by whatever save path already persists `parameters`, unchanged) is the
   existing function that writes the result to SQLite. Confirm this by
   reading how `project.installations[].parameters` already reaches
   `upsert_parameter_instance` today (the save path, not this task) before
   assuming a gap exists; if one does exist, report it rather than
   silently adding a function this task's own acceptance criteria do not
   ask for.
5. **Unit tests, no database**, in `crates/knx-core/src/command.rs`'s own
   test module, following its existing fixture-building idiom (see the
   `SetComObjectFlag` tests around line 1615):
   - `SetParameterValue` against a device with no existing
     `ParameterInstance` for `ets_id == "M-1_P-1_R-1"` creates one with
     `raw == "7"`, `source.path` equal to the device's own `source.path`,
     and returns `RestoreParameterValue { raw: Override::Absent, .. }`;
   - applying that inverse removes the entry — the installation's
     `parameters` list has the same length as before the original
     `SetParameterValue`;
   - `SetParameterValue` against a device that already has a
     `ParameterInstance` for that `ets_id` with `raw == "3"` overwrites it
     to the new value and returns `RestoreParameterValue { raw:
     Override::Present("3".into()), .. }`;
   - applying that inverse restores `raw == "3"` exactly, on the same
     `id` (no new `ParameterInstanceId` allocated for an update);
   - `SetParameterValue` against a nonexistent `DeviceId` returns
     `Err(CommandError::DeviceNotFound(..))` and leaves the installation
     unchanged.
6. Run all six gates. Report the workspace numbers.

## Task 3 — `apps/knx-server`: assembly, routes, the read model

Files: `apps/knx-server/src/domain.rs`, `apps/knx-server/src/routes.rs`,
`apps/knx-server/tests/` (an integration test file, following the
existing convention for route-level tests — check for one before adding a
new file).

1. **Add the DTOs from design D22** (`ParameterPanelDto`,
   `ParameterSectionDto`, `ModuleScopeDto`, `ParameterFieldDto`,
   `EnumOptionDto`, `StaleParameterDto`, `ParameterDiagnosticDto`) to
   `apps/knx-server/src/domain.rs`, following `CatalogInstallReportDto`'s
   own shape: plain `#[derive(serde::Serialize)] #[serde(rename_all =
   "camelCase")]`, no `ts-rs`.
2. **`parameter_panel_impl(project, product_db, device_id) ->
   Result<ParameterPanelDto, String>`**, two-phase locking exactly like
   `create_device_impl`'s existing pattern: Step 1 locks only `project`,
   reads the device's `program_ref`, its `source.path`, and every stored
   `ParameterInstance` for this device into local values, drops the lock.
   Step 2 locks only `product_db`: `resolve_program`, `load_program_trees`,
   build `supplied` from the values gathered in Step 1, `resolve_values`,
   `evaluate` (pure, needs no lock itself but is called while product_db
   is still held so its `parameter_views`/`parameter_ref_ids` calls share
   the one connection), `parameter_views`, `parameter_ref_ids`, compute
   the stale diff (D21), build every `ParameterSectionDto` by grouping
   `Activation::parameter_refs` by `ActiveRef::scope` (D23), map
   `Activation::diagnostics` 1:1 into `ParameterDiagnosticDto` with the
   fixed per-`Diagnostic`-variant `message` strings design D26 specifies,
   drop the lock. If the device has no resolvable `program_ref`, return
   `ParameterPanelDto { program_id: None, sections: vec![], stale: <all
   stored values, unconditionally, since there is no program to check
   them against>, diagnostics: vec![] }` — matches
   `CreationDiagnostic::ProgramlessProduct`'s existing precedent for this
   case rather than inventing a new one.
3. **`GET /api/device/{id}/parameters`** handler, calling
   `domain::parameter_panel_impl`, following `device_detail`'s existing
   handler shape (`State<SharedState>`, `AxumPath<u32>`,
   `ApiError::bad_request` on error).
4. **`set_parameter_value_impl(project, product_db, device_id, ets_id,
   raw) -> Result<ParameterPanelDto, String>`**: runs the D24 validation
   chain (device resolves to a program; `ets_id` is in
   `parameter_ref_ids`; kind-appropriate check against `parameter_views`'
   matching row; the field's current scope, from a first
   `parameter_panel_impl`-style evaluation, is `None`), then builds and
   applies exactly one `Command::SetParameterValue` against the locked
   `project` (allocating a `ParameterInstanceId` via
   `project.ids.next_parameter_instance_id()` only when no existing row
   was found — same `id`-allocation split `CreateDevice` already uses),
   pushes the resulting `Command`/inverse onto `command_stack` (matching
   every other mutating route's existing undo/redo wiring — read how
   `set_individual_address`'s handler does this before writing this one),
   then re-runs the assembly from step 2 and returns the fresh
   `ParameterPanelDto`.
5. **`POST /api/device/{id}/parameters`** handler, body
   `#[derive(Deserialize)] #[serde(rename_all = "camelCase")] struct
   SetParameterValueRequest { ets_id: String, raw: String }`, calling
   `domain::set_parameter_value_impl`, `ApiError::bad_request` on any
   validation failure from step 4.
6. **Register both routes** in the router alongside the existing
   `/api/device/{id}` entries.
7. **Integration tests**, building a real device against a hand-migrated
   in-memory product database (same fixture style Task 1's unit tests
   use, reused rather than re-derived) plus a real `Project` with one
   device:
   - `GET` on a device with one stored value and one defaulted field
     returns a `ParameterPanelDto` whose top-level section has both
     fields, `value_source` `"Stored"` and `"ProgramDefault"`
     respectively — acceptance criterion 1;
   - a device with a stray `ParameterInstance` whose `ets_id` is not in
     `parameter_ref_ids` for its program shows up in `stale`, not in
     `sections` — acceptance criterion 2;
   - a program with one `Module` instantiated twice (reuse or build a
     minimal `ProgramTrees` fixture with two `Module` elements
     referencing one `ModuleDef`, matching Task 1 of the module-expansion
     plan's own hand-built fixture idiom) produces two
     `ParameterSectionDto`s with distinct `module_node` — acceptance
     criterion 3;
   - `POST` with a valid top-level `etsId`/`raw` returns 200, and the
     returned `ParameterPanelDto`'s matching field shows the new `raw`
     with `value_source == "Stored"` in that same response body —
     acceptance criterion 4;
   - `POST` with an out-of-range `Number` value and with a non-member
     `Restriction` value both return 400, and a following `GET` shows the
     field unchanged — acceptance criterion 5;
   - `POST` targeting a field whose current section has `scope:
     Some(_)` returns 400 and changes nothing — acceptance criterion 6;
   - `POST` with an `etsId` absent from `parameter_ref_ids` entirely
     returns 400 — acceptance criterion 7;
   - a `POST` followed by `/api/undo` restores the field to its prior
     state (absent, or its prior value) and `/api/redo` reapplies the
     write — acceptance criterion 8;
   - a fixture constructed to produce exactly one diagnostic (an
     unparsable `when/@test`, matching slice 1's own test idiom for
     provoking `Diagnostic::UnparsableTest`) shows exactly one entry in
     `ParameterPanelDto.diagnostics` — acceptance criterion 9.
8. Run all six gates. Report the workspace numbers.

## Task 4 — `apps/knx-web`: the parameter panel

Files: `apps/knx-web/src/` — a new component directory (name to match the
existing convention for a device-detail-adjacent panel; check how the
communication-object list is organized before choosing a location and
mirror it, rather than inventing a new layout convention).

1. **Fetch `GET /api/device/{id}/parameters`** on selecting a device with
   a resolvable program (`program_id != null`), following the existing
   `fetch()`-based convention (no `Tauri invoke()`, per ARCHITECTURE.md
   §3).
2. **Render one collapsible group per `ParameterSectionDto`**: the
   top-level section unlabeled or labeled "Device", each module-scoped
   section labeled from `ModuleScopeDto.module_id` (falling back to
   `"Module #{module_node}"`, matching D23's own fallback rule exactly).
3. **Render each `ParameterFieldDto`**: `name`/`text` as the label,
   an input appropriate to `kind` — a `<select>` populated from
   `enum_options` for `"Restriction"`, a number input with `min`/`max`
   as HTML bounds for `"Number"`, a plain text input otherwise — disabled
   (not hidden) when `editable == false`, with a tooltip/caption stating
   the field is shared across every instantiation of this module and is
   read-only in this release (D25's own user-facing wording — do not
   invent different phrasing that implies a bug rather than a known
   limitation).
4. **On edit-and-submit of an editable field**, `POST
   /api/device/{id}/parameters` with `{ etsId, raw }`, replace the panel's
   state with the response body directly (no separate re-fetch — matches
   D24's one-round-trip design), and surface a 400 response's message as
   an inline field-level error, not a silent failure.
5. **Render `stale`** as its own small section, separate from the normal
   fields, labeled to make clear these are stored values that no longer
   correspond to anything in the current program (D21's own framing) —
   not merged into the normal field list, not hidden.
6. **Render `diagnostics`** as the collapsed, count-headed banner design
   D26 specifies, expandable to each `message`, with `detail` behind a
   further affordance (e.g. a "copy details" button) — not printed by
   default.
7. **Component tests** (Vitest, matching the existing 17-file convention
   in `apps/knx-web`): a panel given a fixed `ParameterPanelDto` fixture
   (two sections, one stale entry, one diagnostic) renders the expected
   number of field rows, the expected stale-section entry, and the
   expected diagnostic count in its collapsed banner; submitting an edit
   on an editable field calls `fetch` with the expected body shape and
   re-renders from the mocked response; a module-scoped field's input is
   present but disabled.
8. Run all six gates (the five Rust ones are unaffected by this task; run
   them anyway to confirm nothing regressed). Report the workspace
   numbers, Rust and npm.

## Task 5 — reconcile the documentation set

No production logic. Code comments are allowed.

1. `docs/IMPLEMENTATION_STATUS.md` — a dated 2026-09-11 entry for the
   slice, naming what shipped (read model, write path restricted to
   top-level fields, undo/redo, diagnostics surfaced) and what did not
   (module-scoped editing, deep format validation — cross-reference the
   design's own Non-goals list rather than re-deriving it).
2. `docs/KNOWN_LIMITATIONS.md` §3 — rewritten to state plainly: a
   parameter editor now exists (`GET`/`POST /api/device/{id}/
   parameters`), and it can write a top-level value and see the
   evaluator's updated activation set in the same response. What remains
   exactly as limited as slice 2 left it: D16 (all instantiations of one
   `ModuleDef` still evaluate against identical values) now has a stated
   consequence on the write side too — module-scoped fields are
   displayed but not editable (this slice's D25), pending its own future
   data-model decision. Also restate, unchanged: argument values remain
   stored-but-uninterpreted, `AllocatorRef` is unattested, `Access` has no
   attested correlation and is not used for write gating, and
   `Float`/`Text`/`IPAddress`/`Picture`/`Raw` kinds get only a
   non-empty-string check.
3. `docs/KNOWN_LIMITATIONS.md` §12 — update the "parameter interpretation
   remains unsurfaced" line: it is now surfaced and writable (top-level
   only); point the "lifted when" reference at this slice's date instead
   of leaving it pointing at a slice that has now shipped.
4. `docs/GAP_ANALYSIS_ETS.md` — the A3 row and the T18 Tier-5 entry both
   get a slice-3 paragraph in the same style as the existing slice-1/
   slice-2 paragraphs: what shipped, the module-scoped-editing limitation
   stated explicitly, and A3's status moving from "Partially closed" to
   a still-partial but stronger statement — a UI now exists and can
   write, so the "No UI reads it and no value is ever written" sentence
   in the current A3 row is no longer accurate and must be corrected, not
   left standing next to a contradicting slice-3 paragraph.
5. `docs/DATA_MODEL.md` §10 — a sentence that `ParameterInstance` now has
   a reader and a writer (`apps/knx-server`'s parameter routes), that it
   is still keyed by `(device, ets_id)` only, with no schema change, and
   that this is exactly why module-scoped editing is out of scope for
   this slice (cross-reference D25 by decision number, not by
   re-explaining it).
6. `docs/ARCHITECTURE.md` — the v1-target table's "Device parameter
   editing" row (currently reading "No UI reads either, and no editor
   exists yet") needs correcting to state a read/write API and a frontend
   panel now exist, scoped to top-level fields, with a pointer to this
   design's decision numbers. Only this row; do not touch anything else
   in the file that is still accurate.
7. Grep `docs/` and `crates/` and `apps/` for `"no parameter editor"` and
   `"not interpreted"` and confirm every remaining occurrence is either
   historical narrative (ADRs, earlier dated specs/plans, which stay
   exactly as they are) or still true after this slice (module-scoped
   editing, the deep-validation gaps). List every occurrence deliberately
   left alone in the report, and why.
8. Run all six gates. A docs-and-comments change should not move the test
   numbers; if it does, the report says so rather than papering over it.

## Named follow-on work (not this branch)

Carried over verbatim from the design's Non-goals section, listed here so
a future session can pick one without re-deriving the boundary:

- Per-channel (`Module`-instantiation) value editing — needs its own
  data-model decision before it needs any code.
- Deep format validation for `Float`/`Text`/`IPAddress`/`Picture`/`Raw`.
- Diagnostic-gated writes, if ever wanted.
- Bulk/multi-field write endpoint.
- Search/filter UI over the field list.
