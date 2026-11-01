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
- **Source candidates are arithmetic, not validity.** `sourceCandidates`
  (report format 4) compares TypeNumber values with their declared bounds
  and Union member intervals with the Union size, under named KNXBench
  models. A candidate is not a schema, ETS or device error. An omitted
  Offset or BitOffset, or a missing width, gives no interval. Nothing is
  initialized, corrected or chosen; official schema defaults, bit numbering
  and runtime effects stay unverified. Other type kinds are not checked.
  The CLI lists findings only; occurrences inside their bounds appear as
  counts in `sourceCandidateTotals`, not as rows.
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

## Source extension: distribution resources (1 November 2026)

The format-5 inspector now associates canonical declarations with literal
adjacent-XML-stem payload paths and joins exact consumers in original namespace
and manufacturer scopes. It produces bounded decoded/re-encoded PNG previews
and a stdout-only offline HTML view behind `--output-format html`; JSON remains
the default. The [contract](../contracts/distribution-inspection.md) specifies
source provenance, serialization, budgets and exact fallback reasons.

This extends the historical "no baggage association" statement above only for
distribution inspection. `nativePackageBaggageNotPerformed` remains explicit;
no native package, persistence, installation or writer policy changes. ZIP help
and IconFile archives remain directory metadata, never extracted or rendered.
Only single-frame PNG is previewed; other media, malformed PNGs, animations and
budget refusals have named fallbacks. Structural/report limits still refuse the
whole inspection; optional media limits instead retain a per-resource fallback.
HTML sampling is visible and does not delete JSON facts.

Owning API/CLI tests exercise the positive image path and hostile/failure
controls; independent review and an offline real-browser proof are accepted.
Release-build measurement over all 37 OpenKNX releases (offline, read-only originals) passes 37/37 with 116 declarations and 229 consumers; the largest report grows by 451,125 bytes to 212,187,100 bytes (79.05% of the unchanged 256 MiB cap). ETS rendering parity is not claimed.
