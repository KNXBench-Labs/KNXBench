# Project user files and add-in payloads

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-174)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 174. Project user files and add-in data are filed as plain container entries

**Historical baseline.** Project `UserFiles`/`AddinData` reports lacked payload
content descriptions; manufacturer media were described too coarsely. Integrated
F08 already corrected the manufacturer payload description. The source storage
kinds remain unchanged deliberately: retained bytes are not executable domain data.

**Bounded implementation (2026-10-10, `41b3d6267731`).** The real application import
uses existing `sniff_media` for project and manufacturer whole-member payloads.
Opaque summaries and unsupported notices separately state byte-based content
(`zip`, `pe-executable`, image/media classes, XML, empty, unknown) and unverified
name/location roles. `.etsapp` gives **ETS-app-named candidate (identity unverified)**;
`AddinData` gives **add-in-state candidate (semantics unverified)**. Misleading
extensions, unknown bytes and combined/mixed-case hints are tested. No nested
content is unpacked, rendered or executed. Independent same-member capability
warnings remain intact. Exact source path, bytes, owner, hash and native/product
storage destinations are unchanged.

Named evidence: `project_and_manufacturer_payloads_report_content_without_certifying_roles`
(with/without product DB, native Save/Reopen),
`independent_capability_diagnostics_are_not_replaced_by_content_evidence`,
`payload_evidence_reaches_the_http_session_log_without_certifying_an_app`.
Two assertion-killed mutations and restored-source focuses pass. Final no-ff
source `566afbdd` passes eight scoped crates (2400/0/181), strict Clippy/fmt,
five nonempty repository/docs gates, complete nonempty corpus, private original
reconciliation and RefIds. Published/read back `68147ba3` with local/tracking/live equality; no source values published.

**Remaining boundary.** The owner explicitly accepts descriptive, unverified
candidates. Content magic is not App identity/authenticity, licensing, manifest
validation, Add-in-state semantics, media rendering or executable compatibility.
No device is flagged from an uninspected App manifest. Persisted old reports are
not rewritten; fresh application imports get the enrichment, parser-only entry
points remain generic. See [ADR-0109](../adr/0109-opaque-payload-content-and-candidate-roles.md),
[receipt](../evidence/kl174-payload-evidence-2026-10-10.json) and ledger `KL-174`.
<!-- END retained catalogue record -->
