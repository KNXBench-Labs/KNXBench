# Duplicate building-part names in comparison

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-053)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 53. Project diff can collide two same-named sibling building parts

**Limitation.** A building part's natural key is the path of names from
the root (design spec §3.4). Two siblings under the same matched parent
that share a name produce the identical path and therefore collide under
the ambiguity rule (design spec §3.3 step 3): both are reported as
individual `added`/`removed` entries, plus one `AmbiguityNote`, rather
than matched to each other.

**Cause.** The same limitation `knx-etsproj::compare`'s
`semantic_building_part` already accepts for its own single-parent-hop
identity (design spec §3.4's own note): a name-based key has no way to
distinguish same-named siblings, and building parts carry no other
stable identity once their `ets_id`s also fail to correlate.

**Impact.** Renaming, or otherwise editing, one of two same-named sibling
building parts between two saves can render as an ambiguous add/remove
pair instead of a clean field change.

**Measured 2026-09-23 (T15).** Across the ETS4, ETS6, and independent
Schema-21 corpus projects, there are zero duplicate `(parent, name)` sibling
groups. This remains a supported ambiguity path, not evidence that the
shape cannot occur in another project.

**Lifted when.** Open. Recorded as a boundary of the path-based key, not
a bug awaiting a fix; the available corpus gives it no implementation
priority over observed comparison problems.
<!-- END retained catalogue record -->
