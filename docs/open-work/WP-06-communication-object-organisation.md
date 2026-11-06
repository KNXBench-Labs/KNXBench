# WP-06 — Communication-object organisation: remaining slices

**Status:** Bounded saved schema-20 text display software-accepted and published; other slices remain evidence-gated. Listed in [OPEN_WORK](../OPEN_WORK.md); per-ID status of record is the [ledger](../status/LEDGER.md).

## Admitted increment — saved schema-20 node text

The [source observation and bounded contract](../research/2026-11-05-saved-schema20-node-text.md)
admit literal saved `Node/@Text` as additional read-only display, without changing
folder identity or resolving manufacturer text/templates. Source `e4aa31945`
was independently reviewed and software-accepted through the complete selected
branch and main-first gates; runtime `61e28d225` is published and its original
live/tracking readback is retained. The [acceptance record](../status/2026-11-05-saved-schema20-node-text.md)
keeps original failures and narrow-viewport limits separate. No new compatibility
or hardware claim follows. The remaining requirements below stay independent.

## Task

Organise communication objects by source-backed folders and represent object
priority losslessly.

## Open

1. Folder captions from product `ParameterBlock`s / `ShowInComObjectTree`
   (manufacturer mapping with evidence, not guessed).
2. Folders for module objects (depends on
   [WP-07](WP-07-module-instance-authority.md)).
3. Staleness of the saved tree after edits.
4. A broader real-export membership/import witness (the public schema-20 source
   observation does not establish end-to-end vendor-export equivalence).
5. Object priority across core, persistence, import, projection and
   comparison, only after an ADR defines raw/effective provenance.

## Already delivered (do not redo)

The source/identity contract (`9145b03f`), the saved-folder display by
declared `RefId` and the *By saved project folder* grouping view
(`e6f227c4`). Object size and effective flag columns exist.

## Done when

Folder membership is deterministic and source-backed; ambiguous, inactive and
module objects do not acquire invented folders; priority survives
migration/save/reopen/undo where editable. No slice changes bus priority or
manufacturer activation rules.

## References

[Folder contract](../contracts/communication-object-folders.md) ·
[object table](../COMMUNICATION_OBJECT_TABLE.md) ·
[KL-003](../limitations/KL-003-parameter-evaluation-and-editing-boundaries.md) ·
[history](../history/OPEN_WORK_2026-10-25.md#wp-06--communication-object-organisation)
