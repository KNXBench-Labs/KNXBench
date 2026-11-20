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

- Parameter-driven row/column captions (`TextParameterRefId`): grid axis captions now resolve
  the referenced text parameter (delivered 17 November 2026, ADR-0177; contract updated).
  Since 20 November 2026 (WP-05l) the resolved caption's fill provenance (`textFill` in the
  wire DTO) also rides the matrix row/column header entries and shows in the header's
  tooltip, the same provenance separator labels already had; a plain axis header keeps no
  fill line, and placement, identities and the conservative caption fallback are unchanged.
  `Rename` is implemented for the literal leaf that retitles its enclosing block — in the
  program tree and in a `Module` expansion (module-scoped, delivered 19 November 2026) —,
  including a `{{0}}` placeholder filled from the leaf's own `@TextParameterRefId`. Since
  19 November 2026 (WP-05i) a leaf's `@RefId` may also name **another caption host of the
  same tree** — a different `ParameterBlock`, or a `Channel`/`ChannelIndependentBlock` — a
  cross-target rename, and since 19 November 2026 (WP-05k) a `ParameterSeparator` with a
  base caption too, all admitted as KNXBench policy on the declared
  `@RefId`-addresses-a-host semantics (the 9 November observation never showed one);
  naming a host of another tree
  (cross-scope) stays refused. See the
  [contract](../contracts/conditional-parameter-block-rename.md).
- Translated row/column captions outside the bounded literal program-owned
  implementation; module/Repeat, template and text-reference semantics remain open.
- Inline sections: an `Inline` block's fields/separators now join the outer
  block's group and its label renders as a row-view heading in the parent page
  (`inline-section-flatten-v1`, product DB v34, contract section added
  18 November 2026). `Row/@CollapseIfEmpty` is applied since
  17 November 2026 (truthy value + row holds no field/label hides the row in
  the matrix; see [contract](../contracts/parameter-workspace.md)).
- Nested blocks: a program-owned nested `ParameterBlock`'s own `@Cell` now
  places its literal caption in the parent grid
  (`nested-block-cell-passive-v1`, contract section added 18 November 2026);
  the nested block's fields stay in the row view. Since 18 November 2026 a
  block nested two or more levels deep binds in the first ancestor with a
  renderable grid (`nested-block-cell-deep-v2`). Nested-block actions remain
  open.
- Button actions inside grid cells (`EventHandler*`, online callbacks, own
  `Access`, module-scoped Buttons). A Button `@Text` placeholder filled from
  the Button's own `@TextParameterRefId` and a requested language's own `Text`
  translation overlaying the literal are delivered since 19 November 2026
  (`button-text-parameter-ref-v1`, `button-text-overlay-v1`); the action itself
  is still never run.
  Admitted literal in-cell text alone is delivered as passive labels. The
  [WP05-B clarification](../contracts/alpha16-parameter-layout.md#passive-source-button-labels-button-literal-passive-v1)
  (14 November 2026) records the unchanged reporting policy for a reached
  program-owned Button without `Cell`: no passive label or additional grid-local
  `unshownContent`; the existing `UnrecognizedNode` remains. This is KNXBench
  policy, not an observed ETS rule; no product code, test or fixture change.
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
After that, the selected 15-stage package gate (Rust 6001 passed/181 ignored, corpus 97/97, 37 web stages) accepted it and it was published on main 14 November; ships in Alpha 22. WP-05 is not closed.

## Implemented 9 November 2026: conditional block rename

A literal `ParameterBlockRename`/`Rename` leaf that names its own enclosing block now replaces
that block's caption while its `when` is active, in navigation, heading, context and search,
with its own translation and the base caption restored when it is not. Contract:
[conditional-parameter-block-rename](../contracts/conditional-parameter-block-rename.md).
Evidence: one ETS6 observation (one product, German text, no claim about repaint timing).
Still open: cross-**scope** renames (a module-scope leaf naming a program-tree host and
vice versa), competing
or undecided leaves, template replacements, a placeholder in a selected translation (falls
back to the leaf's source text and is named in the panel diagnostics; a placeholder in the
leaf's source text is filled from its own `@TextParameterRefId` since 19 November 2026), other ETS versions, languages and products.
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
