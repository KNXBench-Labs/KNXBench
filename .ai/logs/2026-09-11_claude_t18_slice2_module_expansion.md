# T18 slice 2 — `Module` expansion in the `Dynamic` evaluator

Date: 2026-09-11
Branch: `t18-module-expansion`, forked from `a4c0b76`
Merged to `main` as `f683c4e` (merge `--no-ff`, five commits, 14 files,
1713 insertions, 202 deletions)

## What this changed architecturally

`knx-productdb`'s `Dynamic` evaluator now follows a `Module` node into
the referenced `ModuleDef`'s own stored tree. Before this slice it
emitted a `ModuleNotExpanded` diagnostic and walked past, so a modular
application program's computed activation set was truncated at every
module boundary.

Design decisions **D12-D19** are recorded in
`docs/superpowers/specs/2026-09-11-module-expansion-design.md`. The four
that matter to anyone reading the code later:

- **D13** — `evaluate` stays pure. Its input grew from a single
  `DynamicTree` to a `ProgramTrees` (the program's own tree plus every
  `ModuleDef` tree); the new `load_program_trees` is the only added
  function that touches a `Connection`. No I/O, no logging, every
  problem still returned as a diagnostic.
- **D14** — every activation and diagnostic is qualified by an
  `Option<ModuleScope>` (`module_node`, `module_id`, `module_def_id`).
  This is not cosmetic. A `ModuleDef`'s `ParameterRef`/`ComObjectRef`
  ids are declared once and reused verbatim by every sibling `Module`,
  **and** `dynamic_node.node_id` restarts at zero in every stored tree
  (see `dynamic/parse.rs`, `handle_dynamic_element`), so node ids collide
  across trees. Two independent reasons the qualifier is mandatory.
- **D15** — exactly one level. A `Module` reached while already inside a
  module scope yields `NestedModuleNotExpanded` and is not descended.
  The corpus contains zero nested modules and nothing documents a cycle
  rule, so unbounded recursion was rejected rather than guessed at.
- **D18** — the dedup key is `(Option<module_node>, ref_id)`. With slice
  1's flat `ref_id` key, `prod3`'s three programs evaluate to 52/48/44
  activations instead of 382/258/134. Measured during review, not
  estimated.

`Diagnostic::ModuleNotExpanded` is gone, replaced by `ModuleDefNotFound`
and `NestedModuleNotExpanded`.

**No schema change.** The product database stays at v3; the
`dynamic_node` table already stored each `ModuleDef`'s tree under its own
`module_def_id`, and already held `Module/@Id` in `element_id` — slice 1
simply never loaded that column. The import path is untouched (ADR-0014:
import reads `GroupObjectTree` and never evaluates this tree).

## Evidence, and its limits

Corpus regressions now enforce, across the four installed `.knxprod`
archives: zero `ModuleDefNotFound`, zero `NestedModuleNotExpanded`. For
`prod3`'s three module-bearing programs, activation totals grow from
22/18/14 (program tree only) to 382/258/134 (expanded), across 12/8/4
distinct `ModuleScope`s. A module-free control program is pinned at 145
activations, unchanged between the two modes.

Every expected number was derived from the raw XML by a separate
implementation before any assertion was written, and none was adjusted
afterwards to match what the code printed.

**The corpus evidence covers `prod3` only.** `docs/RESEARCH.md` §4.4 Q7
lists seven module-bearing application programs, but four of them live in
the `kv25` demo `.knxproj` under `OriginalData/DemoProjects/`, which the
corpus tests do not install — they install `.knxprod` archives from
`OriginalData/ProductDatabases/`. Any future claim about module
behaviour beyond `prod3`'s three programs needs new evidence first.

## A finding worth remembering

`prod3`'s programs hold 44/28/14 structural `Module` rows, but `evaluate`
reaches only 12/8/4 of them under the corpus's own default parameter
values: the `Module`s naming `MD-2`/`MD-3`/`MD-4` sit on `choose`
branches the defaults never select. Verified independently twice.

This falsified the plan's own phrasing, which had asserted that the
number of distinct `ModuleScope`s equals the number of `Module` rows in
the program's tree. The reachable count is the correct invariant, and the
test asserts it. The plan document was left as written — it is a dated
record.

## What did not close

Stated in `docs/KNOWN_LIMITATIONS.md` §3, and worth repeating here so it
is not rediscovered as a surprise:

- All instantiations of one `ModuleDef` evaluate against **identical**
  parameter values (D16). Genuinely per-instantiation values
  (`ParameterInstanceRef`, the mangled `_M-<m>_MI-<k>_` id scheme) are a
  project-side construct `knx-productdb` does not model.
- Argument values (`NumericArg`/`TextArg`) remain stored but
  uninterpreted. `choose` never branches on them (§4.4 Q3), so the
  activation set does not need them, but memory-offset placement and
  `{{ChNo}}`-style text substitution are unresearched.
- `AllocatorRef` has zero corpus occurrences and stays unattested.
- There is still no parameter editor and no UI. Nothing outside
  `knx-productdb`'s own tests calls the evaluator.

## Process record

Three tasks, each implemented by a separate Sonnet subagent in an
isolated worktree and reviewed by another before the next started, then a
final whole-branch review. Task 3 needed one fix round: three
present-tense false claims survived the plan's literal grep because
`` `Module` expansion `` carries a backtick and a space between the two
words that `module expansion` does not match. Worth remembering — a grep
pattern chosen from prose will miss the same prose once someone wraps a
code element in backticks.

All five gates green on merged `main`: `cargo fmt --all --check`,
`cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace` **817 passed / 0 failed / 3 ignored** (up from
809), `cargo run -p xtask -- check-layering`, `cargo deny check`
(advisories ok, bans ok, licenses ok, sources ok).
