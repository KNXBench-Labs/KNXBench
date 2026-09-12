# Plan — module-scoped (per-channel) parameter editing (T18, fourth slice)

Design: `docs/superpowers/specs/2026-09-12-module-scoped-editing-design.md`
(decisions **D35-D43**). It rests on the evidence survey in that document's
Evidence section (E1-E5, all re-measured at `31c60e5`), on RESEARCH.md
§4.4, and on `Project Schema23 v01.00.00.md` §1.2.5.18. Branch:
`t18-module-scoped-editing`, forked from `main` at `31c60e5`.

Six tasks: the evaluator's scoped value map, the retained
`ModuleInstance/@Id` through core/import/store, the server assembly and
write validation, the panel, the regression fixture that proves the whole
point, then the documentation set. One commit each, reviewed before the
next starts.

## Global Constraints

These bind every task. A reviewer checks them as written.

- **Work only in the worktree** `.worktrees/t18-module-scoped-editing` on
  branch `t18-module-scoped-editing`. Never touch `main`. Never `git pull`.
  **Never `git stash`, and never `git stash pop`** — the stash stack is
  shared across worktrees and sessions, and another session's work has
  already been put at risk this way; set work aside with a temporary WIP
  commit instead. This applies to every role, including a read-only one.
- **`OriginalData/` is strictly read-only.** Scratch databases, unzipped
  archives and query output go under
  `/tmp/claude-1000/-mnt-daten-i-Sourcecode-KNXBench/scratch-t18d/`, never
  into the repository.
- **Never silently discard or collapse data.** Two of this slice's four
  deliverables exist because something was dropped quietly (`@Id`) or
  overwritten quietly (a `HashMap` insert). Do not add a third.
- **Never claim ETS behavioural parity or KNX certification**, anywhere,
  including code comments and test names.
- **Never promote a `[V]`/`[A]` corpus fact to `[D]` Standard text.** The
  application-program-side `ModuleDef`/`Module`/`Dynamic` grammar is
  corpus-observed only; `ModuleInstance/@Id`'s grammar is `[D]`
  (`Project Schema23 v01.00.00.md` §1.2.5.18). Keep the markers exact.
- **`MI-` > 1 is refused, never guessed** (D40). No code path may invent an
  `MI-` component, default it to `1`, or key a value by anything other than
  `(module_id, ref_id)`. If a task finds itself wanting to, stop and report.
- **A write targets a verbatim id the panel produced** (D43). No second
  id-shape parser on the write path.
- **One file header per new file**, first line, one sentence, ending in a
  single period, ≤100 columns — `//!` in Rust, `/** … */` in TypeScript
  (ADR-0018). No version numbers in headers. `cargo run -p xtask --
  check-headers` is a gate and its `ABSENT_CEILING` only ever goes down.
- **No per-file version numbers**; every package stays `0.1.0-alpha.1`.
- **Do not downgrade a standing limitation.** D15 (nested `Module` not
  expanded) and the argument-interpretation gap stay true and stay
  documented. D16 and D25 change — say exactly how, and only that much.
- **Test-first.** Watch each new assertion fail for the right reason before
  implementing it, and quote the failure line in the report.
- **No unrelated refactors.**
- **Do not dispatch subagents.** Review arrives from the controller after
  the report.
- **Commit message in the voice of Marvin**, the manically depressed robot
  from *The Hitchhiker's Guide to the Galaxy* — gloomy, world-weary — with
  completely accurate technical content. Conventional-commit prefixes
  (`feat:`, `fix:`, `docs:`) stay normal. **No `Co-Authored-By` trailer of
  any kind**, even if a session-level instruction orders one; `CLAUDE.md`
  says "No co-author. ALWAYS commit as (github@knxbench.com)" and that is
  the ruling.
