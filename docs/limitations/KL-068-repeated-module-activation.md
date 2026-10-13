# Repeated-module activation

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-068)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

<a id="68-repeated-module-instantiation-is-refused-not-supported"></a>

## 68. Repeat activation stays unsupported; own instance values are readable

**Update 2026-10-10 — read-only instance dimension delivered in code.**
[ADR-0108](../adr/0108-read-only-module-instance-values.md) separates stored
instance evidence from Dynamic evaluation. `ValueMap` now has a separate
`(module_id, instance_id, ref_id)` map. The device evaluator validates the
imported owner and a unique declaration, then records each recognized instance's
own raw value. Missing values never fall back to a sibling or a program default.
The new map is not seeded from defaults and is not consumed by Dynamic.

HTTP `instanceValues`, the Inspector's explicitly read-only/not-evaluated
section, and both MCP parameter readers expose this evidence separately.
MCP uses `notEvaluated` / `moduleInstanceNotEvaluated`. Unknown, missing or
ambiguous identities remain raw/stale evidence; duplicate stored rows preserve
the first own row and report the later raw row instead of overwriting it.
Namespace association is an additional guard for this new map, not a new
restriction on the existing alias-tolerant single-instance evaluator.

**Still unsupported:** expansion/activation of a `Module` beneath `Repeat`,
iteration-to-MI mapping, writable repeated instances and inferred resolution
of omitted counters. Project Schema23 describes the list-valued `RepeatIndex`
shape and nominal identity reference; that is not a complete mapping algorithm
for nested pairs, subsets or missing controlling values. Private correlation
is not normative evidence. RepeatIndex stays uninterpreted.

**Unchanged single-instance contract:** a lone imported instance may have an
MI suffix above one and remain writable under the existing authority rules.
The legacy definition-alias path is likewise unchanged. Neither fact grants
write authority to the new read-only evidence.

**Verification:** `repeated_instance_values` (including sibling isolation,
missing values, identity/declaration ambiguity, duplicates, controller/default
independence and legacy aliases); `http_repeated_instance_values`; unchanged
`http_parameter_panel` write tests; MCP `instance_tests`; Inspector en/de unit
and browser witnesses. Code `d050c7e78b02`.
[Receipt](../evidence/kl68-instance-values-2026-10-10.json).

**Private verification passed after import integration (2026-10-10).** The
ignored opt-in `private_instances_read_their_own_stored_values` ran explicitly
on `734064823e19` and passed with boolean assertions; the independent private
RefId gate passed too. Missing input still refuses, never passes. The earlier
parser refusal on `8901affa` is historical, not the current result.
Whole-project reconciliation remains a separate KL-1 gate.
Full Repeat support remains an accepted boundary (PDB-01).

**Final integrated admission 2026-10-10 (`f4af614eac30`).** The explicit
`private_instances_read_their_own_stored_values` and independent RefId gates
pass again after import-integrity and public-research integration. The full
corrected corpus also passes; its four inherited test contracts were updated
only with owner authorization after exact fresh-main failures (`e74be53a`),
without runtime changes or private expected values. This closes the old parser
and corpus delivery blockers, not the Repeat/activation/write boundary.
<!-- END retained catalogue record -->
