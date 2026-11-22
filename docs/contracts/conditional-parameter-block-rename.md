# Conditional parameter-block rename (WP-05)

Status: implemented 9 November 2026 on the narrow shape below; cross-target
renames within one tree (a leaf naming another block, or a channel owner, of the
same tree) were admitted 19 November 2026 (WP-05i); a leaf may also name a
`ParameterSeparator` of its own tree since 19 November 2026 (WP-05k).
Evidence: [observation](../research/2026-11-09-conditional-parameter-block-rename.md).
Everything outside the shape stays what it was: `UnrecognizedNode`, base caption
kept. The 9 November observation never showed a leaf naming anything but its
enclosing block; admitting same-tree cross-target renames and separator targets
is KNXBench policy on the declared `@RefId`-addresses-a-host semantics, not an
observed ETS rule, and is marked as such here.

## Supported shape

A `ParameterBlockRename` (or its schema-23 spelling `Rename`) is applied only when all hold:

1. It sits in a walked `Dynamic` tree: the application's own tree, or a `Module` expansion's
   tree (module-scoped renames, delivered 19 November 2026). It need not be a direct child
   of `Dynamic`; it is reached through the existing `choose`/`when` engine, so an inactive
   branch contributes nothing. A leaf inside an unexpanded `ModuleDef` is not consulted.
2. Its `@RefId` names a caption host declared exactly once in the leaf's own tree: a
   `ParameterBlock`, a channel owner (`Channel`/`ChannelIndependentBlock`), which
   carries a label of its own, or a `ParameterSeparator` with a base caption
   (WP-05k: the separator's `@Text` is its caption like a block's; a separator
   without a non-empty `@Text` leaves nothing to replace and is not a host).
   The host may be the leaf's enclosing host or any other host
   of the same tree — a cross-target rename within the tree is admitted (WP-05i, policy
   above). Naming a host of another tree (e.g. a module-scope leaf naming a program-tree
   block) is a cross-scope rename and is refused, as is naming anything else: no such id,
   an id declared twice, or a non-host element.
3. The leaf has its own `@Id`, declared by no other rename leaf in the tree, a non-empty
   `@Text`, no children, and no attribute besides `Id`, `RefId`, `Text` and the optional
   `TextParameterRefId` (any other attribute stays in `extra` and refuses the leaf). A `@Text`
   containing `{{` placeholders is admitted only when the leaf also carries
   `@TextParameterRefId`; the placeholder then resolves from that text parameter (see Policy).
   Without the attribute, a placeholder refuses the leaf as before.
4. The addressed host has a base caption (an empty base caption leaves nothing to replace).
5. Exactly one such leaf is active for that host in the evaluation. Two or more are all
   refused: there is no first-wins or last-wins.
6. Nothing else in the evaluation could name the host too. Any other rename leaf the walk
   reached or skipped past structurally (wrong shape, cross-scope, an
   unexpected place below a recognized leaf) keeps the host its `@RefId` names from every
   candidate. So does a rename leaf of any shape inside a program-tree subtree that a
   `may_hide_refs` diagnostic left undecided (missing or non-numeric controller value,
   unreadable test, unresolved or unsupported controller, unrecognized element). A branch
   decided inactive is not undecided and blocks nothing; a leaf naming another host blocks
   only that host. A leaf without `@RefId` could name any host, and an evaluation that
   ran out of work or activation budget is incomplete: either blocks every rename of that
   evaluation. Rename leaves in an unexpanded `ModuleDef` are not consulted; a reached
   module-scope leaf blocks the `@RefId` it names **within its own module tree** — a
   module-scope leaf naming a program-tree block is a cross-scope refusal.

Refusal changes nothing else: the leaf is reported by the existing `UnrecognizedNode`
diagnostic (and `RefBelowSkippedNode` for references below it), the host keeps its base
caption, activations, identifiers and access are untouched.

## Policy