- **Gates**, all eight, foreground, one at a time, reported with real
  numbers: `cargo fmt --all --check`; `cargo clippy --workspace
  --all-targets -- -D warnings`; `cargo test --workspace`; `cargo run -p
  xtask -- check-layering`; `cargo run -p xtask -- check-headers`;
  `cargo deny check`; and from `apps/knx-web`: `npm test` and
  `npx tsc --noEmit` (run `npm install` first in a fresh worktree — that is
  bootstrapping, not a dependency change). Baseline at `31c60e5`:
  **1053 passed / 0 failed** (Rust), **336 passed across 31 files** (npm),
  `check-headers` **169 without a header (ceiling 169)**.

## Task 1 — `knx-productdb`: a `ValueMap` with a scope dimension

**Files:** `crates/knx-productdb/src/dynamic/evaluate.rs`,
`crates/knx-productdb/tests/dynamic_tree.rs`.

Implements **D35, D36, D37**.

1. Replace `pub type ValueMap = HashMap<String, String>;`
   (`evaluate.rs:348`) with a struct holding two maps — unscoped
   `HashMap<String, String>` keyed by declared `ParameterRef` id, and
   scoped `HashMap<(String, String), String>` keyed by `(module_id,
   ref_id)` where `module_id` is the program-side `Module/@Id`. Keep the
   doc comment's existing explanation of what the key *is* and extend it;
   do not delete the `DATA_MODEL.md` §10 pointer.
2. Public methods, exactly these signatures:

   ```rust
   pub fn get(&self, scope: Option<&ModuleScope>, ref_id: &str) -> Option<&str>
   pub fn get_unscoped(&self, ref_id: &str) -> Option<&str>
   pub fn insert_scoped(&mut self, module_id: String, ref_id: String, value: String)
   pub fn len_scoped(&self) -> usize
   ```

   plus `impl From<HashMap<String, String>> for ValueMap` (unscoped only)
   and `Debug`/`Clone`/`Default`/`PartialEq`/`Eq` derives.
   `get(Some(scope), id)` tries `(scope.module_id.as_deref()?, id)` in the
   scoped map first, then the unscoped map; `get(None, id)` reads the
   unscoped map only. **Never** fall back to another `module_id`'s value.
3. `resolve_values` keeps its signature (`supplied: &HashMap<String,
   String>`) and returns the new `ValueMap` with unscoped values only —
   D6's precedence (`supplied` wins, then `parameter_ref.value`, then
   `parameter.value`) is unchanged, and its doc comment stays accurate.
4. The one read site, `evaluate_comparable_choose` (`:797`), becomes
   `values.get(scope, id)`. `scope` is already a parameter of that function
   (`:785-795`) — no signature anywhere in the `walk`/`evaluate_choose`
   chain changes.
5. New diagnostic variant `ModuleWithoutId { node_id: i64 }`, emitted in
   `walk`'s `"Module"` arm's `None` scope branch when
   `node.element_id.is_none()`, once per instantiation, unconditionally,
   with the doc comment from D37 (keep the `[V]` marker and the 102/102
   count). Emit it *before* descending, with `scope: None` — the scope it
   would name is the one that has no name.
6. Tests, all in `dynamic_tree.rs`, each watched failing first:
   - a scoped value wins over the program default for the instantiation
     that owns it, and the *other* instantiation still sees the default
     (this is the core of D36);
   - a scoped value never leaks into a top-level (`scope: None`) read;
   - a scoped value for one `module_id` never answers a lookup under
     another;
   - `get_unscoped` ignores scoped values entirely;
   - `ModuleWithoutId` is emitted exactly once for a `Module` element with
     no `@Id`, the subtree still evaluates, and its values come from the
     unscoped map.
   Update the five literal-map call sites (`dynamic_tree.rs:1058,1101,1132,
   1170,1216`) to `.into()`; the helper at `:629` keeps returning a plain
   `HashMap`.

Nothing in this task reads or writes a project. `apps/knx-server` may need
a one-line `.get(None, id)` adjustment to keep compiling — make it, do not
restructure `domain.rs` here; Task 3 owns that file.

