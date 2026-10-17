# 2026-10-17 — Claude — block Access and separator rows (census fix package 1)

Source: `docs/research/2026-10-17-product-database-gap-census.md`, G1/G2 → P1/P2.
Decision record: ADR-0139.

## What changed

- **ProductDB v24** (`migration.rs`, `dynamic/parse.rs`): `dynamic_node.access`
  (ParameterBlock, ParameterSeparator) and `dynamic_node.ui_hint`
  (ParameterSeparator). Same shared column-promotion step as v23
  (`ACCESS_HOSTS`, backfill by re-parsing retained bytes, unknown-attribute rows
  retired, failures recorded per program). Legacy EX-IM programs are skipped,
  because their trees come from the legacy mapper and not from XML.
- **Evaluator** (`dynamic/evaluate.rs`): `PathContext { channel, block_access }`
  replaces the bare channel argument of `walk`. `ActiveRef::block_access` holds
  the most restrictive block on the path. A repeated activation keeps the
  least restrictive path (`Seen` maps each key to its list index).
  `Activation::separators` lists separators with `parameter_refs_before`.
  `record_label` now returns whether a due label was refused, so a
  budget-truncated separator is omitted. Block access lives in
  `DynamicTree::access` (`with_access`), not in `DynamicNode`, so the
  hand-built trees are untouched.
- **Layout** (`parameter_layout.rs`): `SeparatorDeclaration` (group, element id,
  `ui_hint`, own `access`) for every separator, grid or not.
- **Server** (`domain.rs`, `routes.rs`): `WriteRefusal::BlockAccessRestricted` →
  `parameterBlockAccessRestricted`; `field.blockAccess`;
  `section.separators[]`, where `position` comes from a `partition_point`
  over the global activation indices of the section's fields. Sections that
  hold only separators exist. Module-instance authority is skipped for them
  (one match guard).
- **Web**: `restrictingAccess`, `isSeparatorRestricted` and
  `filterParameterSection` (renumbers positions) in `parameterPresentation.ts`;
  `items` in `parameterWorkspace.ts`; `SeparatorRow` in `ParameterWorkspace.tsx`;
  captions and the diagnostic key in `ParameterPanel.tsx`; EN/DE messages.

## Evidence

Policy probe over the local collection (counts only, private): 254,222 refs
restricted somewhere, 301 also unrestricted, 1 under both None and Read, 0
blocks re-opening inside a more restrictive one, 402 restricting blocks in
ModuleDefs, 1,903 separators with their own `Access="None"`, 213,499
separators outside grids and 29,785 inside. Schema 23 §1.1.2.1 documents
`Access_t` only; inheritance into a block's contents is inferred.

## Pitfalls

- A splice in Python computed its offsets before an earlier replacement
  shifted the text and swallowed half an interface. Recompute indices after
  every edit, or use unique anchors.
- `corpus_compatibility_matrix::matrix_output_is_refused_inside_the_corpus_root`
  fails when `TMPDIR` is a symlink: the guard does not refuse an output path
  reached through a symlinked corpus root. It passes with the resolved path.
  This predates the package and belongs to the P4 claim (corpus-output
  refusal through symlinks), so it was not changed here.
- The fresh worktree needs `npm ci` in `apps/knx-web` before `tsc`/`vitest`.
