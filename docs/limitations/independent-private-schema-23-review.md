# Independent private schema-23 review

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-independent-private-schema-23-review)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## Private schema-23 sample review (2026-10-10)

The owner asked for a read-only review of a privately supplied, independently
produced ETS6 schema-23 project against this file. The project is not in the
repository; only generic findings appear here and its evidence stays local.
It meets §125's lifting condition (lifted below) and adds evidence to §1,
§11, §52, §68 and §146 without lifting them. New limitations found on the way:
§170–§175. Updates also went into §6 and §134 (ETS app packages, add-in data),
§8 (secure-capable programs), §106 (endpoints in retained source attributes)
and the PDB-9 boundary (grid layout, value forms).

**Historical, pre-repair (10 October 2026):** the importer did not yet import
that project end to end. Archive name lookup, `Node`-style object trees and
unassigned placement were repaired in the
[import-integrity integration](../status/2026-10-10-import-integrity-integration.md);
since then the explicit private tests run the production importer on this
project (for example `crates/knx-app/tests/private_schema23_reconciliation.rs`)
and are recorded as passing in the
[review closure receipt](../evidence/private-schema23-review-closure-2026-10-11.json).
That is one private regression input, not general ETS compatibility. Private opt-in gate: `a_second_schema_23_project_resolves_every_object_id_through_the_production_rules` (`crates/knx-etsproj/tests/private_schema23_refids.rs`, `KNXBENCH_PRIVATE_SCHEMA23_PROJECT`). [Receipt](../evidence/private-schema23-kl-review-2026-10-10.json).
<!-- END retained catalogue record -->