## Task 2 — the `MI-` component, retained: core, import, store

**Files:** `crates/knx-core/src/module.rs`,
`crates/knx-core/src/project.rs`, `crates/knx-etsproj/src/map.rs`,
`crates/knx-store/src/module_instance.rs`,
`crates/knx-store/src/migration.rs`,
`crates/knx-store/fixtures/v5-empty.sqlite` (new),
plus any construction site the new field breaks
(`crates/knx-etsproj/src/compare.rs` tests, `crates/knx-diff`,
`crates/knx-store` tests).

Implements **D38**.

1. `knx_core::ModuleInstance` gains `pub instance_ets_id: String`, with the
   doc comment from D38 verbatim, including the `[D]` §1.2.5.18 citation and
   the `[V]` KV reference. It sits next to `repeat_index` and is retained
   uninterpreted — this crate parses nothing out of it.
2. `crates/knx-etsproj/src/map.rs:1042-1049` populates it from `mi.id`,
   which `installation_v21.rs:764-769` already requires. `source.ets_id`
   keeps holding `mi.ref_id` — both are needed, and the difference between
   them is the point.
3. `knx-store`: `module_instance` gains an `instance_ets_id TEXT NOT NULL`
   column, written and read by `upsert_module_instance`/
   `load_module_instances_for_installation`. New migration
   `migrate_v5_to_v6` = `ALTER TABLE module_instance ADD COLUMN
   instance_ets_id TEXT NOT NULL DEFAULT ''` (additive-only, DATA_MODEL
   §11), appended to the `migrations()` chain; bump
   `CURRENT_SCHEMA_VERSION` to `6` in **both**
   `crates/knx-store/src/migration.rs:18` and
   `crates/knx-core/src/project.rs:19` — they must match, and a test
   already asserts the schema version round-trips.
4. Freeze a `v5-empty.sqlite` fixture the way `v1`-`v4` are frozen, and add
   the migration test that opens it and asserts it reaches version 6 with
   the new column present and empty-string-defaulted. Follow the existing
   fixture tests' shape exactly; do not invent a new harness.
