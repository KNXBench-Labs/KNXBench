# `Dynamic` Tree Parse and Evaluate Implementation Plan (T18, first slice)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Store the `ApplicationProgram/Dynamic` tree in the product database
and evaluate it headlessly into the active parameter-ref and
communication-object-ref sets, with every unknown, unmatched or unexpanded
construct reported rather than hidden.

**Architecture:** `knx-productdb` gains a `dynamic` module with a parser half
(feeding a new `dynamic_node` table at schema v3, backfilled from stored
blobs) and an evaluator half (a pure function of tree plus value map). No
other crate is touched by tasks 1 and 2; no UI, no HTTP surface, no project
storage change.

**Tech Stack:** Rust, `rusqlite`, `quick-xml`.

**Spec:** `docs/superpowers/specs/2026-09-11-dynamic-tree-parse-and-evaluate-design.md`

## Global Constraints

- The `Dynamic` tree's structural grammar is corpus-observed, not
  standard-normative (RESEARCH §4.3). Only the `@test` value grammar
  (`Condition_t`, `Project Schema23 v01.00.00.md` §1.1.3.18) may be cited as
  specified. Never claim ETS behavioural parity or KNX certification.
- Never silently drop a construct. An unknown element kind is stored under its
  own name, reported as a diagnostic, and activates nothing.
- No branch matching means nothing under that `choose` is active, and it is
  always reported. Never guess a branch.
- `knx-productdb` must not gain a dependency on `knx-core`, `knx-store`,
  `knx-etsproj`, `knx-app`, the server or the UI. `cargo run -p xtask --
  check-layering` stays clean with no new exception.
- The existing limitation "parameter values are retained but not interpreted,
  no parameter editor" stays true and must not be downgraded. An evaluator
  existing underneath it is not an editor.
- Import behaviour must not change: it reads `GroupObjectTree` (ADR-0014) and
  never evaluates this tree.
- Test-first: write the failing test, watch it fail for the right reason, then
  implement.

---

### Task 1: Parse and store the `Dynamic` tree at schema v3

**Files:**
- Create: `crates/knx-productdb/src/dynamic/mod.rs`
- Create: `crates/knx-productdb/src/dynamic/parse.rs`
- Modify: `crates/knx-productdb/src/lib.rs`
- Modify: `crates/knx-productdb/src/migration.rs`
- Modify: `crates/knx-productdb/src/parse/program.rs`
- Create: `crates/knx-productdb/tests/dynamic_tree.rs`

**Interfaces:**
- Produces: table `dynamic_node` per the design's D2, and
  `dynamic::parse::parse_dynamic_trees(conn, program_scope, bytes, sha256,
  collector)`.
- Preserves: every existing `Static`-tree row, unchanged; the existing
  `Dynamic`-is-skipped behaviour of the `Static` parser, which must keep
  skipping rather than double-reading.

- [ ] **Step 1: Write the failing storage tests.**

  In `crates/knx-productdb/tests/dynamic_tree.rs`, assert against small
  hand-written `ApplicationProgram` XML fixtures: a `Dynamic` tree with
  `Channel`/`ParameterBlock`/`choose`/`when`/`ParameterRefRef`/
  `ComObjectRefRef` stores one row per element in document order with correct
  `parent_id`/`position`; an unknown element kind is stored under its own
  literal name; an unmodelled attribute lands in `extra` and in
  `ingest_unknown`; a `ModuleDef`'s own `Dynamic` tree is stored with its
  `module_def_id` set while the program's own tree uses `''`.

- [ ] **Step 2: Run them and watch them fail because no table exists.**

  Run: `cargo test -p knx-productdb --test dynamic_tree`

- [ ] **Step 3: Add schema v3 and the table.**

  Bump `CURRENT_PRODUCTDB_VERSION` to 3, add `migrate_v2_to_v3` creating
  `dynamic_node` and its `dynamic_node_sibling` unique index exactly as the
  design's D2 specifies.

