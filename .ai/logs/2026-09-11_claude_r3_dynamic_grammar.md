# 2026-09-11 — Claude — R3 research spike: `when/@test` grammar and `Dynamic`-tree evaluation semantics

Worktree: `.worktrees/r3-dynamic-grammar`, branch `r3-dynamic-grammar`, forked
from `main` at `5108337`. Documentation-only task: writing up an already-run
research spike's findings. No code changed, no tests added or modified
beyond the standard workspace gate run to prove nothing broke.

## What this was

Risk R3 has been open since Session 0 (RESEARCH.md's original §11 risk
table): the grammar of `when/@test` expressions inside an
`ApplicationProgram`'s `Dynamic` tree was never studied, and every
parameter-related limitation in this repository's `docs/` names R3 as its
cause. T18 (parameter interpretation and editor,
`GAP_ANALYSIS_ETS.md` Tier 5, "the single largest remaining gap by effort")
is explicitly blocked on it.

A separate spike (not part of this task) ran against a read-only extraction
of 4 `.knxprod` manufacturer product databases and 3 `.knxproj` demo/
reference projects under `OriginalData/` (4 manufacturers, 34
`ApplicationProgram` elements, 22630 `when` elements, 12149 `choose`
elements — counts independently cross-checked against raw `grep -o` on the
source XML, not just parser output) and against
`/mnt/daten-i/Sourcecode/knx-spec-kb/extracted/The KNX Standard v3.0.0/`.
This task's job was to get that spike's findings into the repository's
permanent documentation, accurately, without overstating them, and without
touching code.

## The one-sentence verdict

**R3 the research question is answered. T18 the implementation does not
exist.** The KNX Standard v3.0.0 normatively specifies the `@test` *value*
grammar (simpleType `Condition_t`, `Project Schema23 v01.00.00.md`
§1.1.3.18 — independently re-verified against
`Project Schema23 v01.00.00.json#/tables/82` before writing anything, not
just trusted from the spike report). It does **not** specify the
surrounding *structural* grammar (`Dynamic`, `Channel`, `ParameterBlock`,
`choose`, `When_t`, `ChannelIndependentBlock`) anywhere in this
repository's Standard extraction — that part stays corpus-observed only.
No parameter evaluator or editor exists; no limitation about parameter
editing is lifted by this change.

## What the spike settled (full detail in `docs/RESEARCH.md` §4.3)

- The `Condition_t` grammar: single number, space-separated list, or a
  comparison expression (`= != > < >= <=`); controlling parameter must be
  `TypeNumber` or `TypeRestriction`; for `TypeRestriction` the comparison
  uses `Enumeration/@Value`. [Standard-normative.]
- Four observed `@test` shapes across 22630 `when` elements with zero
  unparsed residue: `SINGLE_INTEGER` (19138), `DEFAULT_ATTR(true)` (3417),
  `SPACE_LIST_OF_INTEGERS` (62), `OP_NUMBER(>)` (13 — only `>` of six legal
  operators ever appears, all in one `prod3` application program).
- `choose`→`ParameterRef`→`Parameter`→`ParameterType` resolves 100% of the
  time (0/12149 dangling). Type distribution: `TypeRestriction` 94.3%,
  `TypeNone` 5.0%, `TypeNumber` 0.7%.
- `@default="true"` is the corpus-observed fallback selector (not part of
  `Condition_t`); "first/only match wins, default covers the rest" is a
  well-supported inference, not a documented rule.
- Three findings flagged as load-bearing for T18's design:
  1. `TypeNone`-controlled `choose` (604/12149, always exactly one
     `default="true"` child) contradicts `Condition_t`'s own stated
     `TypeNumber`/`TypeRestriction` constraint — a real ETS/MT4 tooling
     idiom the Standard's literal text has no defined behaviour for.
  2. "No branch matches" is common (5570/8732 no-default `choose`
     elements have a legal enumeration value no `when` covers), not a
     corner case, and undocumented by either source.
  3. `ChannelIndependentBlock` was discovered mid-spike, is new to this
     repository's documentation, and its late discovery is itself
     evidence the when-child vocabulary is an observed superset, not a
     closed grammar.
- `when`-child vocabulary and counts, corpus evidence table (7 archives),
  nesting depth (max 10), and the evaluation-order argument for
  single-pass top-down sufficiency — all in RESEARCH.md §4.3, marked by
  confidence level throughout ([D]/[V]/[A], matching this document's
  existing convention).

## What it did not settle

The `Dynamic`/`Channel`/`ParameterBlock`/`choose`/`When_t`/
`ChannelIndependentBlock` complexType grammar (no schema document covers
it in the current `knx-spec-kb` extraction); the no-match evaluation rule;
`Access`/`Visible` as independent gating mechanisms (the `Memory`-child
correlation came back ~50/50, `Visible` was never observed at all). See
RESEARCH.md §4.3's "sharpest remaining unknowns" for what would resolve
each.

## What T18 should do with this

Per the spike's own advisory (not a decision made by this task): the value
grammar and resolution chain are solid enough to build an evaluator against
now. `TypeNone` needs its own explicit code path. The no-match case needs
an explicit (even conservative) policy decision before shipping — it is
common, not rare. The parser should preserve or loudly flag unrecognized
`Dynamic`/when-child constructs rather than silently drop them, since this
spike already found one undocumented one mid-research.

## Documentation changed by this task

- `docs/RESEARCH.md` — new §4.3 (the main write-up), refined §4.1's closing
  sentence and the `Dynamic` tree diagram, updated the R3 risk-table row
  and the open-questions list (item struck through, replaced by what is
  still genuinely open).
- `docs/ARCHITECTURE.md`, `docs/COMPATIBILITY.md`, `docs/DATA_MODEL.md`,
  `docs/IMPORT_EXPORT.md` (two spots), `docs/KNOWN_LIMITATIONS.md` (four
  spots, cause/lifted-when lines only — the limitations themselves stand),
  `docs/GAP_ANALYSIS_ETS.md` (T18 entry), `docs/ROADMAP.md` (Session 4
  carry-forward bullet), `docs/IMPLEMENTATION_STATUS.md` (correction plus
  this session's own dated entry) — each corrected in its own voice, per
  the brief, not a copy-pasted sentence.

A precise, narrow refinement was made to one existing claim: `RESEARCH.md`
§4.1/`DATA_MODEL.md` describe "Module-based objects" as schema ≥21 — that
claim is about *project*-level `ModuleInstance`/`ModuleDef` composition and
stands untouched (no schema-20 *project* was in this spike's corpus). But
at the *application-program* level, `ModuleDef` and the extended `Dynamic`
when-child vocabulary are already present at scheme 20 (`prod3`, an MDT
product database) — one scheme lower than previously observed. RESEARCH.md
§4.3 states both halves explicitly rather than blanket-correcting
`DATA_MODEL.md`'s unrelated claim.

`.ai/CURRENT_STATE.md` was deliberately **not** touched — the controller
owns that file and writes its entry after this work is reviewed.

## Gate

`cargo test --workspace` run to prove documentation-only changes broke
nothing (no source files touched). Internal-consistency grep audit of
`docs/` for `unresearched`, `R3`, `when/@test`, `choose`/`when` performed
before committing — see the task report for the full results.