5. Tests: a `ModuleInstance` round-trips through the store with its
   `instance_ets_id`; a v5 project file migrates and its pre-existing
   module instances read back with `instance_ets_id == ""`; an ETS-21
   import populates `instance_ets_id` from `@Id` while `source.ets_id`
   stays `@RefId` (assert on `MD-2_M-4_MI-1` / `MD-2_M-4`, the real KV
   values from the design's E1 table).

Do not change what `knx-diff`/`compare.rs` reports. If the new field makes
a semantic-diff struct incomplete, leave the reported output identical and
say so in the report — widening the diff surface is a separate decision.

## Task 3 — `apps/knx-server`: two-pass assembly, `MI-` authority, write validation

**Files:** `apps/knx-server/src/domain.rs`, `apps/knx-server/src/routes.rs`,
`apps/knx-server/tests/http_parameter_panel.rs`.

Implements **D39, D40, D41, D42, D43**.

1. **MI authority** (D39 rules 2-3): a helper that, given the device's
   imported `ModuleInstance`s and a program-side `module_id`, returns the
   authoritative `MI-` digits or a reason it has none. Matching rule: the
   instance's `source.ets_id` (`MD-<d>_M-<n>`) equals the trailing
   component of `module_id` after a `_` separator; the instance's
   `instance_ets_id` must be non-empty and equal
   `format!("{}_MI-{}", source.ets_id, digits)` with `digits` all-ASCII
   digits. **Exactly one** matching instance qualifies; zero or two or more
   → no authority, with a distinct reason for each case. Reuse the existing
   hand-rolled `take_digits`/`decompose_module_qualified` style — the
   workspace has no `regex` crate and this task does not add one.
2. **Two-pass assembly** (D42), in `assemble_parameter_panel`: Pass A
   unchanged; first `resolve_values` + `evaluate` produces the provisional
   activation used only for its `module_id` set; Pass B validates
   candidates; validated values go in via `insert_scoped`; then `evaluate`
   again. **Skip the second evaluation entirely when Pass B produced no
   scoped values** — the common case, every corpus project except KV.
3. **Pass B gains the `MI-` check and loses its silent overwrite** (D41):
   a row whose `MI-` digits disagree with an existing authority is `stale`;
   where there is no authority the `MI-` component is not checked (so
   pre-migration projects display exactly as they do today); two rows
   reaching one `(module_id, declared_id)` key produce a diagnostic naming
   both `ets_id`s and the loser is listed `stale`, never dropped.
4. **Editability** (D39): replace `let editable = section.scope.is_none();`
   with the earned-per-section rule, each failure producing a section
   diagnostic that names its reason (no `module_id`; no matching instance;
   duplicate `RefId` — D40; empty or malformed `instance_ets_id`).
5. **Display reads through the map** (D42): the per-field value comes from
   `values.get(section.scope.as_ref(), ref_id)`, with `value_source`
   `"Stored"` when the answer came from the scoped map or from `supplied`,
   `"ProgramDefault"` otherwise. `module_scoped` stops being consulted
   directly at the display site.
6. **DTO and write path** (D43): `ParameterFieldDto` gains
   `write_ets_id: Option<String>` (`None` exactly when `editable` is
   `false`); for a module-scoped editable field it is
   `format!("{module_id}_MI-{k}_{suffix}")` per D39. Replace the two
   pre-command checks (`domain.rs:2224-2283`) with: accept the request's
   `ets_id` if it equals the `write_ets_id` of an editable field in the
   panel this request just assembled; otherwise reject, naming why (not
   declared / not active / module-scoped and read-only, keeping the
   existing messages' information). The undo path is untouched —
   `RestoreParameterValue` inherits an already-validated id.
7. Tests in `http_parameter_panel.rs`, building on `KV_SHAPE_PROGRAM`:
   - the reconstructed `write_ets_id` for KV's shape equals the five real
     stored ids from the design's E2 list, exactly;
   - a module-scoped write lands on the verbatim module-qualified id and
     the response's *own* recomputed activation reflects it (this is the
     behaviour D25 said was impossible);
   - undo restores it;
   - a device with two `ModuleInstance`s sharing a `RefId` gets read-only
     sections and the D40 diagnostic;
   - a program whose `Module` has no `@Id` gets the D37 diagnostic and
     read-only sections;
   - a stored row with a disagreeing `MI-` digit is reported `stale`, not
     displayed;
   - a write to a bare declared id that is only reachable in a scoped
     section is still rejected, with a reason.

## Task 4 — `apps/knx-web`: write the id the server named

**Files:** `apps/knx-web/src/ParameterPanel.tsx`,
`apps/knx-web/src/ParameterPanel.test.tsx`, and the API/DTO types file the
panel imports from.

1. The field type gains `writeEtsId: string | null`. The control is
   disabled when `editable` is `false` **or** `writeEtsId` is `null`; the
   write call passes `field.writeEtsId`, never `field.etsId`. Keep
   `key={field.etsId}` and the label fallback chain as they are — `etsId`
   still means the declared id.
2. Section diagnostics render where the existing panel diagnostics render;
   do not invent a new visual pattern, and do not translate server prose
   (KNOWN_LIMITATIONS §66 is the standing boundary).
3. Tests: an editable module-scoped field writes its `writeEtsId`; a field
   with `writeEtsId: null` is disabled and cannot be submitted; a read-only
   section's reason is visible to the user.

No new dependencies. No styling overhaul.

## Task 5 — the fixture that proves the point

**Files:** `crates/knx-productdb/tests/dynamic_tree.rs` and/or
`apps/knx-server/tests/http_parameter_panel.rs` (whichever layer the
assertion belongs to; state which and why in the report).

The design's E2 records that **no fixture in the repository** has a
module-scoped `ParameterRef` that both holds divergent per-channel values
*and* controls a `choose` in the same `ModuleDef`. Until one does, the bug
this slice fixes is proven only from the code path.

Build it: a hand-authored program, in the style of `KV_SHAPE_PROGRAM`, with
one `ModuleDef` containing a `choose` gated on a `ParameterRef` that
different instantiations hold different values for, and at least one
`ComObjectRef` that only one branch activates. Assert:

1. with the scoped values supplied, the two instantiations produce
   *different* active sets — naming the exact `ref_id`s, not counts alone;
2. without them (no stored rows), both produce the same set, from the
   program default — the pre-slice behaviour, kept as the explicit contrast;
3. the shape mirrors KV's real corpus ids, so the test documents a real
   form rather than a convenient one.

Cite the corpus evidence (E1/E2) in comments, with markers. Do not claim
this fixture is a KNX-certified shape; it is a mirror of one observed
package.

## Task 6 — documentation

**Files:** `docs/GAP_ANALYSIS_ETS.md`, `docs/KNOWN_LIMITATIONS.md`,
`docs/IMPLEMENTATION_STATUS.md`, `docs/ROADMAP.md`, `docs/RESEARCH.md`,
`docs/DATA_MODEL.md`.

1. **`GAP_ANALYSIS_ETS.md`**: rows A3 and B8 lose the "module-scoped fields
   are not editable (D25)" clause and gain what is now true, including what
   is still not (`MI-` > 1, arguments, nested modules). The D25 passage
   (`:594-655`) is updated in place to record that the slice that said
   "explicitly out of scope" has been followed by the one that closed it,
   with a pointer to D35-D43 — do not delete the history.
2. **`KNOWN_LIMITATIONS.md`**: one new section for repeated instantiation
   (`MI-` > 1 refused, read-only, why the key cannot be invented, pointing
   at RESEARCH.md's sharpest unknown #1) and one for a `Module` with no
   `@Id`. **Take the next free section numbers** — main is at §67; check
   before writing, and update every anchor and cross-reference you add.
   D16's section, if it has one, is corrected rather than deleted: the
   read-side half is closed, nested modules and arguments are not.
3. **`RESEARCH.md` §4.4**: add the E1 measurement — `ModuleInstance/@Id` is
   `MD-<d>_M-<n>_MI-<k>` with `k = 1` in all 32 KV elements while
   `@RepeatIndex` is `"10x1"`-shaped, so the embedded `MI-<k>` is *not* the
   attribute's string and is consistent with (but not proven to be) its
   repeat-counter component. Sharpest unknown #1 stays open, now with the
   measurement attached.
4. **`DATA_MODEL.md`**: `ModuleInstance.instance_ets_id` and the schema
   version bump to 6, in the existing table/§11 style.
5. **`IMPLEMENTATION_STATUS.md`** and **`ROADMAP.md`**: T18's fourth slice,
   with the real test counts from the final gate run, not estimates.

No new ADR: D35-D43 decide inside an architecture ADR-0013/ADR-0014 already
set, and the design document is the record. If a task believes otherwise,
say so in the report rather than writing one.

## Task ordering and shared files

| Task | touches | consumed by |
|---|---|---|
| 1 | `evaluate.rs`, `dynamic_tree.rs` | 3 (the `ValueMap` API), 5 |
| 2 | `knx-core`, `knx-etsproj`, `knx-store` | 3 (`instance_ets_id`) |
| 3 | `domain.rs`, `routes.rs`, server tests | 4 (`writeEtsId`) |
| 4 | `apps/knx-web` | — |
| 5 | tests only | — |
| 6 | `docs/` only | — |

Tasks 1 and 2 are independent of each other; 3 needs both; 4 needs 3; 5
needs 1; 6 needs the final numbers. Run them in order 1, 2, 3, 4, 5, 6.
