# Selective import

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-selective-import)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## Selective import and evidence boundaries (2026-10-10 source integration)

[Separate integration verification](../status/2026-10-10-import-expansion-integration.md) supersedes the local-only delivery status, not its historical test results. This is source integration, not a new release or deployment.

The [selective importer](../SELECTIVE_IMPORT.md) merges devices or complete
lines into one explicit target installation. No building-only selector, automatic
address reassignment, conflict overwrite or manufacturer-catalogue installation.
Existing area/line names may be retained with an explicit preview note; incompatible
configuration/range/address metadata refuse. Both projects must be at the current
normalized model, v14 (`SELECTIVE_MODEL_VERSION` in
`crates/knx-app/src/selective_import.rs`); each model step was audited before the
admission moved: v12 added completion vocabulary only, v13 instruction overlays
(remapped with their devices), v14 project notes (destination notes are kept; a
source carrying notes is refused rather than dropped). Future-model fields still
require a fresh reference-closure audit. This does not restrict
ETS input schemas beyond the existing complete importer.

The source cap is 64 MiB; native snapshot/history limits remain binding. Complete
source archives and opaque content can include unselected private devices/settings.
Consent is required; Undo removes model entities, not retained source evidence.
Source attributes remain scoped evidence and are not target access-key defaults.
Serial-number resolution refuses ambiguous device/source identities. Large-project
latency, every viewport/theme, native WebKitGTK/Orca and real ETS/hardware behavior
are not established by the synthetic built-browser run.

The [private revision harness](../AUTHORIZED_RESTORE_EXPORTS.md) has no authorized
historical ETS export set; its public manifest/path tests are not that real regression.
The [cvexc analyzer](../research/cvexc.md) is UTF-8-only, bounded to 8 MiB/100,000 nodes/
128 levels and preserves unknown observations, not byte-exact XML serialization.
It neither verifies signatures nor activates comparison/reconstruction/download
rules. [Acceptance and rejected attempts](../status/2026-10-10-import-expansion-verification.md)
are local evidence only; no commit, integration, release or deployment is implied.
<!-- END retained catalogue record -->
