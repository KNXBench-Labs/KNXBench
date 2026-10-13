# Product and application identity policy

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-135)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->


## §135 Package identity is recorded, not decided

**Limitation.** Since PDB-11 (schema v17, ADR-0043) every element of the six
package-content kinds (`catalog_section`, `catalog_item`, `hardware`,
`product`, `hardware2program`, `application_program`) in every parsed member
blob is recorded with an element digest, and `knx products identity` names
the winner, every candidate, their packages and whether each equals the
winner. The stored row is still the **first installed** one: installing the
same packages in another order stores other values for the ids whose
elements differ. KNXBench shows this; it does not choose.

- The digest is conservative. Equal digests mean equal as the parsers read
  it: the element's subtree plus the context its stored row takes from
  outside (manufacturer, parent section, parent hardware). Different digests
  can come from differences without meaning (a different namespace prefix, a
  changed `Hash` attribute, re-wrapped text). Whitespace-only text and
  namespace URIs are not compared, as the parsers ignore them too.
- Translations are not compared. A program's `Languages`, and catalogue and
  hardware translations, live outside the element.
- Families (`ApplicationNumber`) and `ReplacesVersions` links rest on an
  unofficial public copy of the project schema plus corpus agreement, not on a
  KNX-published XSD. A value that does not parse is shown raw with the reason
  and nothing is linked; a listed version with no installed program means
  *not installed*, not *does not exist*. KNXBench implements no upgrade
  behaviour behind `ReplacesVersions`.
- Order-number lookup is exact-string per manufacturer. An order number is
  never an identity and never merges products.
- Master data is not covered: `manufacturer.name` stays last-writer-wins
  (§88) and `datapoint_type` first-wins without provenance (§86).
- A blob the scan cannot read (an entity the parsers ignore, or retained
  bytes that no longer match their hash after an upgrade), or whose rows
  from before v17 disagree with the scan, is recorded `unavailable` with its
  reason and listed as unmeasured, not guessed. It is not rescanned later.
- Server and web have no identity, family or order-number views; the
  library and CLI do.

**Cost.** A user who installs overlapping packages sees which package
supplied each value and which offer different ones, but cannot yet pick a
different winner.

**Why it is this way.** No normative ordering of package content exists:
`ToolVersion` is a free-form producer string, the namespace is a format
version, and any hash order would be arbitrary and silently change existing
databases (ADR-0043 §2).

**Lifted when.** A specified ordering of package content exists, or a later
UI slice lets the user pick a winner per id.

<a id="136-mask-0701h-bim-m112-devices-cannot-receive-an-application-download"></a>

<a id="136-mask-0701h-bim-m112-devices-can-receive-an-application-download--lifted-the-restart-stays-unconfirmed"></a>

**Alpha decision (user, 2026-10-04).** No version/pinning policy for the Alpha: first-installed winner with every candidate disclosed is the accepted boundary.
<!-- END retained catalogue record -->
