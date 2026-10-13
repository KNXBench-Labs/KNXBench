# UTF-8 text download encoding

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-171)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 171. UTF-8 text-parameter encoding is refused for download images

**Public-source research completed 2026-10-10; boundary unchanged.**
[Inspected sources and missing contract](../research/schema23-offline-boundaries.md)
record the public evidence for KL-171. No sufficient manufacturer encoding or
loaded-payload contract was found in the inspected sources; no runtime change,
private payload decoding, byte swap or hardware action follows. The negative
finding is scoped to this search, not a claim that a specification cannot exist.

**Limitation.** `TypeText` parameters are encoded only for
`TextParameterEncoding` `iso-8859-1` and `iso-8859-15`, or as ASCII when the
program declares none. Any other declaration, `utf-8` included, refuses the
whole image once a text field is encoded (`TextParameterEncoding "…" is not
one this crate writes`).

**Prevalence.** Same local product corpus: `utf-8` 21, `iso-8859-1` 17,
`iso-8859-15` 12, undeclared 475 program files. A privately supplied project
contains `utf-8` programs too; its offline readiness does not show this
refusal, presumably because §170 fires first (inferred from refusal order, not
measured).

**Cause.** No available source says whether a UTF-8 field's size counts octets
or characters, or how a multi-octet sequence that does not fit is cut.

**Impact.** Such programs get no offline image when the image contains a text
parameter. Display and editing are unaffected.

**Lifted when.** A documented source settles length and truncation, and tests
cover multi-octet characters at the field boundary. Ledger `KL-171`.
<!-- END retained catalogue record -->
