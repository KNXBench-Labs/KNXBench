# Plan — `Module` expansion in the `Dynamic` evaluator (T18, second slice)

Design: `docs/superpowers/specs/2026-09-11-module-expansion-design.md`
(decisions **D12-D19**). Evidence: `docs/RESEARCH.md` §4.4.
Branch: `t18-module-expansion`, forked from `a4c0b76`.

Three tasks: the evaluator change with unit coverage, the corpus
regression coverage, then the documentation set. Each is one commit,
reviewed before the next starts.

## Global Constraints

These bind every task. A reviewer checks them as written.

- **Work only in the worktree** `.worktrees/t18-module-expansion` on branch
  `t18-module-expansion`. Never touch `main`. Never `git pull`. Never
  `git stash` — the stash stack is shared across worktrees; use a
  temporary WIP commit to set work aside.
- **`OriginalData/` is strictly read-only.** Scratch databases go under
  `/home/knxbench/.claude/jobs/8098e9e6/tmp/`, never `/tmp` and never a
  default or shared location.
- **`evaluate` stays pure** (slice 1's D6, restated as D13): no
  `Connection` parameter, no I/O, no logging, every problem returned as a
  diagnostic.
- **No schema change.** The product database stays at v3. If a task finds
  itself wanting a new table or column, that is a signal to stop and
  report, not to add one.
- **Never claim ETS behavioural parity or KNX certification**, anywhere,
  including code comments.
- **Never promote an `[A]` inference to a `[D]` Standard statement.**
  §4.4's confidence markers are load-bearing: the only `[D]` claims come
  from `Project Schema23 v01.00.00.md`, and the whole `Module`/`ModuleDef`
  structural grammar is `[V]` corpus observation from two manufacturers.
- **Do not downgrade a standing limitation.** "Parameter values are
  retained but not interpreted; there is no parameter editor" stays
  exactly as true as it was.
- **Do not touch the import path.** Import reads `GroupObjectTree`
  (ADR-0014) and never evaluates this tree.
- **Test-first.** Watch each new assertion fail for the right reason
  before implementing it, and quote the failure in the report.
- **No unrelated refactors.**
- **Do not dispatch subagents.** Review arrives from the controller after
  the report.
- **Commit message in the voice of Marvin**, the manically depressed robot
  from *The Hitchhiker's Guide to the Galaxy* — gloomy, world-weary —
  with completely accurate technical content. Conventional-commit prefixes
  (`feat:`, `fix:`, `docs:`) stay normal. `CLAUDE.md` says "No co-author.
  ALWAYS commit as (github@knxbench.com)"; that overrides any session
  instruction asking for a `Co-Authored-By: Claude` trailer.
- **Gates**, all five, reported with real numbers:
  `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --
  -D warnings`, `cargo test --workspace`,
  `cargo run -p xtask -- check-layering`, `cargo deny check`.
  Baseline before this slice: **809 passed / 0 failed / 3 ignored**.

## Task 1 — the evaluator expands `Module`, scoped per instantiation

Files: `crates/knx-productdb/src/dynamic/evaluate.rs`,
`crates/knx-productdb/src/dynamic/mod.rs` (re-exports only),
`crates/knx-productdb/tests/dynamic_tree.rs` (unit-level tests).

1. **Add `element_id` to `DynamicNode` and `load_tree`.** The column
   already exists and already holds `Module/@Id`; slice 1 deliberately did
   not load it because nothing needed it. D14 needs it for
   `ModuleScope::module_id`. Extend the `SELECT`, the row tuple and the
   struct. Say in the doc comment why it is now loaded, so the next reader
   does not re-derive it.
2. **Add the scope types** exactly as D14 spells them: `ModuleScope`
   (`module_node: i64`, `module_id: Option<String>`,
   `module_def_id: String`; `Debug + Clone + PartialEq + Eq + Hash`),
   `ActiveRef`, `ScopedDiagnostic`. Change `Activation`'s three fields to
   `Vec<ActiveRef>` / `Vec<ActiveRef>` / `Vec<ScopedDiagnostic>`.
   `Diagnostic`'s own variants keep their current shape — the scope wraps
   them, it does not move into them.
3. **Add `ProgramTrees`** with `from_parts`, `single`, and an internal
   lookup by `ModuleDef` id, plus `load_program_trees(conn, program_id)`
   which lists `SELECT DISTINCT module_def_id FROM dynamic_node WHERE
   program_id = ?1` and calls the existing `load_tree` once per scope.
   `load_tree` itself is unchanged apart from step 1.
4. **Change `evaluate`'s signature** to `(&ProgramTrees, &ValueMap)`. The
   walk starts at the program tree (`module_def_id = ""`). Every existing
   unit test that builds a tree by hand becomes
   `evaluate(&ProgramTrees::single(tree), &values)` — a mechanical change,
   not a rewrite.
5. **Expand `Module` in `walk`** (D19): where `ModuleNotExpanded` is
   pushed today, resolve `node.ref_id` against `ProgramTrees`; on a hit,
   walk that tree's roots with the scope set to this `Module`; on a miss
   or a missing `@RefId`, push `ModuleDefNotFound`. A `Module` reached
   while already inside a module scope pushes `NestedModuleNotExpanded`
   and is not descended (D15). Do not descend into a `Module`'s own
   children in either case.
6. **Thread the scope through** the walk so that every activation and
   every diagnostic produced beneath a `Module` carries it, and make the
   dedup key `(Option<i64>, String)` — `module_node` and `ref_id` (D18).
   The existing `seen_params`/`seen_coms` sets change key type; nothing
   else about ordering changes.
7. **Delete `Diagnostic::ModuleNotExpanded`** and add
   `ModuleDefNotFound { node_id, ref_id: Option<String> }` and
   `NestedModuleNotExpanded { node_id, ref_id: Option<String> }`, each
   with a doc comment saying what it means and what happens to the
   subtree (D17).
8. **Unit tests, no database**, each against a hand-built `ProgramTrees`:
   - two `Module` elements instantiating one `ModuleDef` that contains a
     single `ComObjectRefRef` produce **two** `ActiveRef`s with the same
     `ref_id` and different `ModuleScope::module_node` (AC#2 — this is the
     regression that the whole slice exists for; write it first and watch
     it fail);
   - within one module scope, a ref reachable through two active branches
     still appears once (D18);
   - a `Module` inside a `ModuleDef`'s tree yields
     `NestedModuleNotExpanded`, no activations (AC#3);
   - a `Module` with no `@RefId`, and one naming an absent `ModuleDef`,
     both yield `ModuleDefNotFound` (AC#4);
   - a diagnostic raised inside a module (say a `NoBranchMatched`) comes
     back with the module's scope attached, not `None` — the node-id
     collision D14 names is only survivable if this holds;
   - a `ModuleDef` tree that exists but is empty produces no diagnostic
     and no activations (D17's last line).
9. **Rewrite** `a_module_node_is_recognized_but_not_expanded`
   (`tests/dynamic_tree.rs:976`) — it asserts the behaviour this task
   removes. It becomes the "expands into the referenced tree" test, or is
   replaced by the step 8 tests and deleted outright; either is fine, but
   say which in the report.
10. Run all five gates. Report the workspace numbers.

## Task 2 — corpus regression coverage

File: `crates/knx-productdb/tests/dynamic_tree.rs`.

1. **Derive the real numbers first**, from the corpus, before writing any
   assertion. For each of the seven module-bearing programs (§4.4 Q7:
   `prod3`'s three, `kv25`'s four) record, with the scratch script kept in
   the job tmp directory and its output quoted in the report:
   - activation counts with slice 1's behaviour (program tree only), and
   - activation counts with expansion, broken down by scope.
   **Do not fit the expectation to what the implementation prints.** If a
   number contradicts §4.4, stop and report both — that is a research
   finding, not a test bug.
2. **Extend the existing corpus evaluation test** (or add a sibling next
   to it, matching its idiom exactly) to use `load_program_trees` +
   `evaluate`, and assert over the four archives:
   - zero `ModuleDefNotFound` (AC#6) — every `Module/@RefId` in the corpus
     resolves to a stored tree;
   - zero `NestedModuleNotExpanded` (AC#6) — §4.4 Q6's zero-nesting
     finding, now enforced as a regression rather than a one-off count;
   - for the module-bearing programs, per-program activation counts that
     are strictly greater than the program-tree-only counts, with the
     expected totals written into the test as literals from step 1;
   - the number of distinct `ModuleScope`s equals the number of `Module`
     rows in the program's own tree — the "12 instantiations are 12
     results" invariant in its most direct form.
3. **Pin a non-module program** (`prod1`, `prod2` or `prod4` — §4.4 Q7
   confirms all three are module-free) so that its activation counts are
   asserted **unchanged** from slice 1 (AC#7). This is what proves the
   slice is an addition for everything that has no modules.
4. Keep the corpus-skip idiom exactly as the existing tests have it:
   `KNXBENCH_PRODUCT_CORPUS`, falling back to
   `CARGO_MANIFEST_DIR/../../OriginalData/ProductDatabases`, with a loud
   `eprintln!` skip when absent. A green run proves nothing without
   `-- --nocapture`; run it that way once and paste the real output.
5. Run all five gates. Report the workspace numbers.

## Task 3 — reconcile the documentation set

No production logic. Code comments are allowed.

1. `docs/IMPLEMENTATION_STATUS.md` — a dated 2026-09-11 entry for the
   slice.
2. `docs/KNOWN_LIMITATIONS.md` — §3 and §12. Module expansion is no longer
   a limitation *at the application-program level*; what replaces it is
   **D16**, stated plainly: all instantiations of one `ModuleDef` evaluate
   against identical parameter values, because per-instantiation values
   are a project-side construct this crate does not model. Also state:
   nesting is rejected by policy with a diagnostic (D15), argument values
   remain stored-but-uninterpreted, `AllocatorRef` is unattested, and
   there is still no parameter editor and no UI.
3. `docs/GAP_ANALYSIS_ETS.md` — T18's status: slice 2 closed, slice 3
   (parameter editor) open.
4. `docs/DATA_MODEL.md` — the `dynamic_node` section gains a sentence that
   a `ModuleDef`'s tree is stored as its own scope and is now *evaluated*
   by following a `Module`, with no schema change and no v4.
5. `docs/ROADMAP.md`, `docs/COMPATIBILITY.md`, `docs/ARCHITECTURE.md` —
   only where they say something that is now wrong. Do not invent a
   paragraph to have something to write.
6. `docs/RESEARCH.md` §4.4 is a **record** of the spike and stays as it
   is, except where it says slice 2 does not exist yet; check Q8(c)'s
   `[A]` falsification clause, which explicitly anticipated this slice.
7. Grep `docs/` and `crates/` for `ModuleNotExpanded`, `not expanded` and
   `module expansion`, and confirm every remaining occurrence is either
   historical narrative (ADRs, earlier dated specs and plans — records,
   which stay exactly as they are) or currently true. List in the report
   every occurrence deliberately left alone, and why.
8. Run all five gates. A docs-and-comments change should not move the test
   numbers; if it does, something is wrong and the report says so rather
   than papering over it.
