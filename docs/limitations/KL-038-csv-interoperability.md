# CSV interoperability

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-038)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

## Address-only column profiles

The import adapter recognises four fixed ordered headers with either one name
column or separate hierarchy-name columns, and either one address column or
three numeric address components. See the exact
[contract](../contracts/group-address-csv-profiles.md). This improves bounded
spreadsheet exchange without changing the native CSV export or project schema.

Range rows are explicitly reported and not applied. Nonempty descriptions and
security tokens other than `Auto` refuse the entire import. `Auto` is reported
as not imported; it never becomes a guessed security value. DPT is validated
read-only, not assigned. Only three-level addresses and UTF-8 comma/semicolon
files are admitted in these profiles. Native CSV behaviour is unchanged.

This is not full hierarchy/metadata roundtrip, a new external export format,
ESF/OPC support or general application interoperability. The historical record
below describes the earlier native-format-only boundary.

<!-- BEGIN retained catalogue record -->

## 38. Group-address CSV export/import (T12) has no verified ETS interoperability

**Limitation.** "KNXBench group-address CSV v1" (`crates/knx-csv`,
[IMPORT_EXPORT.md §11](../IMPORT_EXPORT.md#11-group-address-csv-exchange)) is
a format this project defines and documents itself. It is not, and cannot
currently be shown to be, compatible with ETS's own "Export Group
Addresses" CSV feature, or with the legacy `.esf`/OPC export format.

**Cause.** No sample of either format exists anywhere in this repository,
and searching all 179 documents of the extracted KNX Standard v3.0.0
corpus for `csv`, `esf`, `OPC export`, and group-address-export
terminology turned up nothing but two incidental prose hits (a
data-security test report and an RF application note) — there is no
standardized group-address exchange text format at all. Group-address CSV
export is an ETS *application* feature, not something the KNX Association
specifies, so there is nothing to read except a real file, and none has
been obtained.

**Impact.** A file exported by KNXBench is not guaranteed to open sensibly
in ETS, and a CSV exported from ETS is not guaranteed to import cleanly
here — the importer is column-name-driven and separator-detecting
specifically so a foreign file has a *fair chance*, but that is a design
mitigation, not a tested claim. Nothing in the UI, CLI output, or this
documentation set may say "ETS CSV" or imply interoperability, and none of
it does.

**Lifted when.** A genuine ETS-produced group-address CSV export is
obtained. At that point, adding a second, ETS-shaped column profile to
`crates/knx-csv`'s reader is the stated upgrade path — header matching is
already isolated in one function (`map_headers`), so the profile would go
there rather than spreading through the parser, though that function is a
hard-coded `match` and would itself have to be edited. `.esf`
import is a separate, larger undertaking (writing a parser against
remembered syntax with no sample to check it against is exactly what
CLAUDE.md's "do not invent technical facts" forbids) and would need its
own task, gated the same way on first obtaining a real file.
<!-- END retained catalogue record -->
