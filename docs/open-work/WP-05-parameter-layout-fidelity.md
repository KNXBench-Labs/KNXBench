# WP-05 — Parameter layout fidelity: remaining features

**Status:** In progress: conditional block rename is implemented (9 November 2026).
Literal program-owned axis translations are implemented, package-gate accepted and
published on main (14 November), with independent source review; broader features stay open.
Listed in [OPEN_WORK](../OPEN_WORK.md); per-ID status of record is the [ledger](../status/LEDGER.md).

**Evidence route:** clean-room observation is admitted by the owner ([OWN-01](OWN-01-owner-decisions.md#decisions-recorded--8-november-2026), [rules](../RESEARCH.md#evidence-routes)). Bounded observations cover
[conditional block rename](../research/2026-11-09-conditional-parameter-block-rename.md)
and [literal program-owned axis translations](../research/2026-11-13-literal-grid-axis-translations.md);
neither admits the other layout behaviors below.

## Task

Reproduce further declared parameter-layout features (KL-003), one evidenced
feature at a time, never inferred manufacturer behaviour.

## Open (each needs its own evidence)

- Parameter-driven row/column captions (`TextParameterRefId`). `Rename` is implemented only for the
  literal leaf that retitles its enclosing block; module-scoped, cross-target and text-parameter
  renames remain open (see below).
- Translated row/column captions outside the bounded literal program-owned
  implementation; module/Repeat, template and text-reference semantics remain open.
- Inline sections, nested blocks and `CollapseIfEmpty`.
- Button actions inside grid cells (`EventHandler*`, online callbacks, own
  `Access`, translated or text-parameter Button text, module-scoped Buttons).
  The literal text alone is delivered as passive labels.
- `Table` versus `Grid` visual rules (needs the referenced schema rules
  document) and remaining control hints.

## Already delivered (do not redo)

Area register, matrix placement, separators, block access, indentation, the
#26 section-caption resolver (reuse it), the Alpha 16 block/separator
`Text` language overlays (A16-07, `cde8d89a3`) and, since Alpha 19, passive literal
Button labels in grid cells
([`button-literal-passive-v1`](../contracts/alpha16-parameter-layout.md#passive-source-button-labels-button-literal-passive-v1)).

## Implemented and published 14 November 2026: literal axis translations

Source-bound literal program-owned rows/columns select their own requested-language
Text without changing layout identity, values or activation. Missing stays source;
explicit empty stays empty; selected placeholders are not evaluated. The
[contract](../contracts/alpha16-parameter-layout.md#literal-program-owned-axis-translations)
records executed fictional ProductDB, actual HTTP and both owning matrix-browser
regressions, preservation boundaries and independent Standards/Spec approval with
no findings. Existing-value history/reopen coverage is not allocator rewind;
intercepted browser fixtures are not joined real-server/native compatibility.
After that, the selected 15-stage package gate (Rust 6001 passed/181 ignored, corpus 97/97, 37 web stages) accepted it and it was published on main 14 November; not released. WP-05 is not closed.

## Implemented 9 November 2026: conditional block rename

A literal `ParameterBlockRename`/`Rename` leaf that names its own enclosing block now replaces
that block's caption while its `when` is active, in navigation, heading, context and search,
with its own translation and the base caption restored when it is not. Contract:
[conditional-parameter-block-rename](../contracts/conditional-parameter-block-rename.md).
Evidence: one ETS6 observation (one product, German text, no claim about repaint timing).
Still open: module-scoped renames, renames of anything but the enclosing block, competing
or undecided leaves, text-parameter or template replacements (a selected translation with a
placeholder falls back to the leaf's source text and is named in the panel diagnostics), other ETS versions, languages and products.
The slice does not close the package.
Accepted on main `da7d667d1`: [software acceptance](../status/2026-11-09-conditional-block-captions.md),
including the scoped gate, preserved failed runs and next-release ProductDB obligation.

## Done when (per admitted feature)

Its contract names source identity, activation/access interactions and the
unsupported fallback; browser regressions cover search, drafts, restricted
rows, narrow layout and grid/row variants. Undefined hints stay visible as
unsupported rather than guessed.

## References

[Alpha 16 layout contract](../contracts/alpha16-parameter-layout.md) ·
[positional text templates](../contracts/positional-text-templates.md) ·
[ADR-0137](../adr/0137-declared-parameter-grids-reach-the-workspace.md) ·
[gap census](../research/2026-10-17-product-database-gap-census.md) ·
[KL-003](../limitations/KL-003-parameter-evaluation-and-editing-boundaries.md) ·
[history](../history/OPEN_WORK_2026-10-25.md#wp-05--parameter-layout-fidelity)
