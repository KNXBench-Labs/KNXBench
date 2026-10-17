# Declared parameter grids in the Parameters tab (Claude, 17 October 2026)

## Outcome

Published on `main` as merge `0a13b670` (feature `8dbba68a`, main-first parent
`6a8b681b`). ADR-0137 (renumbered from a draft 0136, which main had meanwhile
given to plug-in data adoption).

Parameter areas whose product data declares a `ParameterBlock` with
`Layout="Table"|"Grid"`, one non-empty `Rows` and one non-empty `Columns` render
as a matrix of the existing `ParameterFieldRow`s at their declared 1-based
`Cell`s. Every other area keeps the row view and states why: no grid declared,
server does not report layouts, unknown version, unknown layout token (kept
verbatim, e.g. `List`), invalid declaration, unreadable source, unknown status.
Cell problems (malformed, out of range, missing, ambiguous, unshown content) are
named per item. Nothing is derived from captions, names or identifiers; no
manufacturer case; no schema, Core or write-path change.

## Layers

- ProductDB `parameter_layout/grid.rs`: re-reads `Layout`/`Rows`/`Columns`/`Cell`
  from the hash-checked retained source (8 MiB editor limit) with the caption
  reader's ownership rules (now shared predicates in `ownership`), own event
  budget (4,000,000) and depth limit (64); budget overflow is `sourceUnavailable`,
  never "no grid". Attribute parsing is restricted to elements that can carry
  grid data (measured hot path).
- Server `parameter_grid.rs`: DTO mapping; `section.groupLayouts` (always present,
  possibly empty) and `field.cell`, wire version 1; captions only from active
  separators placed in exactly that block and scope.
- Web `parameterMatrix.ts`: pure classification and CSS placement; the matrix
  uses the same stable rows parent, so drafts, focus and conflicts survive
  navigation and areas still start closed. Search stays a row view.

## Evidence

- RED observed first for ProductDB, HTTP (3 cases at the `groupLayouts`
  assertions), Vitest and the 400 px browser overflow.
- Full gate on merge `f24ebb62` (main `a340c5a8`): fmt, web build, workspace
  Clippy, `cargo test --workspace` 4799/0 (212 ignored), ts-rs bindings
  (EOL-only), five xtask checks, documentation, local-info, tool tests, private
  corpus 153/153, web gate 63/63 complete; inputs frozen.
- Main then moved by planning docs only; docs-delta acceptance on `0a13b670`:
  tree = `f24ebb62` + upstream delta hunk for hunk, documentation, tool tests,
  whitespace and five xtask checks green; fetch-conditional push, readback equal.
- Browser: EN/DE x 1440/400 px passed in both the fixture lane and the
  production `devices` build.
- Mutation: disabling the block/scope separator filter fails the HTTP test.
- Private real data (aggregates only): reader equals the independent census on
  all 13 grid-bearing members (602 grids, 1,513 rows, 3,163 columns, 3
  ambiguous); release median 85.5 ms per `load` on the largest member.

## Open

`Table` vs `Grid` visual rules ([PSR] 2.1.1 not available locally),
`CollapseIfEmpty`, `TextParameterRefId` captions, in-cell buttons and nested
blocks, translated axis captions (OPEN_WORK, KL-003).

## Environment notes

- Workspace Clippy in a fresh worktree needs `apps/knx-web/dist` (Tauri build
  script); build the web app first.
- `corpus_compatibility_matrix::matrix_output_is_refused_inside_the_corpus_root`
  fails with a symlinked `TMPDIR` and passes with the resolved path; matches P4
  item 3.
- `tools/browser_tmp.py` approves a temp root only through a lexical `scratch`
  parent; a resolved scratch path fails the `test_browser_tmp*` suites.
