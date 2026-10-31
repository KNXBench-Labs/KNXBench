# Manufacturer distribution inspection is partial and read-only

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-187)

<!-- BEGIN retained catalogue record -->

## Read-only distribution inspection (31 October 2026)

`knx products inspect-distribution` reports what a manufacturer XML or
distribution ZIP declares. Without a master file and a signature it is not a
product package. The report format, rules and limits are in the
[contract](../contracts/distribution-inspection.md).

Limits:

- **Not installable.** Installation still refuses these sources. Inspection
  gives no write, programming or initialization authority.
- **Dynamic is a subset.** Only what the existing evaluator activates from raw
  source fallbacks. Dynamic is not attempted for ambiguous or unparsed
  applications.
- **Dependencies are declarations only.** Scripts, calculations, validations,
  button handlers and Assign are listed with their declared endpoints
  (`dependencyEvidence`, report format 3) but never run, resolved to functions
  or applied. Their conditions are not evaluated, and impact beyond declared
  Assign endpoints is unknown.
- **Reference coverage is a subset.** Mask, medium and DPT references are
  external and unresolved. Argument, Rename and BinaryData references are not
  followed.
- **Not inventoried.** Comments and processing instructions are covered only
  by the document hash.
- **Whole-source refusal.** Any resource limit, including a structural one in
  a single ZIP member, refuses the whole inspection. The element ceiling
  (500,000) leaves about 2 % headroom over the largest measured real member.
- **No baggage association, translation, XSD or authenticity check.**
- **Not ETS or KNX compatibility.** A positive report says only that the
  source was read.

Lifted when: a source-backed installation route for distributions exists
(master data and authority), or the listed subsets are extended with tests.

<!-- END retained catalogue record -->
