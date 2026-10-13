# Parameter-attribute reporting

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-156)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 156. Unstored `Parameter`/`ParameterRef` attributes are not reported

**Limitation.** The static program parser stores a fixed set of
`Parameter` attributes (`Id`, `Name`, `Text`, `ParameterType`, `Access`,
`Value`, `Suffix`) and of `ParameterRef` attributes (`Id`, `RefId`,
`DisplayOrder`, `Tag`, `Text`, `Value`, and since ADR-0080 `Access`). Any
other attribute on these two elements is neither stored nor written to
`ingest_unknown`, so the install report does not name it. The bytes stay in
the retained program blob; nothing is lost, but the report under-states what
was not interpreted. A synthetic probe (AR07, 2026-10-05) confirmed it for
`Parameter/@SuffixText`, `@InitialValue`, `@LegacyPatchAlways` and
`ParameterRef/@Name`, `@SuffixText`, `@InitialValue`; `ParameterValidation`,
by contrast, is reported as an unknown element with its attributes.

**Cause.** The `Parameter` and `ParameterRef` arms in
`crates/knx-productdb/src/parse/program.rs` insert their columns without the
`report_unknown_attrs` call the other modelled elements make. `Suffix` is
also not the corpus spelling: the 332 `OriginalData` programs use
`SuffixText` (55,060 times) and never `Suffix`, so the `suffix` column stays
empty (the separate `SuffixText` display gap is described with the
product-language limitations above).

**Impact.** Install reports and the manufacturer report omit these
attributes. Corpus attribute names seen and not stored (332 programs):
`Parameter` — `SuffixText`, `DefaultUnionParameter`, `InitialValue`,
`LegacyPatchAlways`, `BaseValue`, `InternalDescription`, and the union
member placement `Offset`/`BitOffset` (the download image builder reads
those two from the retained bytes itself); `ParameterRef` — `Name`,
`SuffixText`, `InitialValue`, `InternalDescription`,
`ForbidGrantingUseByCustomer`.

**Lifted when.** Both arms report what they do not store, and a schema
migration re-derives the affected unknown rows and install reports the way
v18 did for `Channel/@Number`, with the corpus matrix re-pinned. Ledger row
`KL-156`.

**Update 2026-10-05 (KL-156, [ADR-0081](../adr/0081-parameter-attributes-are-reported.md)):
lifted.** Both arms now report every attribute outside their stored set,
through one helper the v20 -> v21 backfill shares; schema v21 adds the rows to
existing databases and re-derives measured install reports and
`package.unknown_count` (a blob that cannot be re-read is named and its
reports become `unavailable`). Nothing is newly *interpreted*: `SuffixText`,
`InitialValue`, union placement and the other names above are reported, not
stored. Evidence: `parameter_attribute_unknowns.rs`, the corpus matrix
re-pin, and a corpus probe comparing migrated and fresh installs.
<!-- END retained catalogue record -->
