# Manufacturer report history

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-manufacturer-report-history)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## PDB-3 report history and coverage boundary

Product install facts are measured only for installs carrying the schema-v12
encounter ledger. Older installs deliberately render as unavailable rather
than as zero. The projection reports unsupported constructs and diagnostics,
but does not interpret every vendor construct, verify signatures, or claim ETS
parity. PDB-8 (schema v14) reports uninterpreted element subtrees inside
supported master sections (`Format`, `PublicKeys`, `OrderNumberFormattingScript`)
but gives none of them typed storage: DPT bit layouts, manufacturer public keys
and order-number scripts are retained bytes plus a diagnostic, not queryable
data. Section-level master data (`MaskVersions` with its resources and access
rights, `InterfaceObjectTypes`, …) likewise stays reported per section, not per
element, and untyped until a commissioning feature proves which parts it needs.
AR05's v19 candidate reports unconsumed attributes inside the master `Languages`
branch through a shared byte-only pass; consumed translation keys/text are not
mislabelled unknown. TranslationUnit RefId/Version remain uninterpreted, not
typed metadata. The bounded, exact-scope census observes 1,870 Version
occurrences across 67 distinct master blobs; the historical 1,928 total used
a different scope/unit and is not this gate's pin. Current source-derived
evidence is separate from immutable historical install reports. The scoped
matrix and real upgrade pass after independently reconciling the +230 isolated /
+226 shared keys; no normalized values or prior unknown evidence were lost.
Repair-path regressions also preserve unexamined status for identity-invalid
bytes while clearing stale exact-owner markers after authentic recovery.
The former acceptance/publication handoff is historical: the owning AR05
delivery and per-ID evidence are recorded in the current ledger. See
[the AR05 contract](../contracts/MANUFACTURER_REPORT_CONTRACT.md). Persisted subtree
diagnostics are validated for shape, not re-derived from the blob. PDB-10 (schema v16, ADR-0042) inventories baggage: every `Baggages.xml` declaration typed as raw lexemes and resolved exactly to its member, every payload classified by content, nested ZIPs measured from their directory only. See §134 for what it deliberately does not do.

Each entry states the limitation, cause, practical cost and lift condition.
Some are actual defects; others are evidence limits or recorded design/safety
decisions in [docs/adr/](../adr/). Do not describe all three as either inevitable
or merely unfinished work.

The companion document is [COMPATIBILITY.md](../COMPATIBILITY.md), which states
what is verified. A verified narrower capability may coexist with a remaining
limitation here; the scopes must agree rather than erase each other.
<!-- END retained catalogue record -->

## v28 correction candidate: lost member attribution (20 October 2026)

The [promotion repair](../contracts/product-promotion-repair.md) reconstructs
only byte-derived columns and targeted unsupported evidence. Earlier migrations
could delete a report key shared by several programs while subtracting only
some member counter units. Missing/partial aggregates cannot identify those
units after the fact: the candidate preserves the package counter and marks
its report unavailable, with a named diagnostic. Complete shared aggregates
and unambiguous single-member history remain eligible for reconciliation.
Opaque members never inherit program semantics by matching content hashes;
inconsistent non-program target ledgers also require unavailable reports.
The fixed evidence allowance is conservative repair policy, not a format or
process-RSS limit. Independent review accepts the corrected candidate; corpus
proof and integrated acceptance remain pending. This section does not claim
delivery or new ETS compatibility.
