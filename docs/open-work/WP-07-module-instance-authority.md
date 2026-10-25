# WP-07 — Module-instance authority

**Status:** Blocked: evidence. Listed in [OPEN_WORK](../OPEN_WORK.md); per-ID status of record is the [ledger](../status/LEDGER.md).

## Task

Establish `ModuleDef`/`ModuleInstance`, repeat and number-offset identity
(KL-068, PDB-01) for module-scoped consumers and for constructing newly
activated module objects. Caption matching is no substitute.

## Current state

Read-only own instance values and unscoped object creation are delivered. A
parameter write that would activate a module object not already represented
by an imported instance is refused as a whole; the project stays unchanged.

## First deliverable

One admitted mapping contract backed by a real, independently counted
witness, with missing, duplicate, nested and repeated cases.

## Done when

Evaluation, parameter writes and object reconciliation reuse that authority;
ambiguous mappings refuse the whole edit without spending IDs or changing
native history. Identity-keyed module comparison follows the mapping.

## References

[ADR-0108](../adr/0108-read-only-module-instance-values.md) ·
[object-refresh contract](../contracts/parameter-object-refresh.md) ·
[KL-068](../limitations/KL-068-repeated-module-activation.md) ·
[source stop lines](../research/2026-10-16-offline-source-stop-lines.md) ·
[history](../history/OPEN_WORK_2026-10-25.md#wp-07--module-instance-authority)
