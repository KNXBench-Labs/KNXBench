# Offline naming recipes

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-offline-naming-recipes)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## Offline naming recipe boundary (2026-10-12)

The [bounded literal recipe](../contracts/offline-name-recipe.md) changes only names
of explicitly ordered existing GroupAddressIds. No interpolation, regex, target
inference, macro registry, UI writer, new format or bus operation is added; the
only MCP writer is the opt-in ADR-0116 surface above.
Native working history is not root Save; reopen through the existing editor to
Undo/Redo, and Save explicitly when desired. Concurrent generation/working-hash
conflicts require re-preview. A failed or lost output after a durable commit is
not evidence of rollback: inspect/reopen rather than automatically retrying.
The deterministic CLI fingerprint binds canonical target, exact intent, semantic
native context and history generation; it is not an authentication capability.
The broader FUTURE-02 scope remains in the ledger, not closed by this small case.
<!-- END retained catalogue record -->

## Update 2026-10-13: a web dialog (Goal B7)

"No … UI writer" above is superseded: the web app offers the same literal
recipe as **Rename…** for a group-address selection, through server routes bound
to the live snapshot ([contract](../contracts/offline-name-recipe.md#web-consumer-goal-b7-13-october-2026)).
Everything else in the boundary stands: explicit ids only, no interpolation,
regex, target inference, macro registry or bus operation, and the broader
FUTURE-02 scope stays in the ledger.
