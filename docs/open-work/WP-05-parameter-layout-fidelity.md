# WP-05 — Parameter layout fidelity: remaining features

**Status:** Blocked: evidence. Listed in [OPEN_WORK](../OPEN_WORK.md); per-ID status of record is the [ledger](../status/LEDGER.md).

## Task

Reproduce further declared parameter-layout features (KL-003), one evidenced
feature at a time, never inferred manufacturer behaviour.

## Open (each needs its own evidence)

- Parameter-driven row/column captions (`TextParameterRefId`) and `Rename`.
- Translated row/column captions.
- Inline sections, nested blocks and `CollapseIfEmpty`.
- Buttons inside grid cells.
- `Table` versus `Grid` visual rules (needs the referenced schema rules
  document) and remaining control hints.

## Already delivered (do not redo)

Area register, matrix placement, separators, block access, indentation, the
#26 section-caption resolver (reuse it) and the Alpha 16 block/separator
`Text` language overlays (A16-07, `cde8d89a3`).

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