- The evaluator reports `Activation::renames` (leaf, target host, leaf identity, source text,
  and the leaf's module scope chain — `None` for the program tree). Settlement is per tree:
  competing candidates, intents and blocked targets count within the candidate's own tree
  (`module_def_id`), so a module-scope leaf never competes with or blocks a program-tree
  host, and vice versa. The target host is resolved through a per-tree index of caption-host
  `@Id`s (`ParameterBlock` plus `Channel`/`ChannelIndependentBlock`, and since WP-05k
  `ParameterSeparator`), built once per tree that holds a rename leaf — never by scanning
  labels. The host's `ActiveLabel` keeps the
  base caption; nothing mutates a label or stored source. Every evaluation recomputes the
  result, so no active leaf means the base caption again.
- The overlay matches an accepted rename to a label by the rename's module scope chain and
  target node (WP-48.3), so a sibling instance of the same definition whose branch is inactive
  keeps its base caption. A replacement filled from `@TextParameterRefId` carries the real
  fill origin on the caption.
- The server overlay (`parameter_captions::apply`) picks the displayed caption once per
  request and the existing DTO carries it: navigation, heading, context and search read the
  same group label. The group key and every DTO field stay as they were. The replacement
  applies to any labelled host — block, channel owner or separator — of the same tree.
- A replacement is translated by its **own** identity (`translation` entry for the leaf's
  `@Id`, attribute `Text`) with the existing language rule. The block's translation never
  overrides an active replacement; without a translation the leaf's source text is shown.
  A selected translation containing `{{` is not filled; the leaf's source text is shown and
  the panel lists an `unresolvedTextPlaceholder` diagnostic naming the leaf's node (reusing
  the existing kind, whose generic wording is owned by the shared message catalogue). A
  missing translation falls back silently; an explicitly empty one is used as it is, so the
  group has no caption. The block's positional-text fill does not apply to a replaced
  caption.
- Exception to the rule above: a leaf whose **source** `@Text` contains `{{` and that carries
  `@TextParameterRefId` is admitted (item 3), and the displayed replacement's placeholder
  fills from the text parameter the attribute names, resolved in the leaf's own scope —
  program scope for a program-tree leaf, the module expansion's scope otherwise. The fill
  runs through the shared text-projection budget exactly like every other positional fill;
  an unresolved reference behaves like any other unresolved placeholder (source text shown,
  diagnostic named). A **selected translation** with a placeholder is still never filled,
  even when the leaf names a text parameter: the source text is shown and the diagnostic is
  named, because the translation's positional values belong to the leaf's own text, not to
  the block's.
- Display only: no new command, no state write, no native schema change. Values change through
  the existing validated parameter command and undo/redo move the caption with the value.
- Restricted and hidden hosts are renamed like any other host; their access rules are
  unchanged and a rename never makes a row visible or writable.
- Budgets: a candidate counts against the existing activation budget. Scanning a leaf's text
  and every copy it keeps (text, its own `@Id`, the target `@Id`, the kind) are admitted on
  the shared evaluation work budget before allocation, as is each noted intent. Rename
  settlement runs on the final pass's own budget before it is reset, and its refusal
  diagnostics go through the ordinary admission; settlement that hits the budget applies no
  rename. In the server overlay the selected translation and the source fallback are both
  copied through the request's one `TextProjectionBudget`. Target uniqueness and refused
  subtrees are read from per-tree indexes built once (caption-host `@Id` index and pre-order
  spans, built only for trees holding a rename leaf), never from a scan per label. No limit
  was raised and nothing is truncated.

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

Renames of a host of another tree (cross-scope: a module-scope leaf naming a
program-tree block and vice versa), template replacements,
competing or undecided leaves, a rename of a host without a base caption,
a placeholder in a selected translation (still never filled), and any claim about
ETS behaviour beyond the one observation. Same-tree cross-target renames (another
block or a channel owner of the leaf's own tree) are implemented as of
19 November 2026 (WP-05i) as KNXBench policy, not ETS observation. Separator
targets (`@RefId` naming a `ParameterSeparator` of the leaf's own tree) are
implemented as of 19 November 2026 (WP-05k), likewise as KNXBench policy. Text-parameter
replacements via the leaf's own `@TextParameterRefId` and module-scoped renames
are implemented as of 19 November 2026 (`rename-text-parameter-ref-v1`,
module-scoped rename admission).

## Verification

Evaluator unit tests (`evaluate/rename_tests.rs`, including same-tree cross-target
admission, channel-owner targets, separator targets (WP-05k), cross-scope refusal and
competing cross-target
leaves), `tests/it/conditional_block_rename.rs`
(ingest columns, evaluation sequence over one loaded source with default value resolution,
every refused shape, admitted cross-target and channel-target leaves, a separator target,
undecided competitors,
module scope, v32 to v33 equal to
a fresh install with one and two members, idempotence, damaged blob, mixed intact and damaged
member, invalid historical report, injected database failure, id-conflict loser), the HTTP
test `http_parameter_block_rename.rs` (edit, undo, redo, languages including empty and
unsupported entries, restricted block, cross-target rename onto a restricted block with its
own translation, competing leaves, a separator target through the panel DTO, source bytes,
translation copy
budget on GET and POST), `ParameterWorkspace.test.tsx` (row and declared-matrix areas) and
`e2e/parameter-block-rename.e2e.ts` in the `parameters` browser lane. Opt-in private tests run
every rename leaf of the held witness through the public entry points, and migrate a store
written by the v32 build itself from the invented test packages. The default-lane migration
tests start from a store rewound to the v32 shape.
