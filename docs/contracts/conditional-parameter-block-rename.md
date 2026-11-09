# Conditional parameter-block rename (WP-05)

Status: implemented 9 November 2026 on the narrow shape below. Evidence:
[observation](../research/2026-11-09-conditional-parameter-block-rename.md). Everything outside
the shape stays what it was: `UnrecognizedNode`, base caption kept.

## Supported shape

A `ParameterBlockRename` (or its schema-23 spelling `Rename`) is applied only when all hold:

1. It sits in the application's own `Dynamic` tree, never inside a `Module` expansion or a
   `ModuleDef`. It need not be a direct child of `Dynamic`; it is reached through the existing
   `choose`/`when` engine, so an inactive branch contributes nothing.
2. Its nearest enclosing `ParameterBlock` carries an `@Id`, that `@Id` is declared by no other
   block in the tree, and the leaf's `@RefId` equals it. Naming any other block or anything
   else is a cross-target rename and is refused.
3. The leaf has its own `@Id`, declared by no other rename leaf in the tree, a non-empty
   `@Text` without `{{` placeholders, no children, and no attribute besides `Id`, `RefId` and
   `Text` (any other attribute stays in `extra` and refuses the leaf).
4. The block has a base caption (an empty base caption leaves nothing to replace).
5. Exactly one such leaf is active for that block in the evaluation. Two or more are all
   refused: there is no first-wins or last-wins.
6. Nothing else in the evaluation could name the block too. Any other rename leaf the walk
   reached or skipped past structurally (wrong shape, module scope, cross-target, an
   unexpected place below a recognized leaf) keeps the block its `@RefId` names from every
   candidate. So does a rename leaf of any shape inside a program-tree subtree that a
   `may_hide_refs` diagnostic left undecided (missing or non-numeric controller value,
   unreadable test, unresolved or unsupported controller, unrecognized element). A branch
   decided inactive is not undecided and blocks nothing; a leaf naming another block blocks
   only that block. A leaf without `@RefId` could name any block, and an evaluation that
   ran out of work or activation budget is incomplete: either blocks every rename of that
   evaluation. Rename leaves in an unexpanded `ModuleDef` are not consulted; a reached
   module-scope leaf blocks the `@RefId` it names.

Refusal changes nothing else: the leaf is reported by the existing `UnrecognizedNode`
diagnostic (and `RefBelowSkippedNode` for references below it), the block keeps its base
caption, activations, identifiers and access are untouched.

## Policy

- The evaluator reports `Activation::renames` (leaf, target block, leaf identity, source text).
  The block's `ActiveLabel` keeps the base caption; nothing mutates a label or stored source.
  Every evaluation recomputes the result, so no active leaf means the base caption again.
- The server overlay (`parameter_captions::apply`) picks the displayed caption once per
  request and the existing DTO carries it: navigation, heading, context and search read the
  same group label. The group key and every DTO field stay as they were.
- A replacement is translated by its **own** identity (`translation` entry for the leaf's
  `@Id`, attribute `Text`) with the existing language rule. The block's translation never
  overrides an active replacement; without a translation the leaf's source text is shown.
  A selected translation containing `{{` is not filled; the leaf's source text is shown and
  the panel lists an `unresolvedTextPlaceholder` diagnostic naming the leaf's node (reusing
  the existing kind, whose generic wording is owned by the shared message catalogue). A
  missing translation falls back silently; an explicitly empty one is used as it is, so the
  group has no caption. The block's positional-text fill does not apply to a replaced
  caption.
- Display only: no new command, no state write, no native schema change. Values change through
  the existing validated parameter command and undo/redo move the caption with the value.
- Restricted and hidden blocks are renamed like any other block; their access rules are
  unchanged and a rename never makes a row visible or writable.
- Budgets: a candidate counts against the existing activation budget. Scanning a leaf's text
  and every copy it keeps (text, its own `@Id`, the target `@Id`, the kind) are admitted on
  the shared evaluation work budget before allocation, as is each noted intent. Rename
  settlement runs on the final pass's own budget before it is reset, and its refusal
  diagnostics go through the ordinary admission; settlement that hits the budget applies no
  rename. In the server overlay the selected translation and the source fallback are both
  copied through the request's one `TextProjectionBudget`. Target uniqueness and refused
  subtrees are read from per-tree indexes built once (pre-order spans, built only for trees
  holding a rename leaf), never from a scan per label. No limit was raised and nothing is
  truncated.

## Product database

ProductDB v33 adds no DDL. A rename leaf already had `element_id`, `ref_id` and `text`
columns; the parser now fills them (`Id`, `RefId`, `Text` only) and the three attributes stop
being reported unknown at those hosts. The v32 to v33 step re-parses every retained
application blob that still matches its hash, per blob in its own savepoint, retires exactly
those attribute rows from the blob and from the measured install reports, and leaves a
package untouched when none of its members lost a row. A blob that cannot be re-read keeps its
old rows and gets a `RenameBackfillError` record, readable through the source diagnostics.
A measured report is rewritten only after the stored report validates as it stands; a
historical counter that disagrees with its rows or a missing mandatory row makes it
`unavailable` with an `InstallReportBackfillError`, never a rebuilt measured report. The
earlier promotions keep the unvalidated rewrite they shipped with. A real database error rolls
the whole step back.
Catalogue schema admission moves with the version (`ADMITTED_VERSION` 33; data-only).

## Not implemented

Module-scoped renames, renames of anything but the enclosing block, text-parameter or
template replacements, competing or undecided leaves, renames of channels or separators, a rename of a
block without a base caption, and any claim about ETS behaviour beyond the one observation.

## Verification

Evaluator unit tests (`evaluate/rename_tests.rs`), `tests/it/conditional_block_rename.rs`
(ingest columns, evaluation sequence over one loaded source with default value resolution,
every refused shape, cross-target and undecided competitors, module scope, v32 to v33 equal to
a fresh install with one and two members, idempotence, damaged blob, mixed intact and damaged
member, invalid historical report, injected database failure, id-conflict loser), the HTTP
test `http_parameter_block_rename.rs` (edit, undo, redo, languages including empty and
unsupported entries, restricted block, competing leaves, source bytes, translation copy
budget on GET and POST), `ParameterWorkspace.test.tsx` (row and declared-matrix areas) and
`e2e/parameter-block-rename.e2e.ts` in the `parameters` browser lane. Opt-in private tests run
every rename leaf of the held witness through the public entry points, and migrate a store
written by the v32 build itself from the invented test packages. The default-lane migration
tests start from a store rewound to the v32 shape.
