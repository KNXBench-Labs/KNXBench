# Legacy enum binary representation (28 October 2026)

## Bounded evidence

A read-only comparison used the production legacy reader and EX-IM parser,
then independently decoded the canonical converted XML's direct
`ParameterTypes` declarations. It matched 22 long-enum types and 87 values;
16 values carried a leading zero byte. Each paired byte sequence, declared
byte length and type bit width agreed. Nine synthetic observer controls
checked the comparison. An independent review found no blocking finding in
that bounded field comparison; it was not importer or migration acceptance.
Private inputs, extracted payloads and identifying manifests are not published.

Seven further long-enum types (30 values) had no independently matched
converted application identity. They do not establish conversion equivalence.
Three ordinary-enum source rows carried nonempty binary values while their
paired XML enumeration had no `BinaryValue`; their interpretation stays open.
Additional XML documents in baggage were not canonical ParameterType witnesses.

## Representation contract

- Legacy `parameter_list_of_values.BINARY_VALUE` is hexadecimal text in the
  measured long-enum witness, not XML Base64 and not an integer selector.
- For atomic type 5, accept nonempty even-length ASCII hex in either case,
  without whitespace trimming or a prefix. `BINARY_VALUE_LENGTH` must be
  nonempty ASCII decimal digits equal to the decoded byte count. The type's
  parsed bit size must equal that byte count multiplied by eight.
- Preserve the complete decoded byte sequence, including leading zeros, and
  write canonical standard Base64 to `parameter_type_enum.binary_value`, the
  existing XML-shaped column. Preserve `REAL_VALUE` as the separate selector.
- An invalid or absent long-enum binary value stays NULL and is diagnosed;
  the retained payload still owns its original source lexeme.
- Atomic type 4 is not covered by this mapping. A nonempty binary or length
  field, including a length-only field, is diagnosed and retained, never
  copied into the XML-shaped column. A length-only `0` is not evidence of a
  binary encoding; reporting it is deliberately conservative.

The same validated long-enum representation rule is implemented for other
legacy inputs, but this finite witness does not prove ETS equivalence for
unpaired programs or all legacy databases.

## Persistence and migration boundary

Product database v30 is a data-only successor to v29. It replays retained
legacy payloads with the fresh mapper only after checking the payload SHA-256,
namespace, program inventory and owner, relevant type shape and exact enum
keys, selectors, text and order. It fills NULL binary columns only. A conflict,
missing/corrupt source or ambiguous identity refuses that entire payload's
backfill and appends an explicit diagnostic; other payloads can continue.
Historical diagnostics and reports are not rewritten as new import evidence.

Reopening a successfully upgraded database preserves its mapped rows and
diagnostics. Re-ingesting an already stored payload changes no mapped rows or
diagnostics, although a new source name or original archive can add provenance.
A refused backfill is not silently retried by the
existing skipped-ingest path; restoring or reconstructing damaged historical
catalogue data is separate work.

This is field retention, not permission to program a legacy device. No
`source_xml` is invented; existing experimental source binding and image /
commissioning guards are unchanged. No bus, VM or user catalogue was used.

See [WP-03's scope](../open-work/WP-03-legacy-binary-enumerations.md),
[ADR-0094](../adr/0094-legacy-exim-product-files.md) and
[the earlier mapping record](legacy-vd-mapping.md).
