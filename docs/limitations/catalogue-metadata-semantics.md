# Catalogue metadata semantics

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-catalogue-metadata-semantics)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## PDB-7 catalogue metadata are source strings, not capabilities

**Limitation.** The eight application-program catalogue attributes persisted
at product-database schema v13 — `IsSecureEnabled`,
`MaxSecurityGroupKeyTableEntries`, `MaxSecurityIndividualAddressEntries`,
`MaxSecurityP2PKeyTableEntries`, `MaxTunnelingUserEntries`, `MaxUserEntries`,
`MinEtsVersion` and `ReplacesVersions` — are stored and shown exactly as the
manufacturer wrote them. They are never parsed into booleans, capacities,
version ranges or replacement graphs.

**Cause.** No public schema defines their value space, and the private
115-instance corpus shows non-uniform shapes: `MinEtsVersion` appears both as
dotted-numeric and as other forms, `ReplacesVersions` both as an unsigned
decimal and as other forms. Guessing a type would be an invention, and
`IsSecureEnabled="true"` in a catalogue file is a vendor claim about a
product, not a verified property of the device on the bus.

**Cost.** A user cannot filter by "devices supporting KNX Data Secure", sort
by real ETS version, or follow `ReplacesVersions` as links. The strings must
be read by a human.

**Lifted when.** A normative value-space definition, or a large enough
multi-manufacturer corpus, justifies a typed projection alongside — never
instead of — the retained source string. Interpreting `IsSecureEnabled` as a
security capability would additionally require the runtime KNX Secure work
tracked in limitation 8.

**Related.** Stricter XML well-formedness is now enforced for every recognized
application-program document, on direct ingest as well as on package install
and v12→v13 backfill. A file whose comments, entities, attributes, prolog or
element nesting violate XML 1.0 is rejected with a typed error instead of
being partially interpreted. All 115 corpus instances pass this check; a
hypothetical vendor file that ETS tolerates but XML forbids would now be
refused rather than half-read.
<!-- END retained catalogue record -->