- [ ] **Step 4: Implement the parser.**

  A `quick-xml` pass that enters `Dynamic` (both the `ApplicationProgram`'s own
  and each `ModuleDef`'s), walks the subtree keeping a parent stack, assigns
  `node_id` in document order and `position` per sibling group, maps the
  modelled attributes of the design's D4 table into columns and everything else
  into `extra` plus the existing `UnknownCollector`. It must not interpret
  `@test`; storage is verbatim.

- [ ] **Step 5: Wire it into ingest without disturbing the `Static` pass.**

  `parse/program.rs` keeps skipping `Dynamic` in its own pass; the new parser
  runs as a second pass over the same bytes inside the same transaction. Update
  the module comment there, which currently claims the grammar is unresearched.

- [ ] **Step 6: Add the corpus counting test.**

  A corpus test that installs each `.knxprod` under
  `OriginalData/ProductDatabases/` into a scratch database and asserts the
  stored `choose`/`when` counts per archive against RESEARCH §4.3's table
  (1646/2252, 5/5, 509/982, 0/0), plus 0 dangling `choose/@ParamRefId` against
  `parameter_ref`. Follow `installs_the_readable_corpus`'s existing
  `KNXBENCH_PRODUCT_CORPUS` override and loud `eprintln!` skip idiom.

- [ ] **Step 7: Run the focused tests and the workspace suite.**

  Run: `cargo test -p knx-productdb` then `cargo test --workspace`

- [ ] **Step 8: Commit.**

### Task 2: Backfill existing databases and evaluate the tree

**Files:**
- Create: `crates/knx-productdb/src/dynamic/evaluate.rs`
- Modify: `crates/knx-productdb/src/dynamic/mod.rs`
- Modify: `crates/knx-productdb/src/migration.rs`
- Modify: `crates/knx-productdb/tests/dynamic_tree.rs`

**Interfaces:**
- Consumes: `dynamic_node` rows from Task 1, plus the existing
  `parameter_ref`/`parameter`/`parameter_type`/`parameter_type_enum` rows.
- Produces: `dynamic::{load_tree, evaluate, Activation, Diagnostic, Test, Op}`.

- [ ] **Step 1: Write the failing evaluator unit tests.**

  Over hand-built trees, one test per behaviour: each `Test` shape including
  all six operators; a space-separated list; `@default` fallback; no match
  (`NoBranchMatched`, nothing activated); a missing value; a non-numeric value;
  a `TypeNone`-controlled `choose` taking its sole default branch with no
  diagnostic; a `TypeNone` `choose` of any other shape
  (`UnexpectedTypeNoneShape`); an unknown element kind (`UnrecognizedNode`,
  subtree not descended); a `Module` node (`ModuleNotExpanded`); duplicate
  activations deduplicated by first occurrence in document order.

- [ ] **Step 2: Run them and watch them fail.**

  Run: `cargo test -p knx-productdb --test dynamic_tree evaluate`

- [ ] **Step 3: Implement `Condition_t` parsing.**

  `Test::parse` handling the three normative alternatives, returning an error
  rather than a guess for anything else. `&lt;`/`&gt;` arrive already decoded by
  `quick-xml`; the parser sees `<`/`>`.

- [ ] **Step 4: Implement the evaluator.**

  Single-pass, depth-first, document order, per the design's D6/D8/D9/D10/D11.
  Value resolution order: supplied value, then `parameter_ref.value`, then
  `parameter.value`, then `MissingValue`.

- [ ] **Step 5: Write the failing migration-backfill test.**

  Build a database at v2 (schema-v2 SQL plus a stored `source_file` blob for a
  real application program), migrate it to v3, and assert `dynamic_node` rows
  appeared without any re-install.

- [ ] **Step 6: Implement the backfill in `migrate_v2_to_v3`.**

  After creating the table, iterate `source_file` blobs and run the Task 1
  parser over each. A blob that contains no `Dynamic` simply contributes
  nothing. A parse failure on one blob must not abort the migration — record it
  through the existing unknown/diagnostic path and continue, because a database
  that refuses to open is worse than one with a gap.

- [ ] **Step 7: Add the corpus evaluation test.**

  Over every stored `choose` in the `.knxprod` corpus, evaluate with defaults
  only and assert: zero `UnparsableTest` diagnostics, zero
  `UnresolvedParamRef`, and that the `@test` shape histogram matches RESEARCH
  §4.3 restricted to these archives. Assert the `TypeNone` idiom holds on the
  real corpus subset (every `TypeNone`-controlled `choose` has exactly one
  default `when`).

- [ ] **Step 8: Run the focused tests and the workspace suite.**

  Run: `cargo test -p knx-productdb` then `cargo test --workspace`

- [ ] **Step 9: Commit.**

### Task 3: Reconcile documentation and retire the stale "unresearched" comments

**Files:**
- Modify: `docs/IMPLEMENTATION_STATUS.md`
- Modify: `docs/KNOWN_LIMITATIONS.md`
- Modify: `docs/COMPATIBILITY.md`
- Modify: `docs/DATA_MODEL.md`
- Modify: `docs/GAP_ANALYSIS_ETS.md`
- Modify: `docs/ROADMAP.md`
- Modify: `docs/ARCHITECTURE.md`
- Modify: `crates/knx-core/src/parameter.rs`
- Modify: `crates/knx-core/src/module.rs`

**Interfaces:**
- Consumes: what Tasks 1 and 2 actually shipped, read from the code, not from
  this plan's intentions.

- [ ] **Step 1: Fix the stale source comments.**

  `knx-core/src/parameter.rs` and `knx-core/src/module.rs` still say the
  `Dynamic` grammar is unresearched. It is not; the `@test` value grammar is
  documented and the structural grammar is corpus-observed. Correct both
  comments without changing any behaviour. (`knx-productdb/src/parse/program.rs`
  is Task 1's to fix.)

- [ ] **Step 2: Update the documentation set.**

  Record the evaluator as existing, headless, and corpus-tested; keep the
  parameter-editor limitation standing. State the no-match policy and that it
  is an inference, the `TypeNone` special case, and that `Module` expansion is
  not implemented. `IMPLEMENTATION_STATUS.md` gets a dated entry for
  2026-09-11.

- [ ] **Step 3: Verify internal consistency.**

  Grep `docs/` and `crates/` for `unresearched`, `R3`, `Dynamic` and confirm
  every remaining occurrence is either historical narrative (ADRs and earlier
  specs/plans, which are dated records and stay as they are) or currently true.

- [ ] **Step 4: Run the full gate suite.**

  Run: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --
  -D warnings`, `cargo test --workspace`, `cargo run -p xtask --
  check-layering`, `cargo deny check`

- [ ] **Step 5: Commit.**

## Plan self-review

- Tasks 1 and 2 both touch `dynamic/mod.rs` and `migration.rs`, so they run
  strictly in sequence, never in parallel.
- Task 1 deliberately stores `@test` verbatim and does not parse it; Task 2
  owns parsing. A reviewer seeing no `Condition_t` handling in Task 1 is
  seeing the intended split.
- Task 2's backfill runs the Task 1 parser inside a migration, which is the
  first Rust-bearing migration in this crate. That is intentional (design D5)
  and the reason the blob store exists at all (ADR-0011).
- The corpus tests skip loudly when `OriginalData/` is absent. A green run
  proves nothing about real files without `-- --nocapture`; the reviewer
  should ask for that output rather than assume it.
