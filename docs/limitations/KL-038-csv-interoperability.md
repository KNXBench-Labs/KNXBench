# CSV interoperability

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-038)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

## Address-only column profiles

The import adapter recognises eight fixed ordered headers with either one name
column or separate hierarchy-name columns, and either one address column or
three numeric address components, with or without the five metadata columns. See the exact
[contract](../contracts/group-address-csv-profiles.md). This improves bounded
spreadsheet exchange; the native CSV gained only an optional `Description`
column.

Range rows are explicitly reported and not applied, including a range's
description. Security tokens other than `Auto` refuse the entire import. `Auto`
is reported as not imported; it never becomes a guessed security value. Only
three-level addresses and files separated by comma, semicolon or tab are
admitted.

**Checked against real ETS output (8 November 2026).** ETS6 6.4.1 exports of a
fictional project ([research](../research/2026-11-08-ets6-group-address-csv-export.md))
have exactly these header rows. The files are a single-byte code page, not
UTF-8; the first round refused all 40 at the UTF-8 check, full `CSV` files
also on their descriptions and on types of new addresses.

**Owner decisions applied (8 November 2026, KL-038b,
[ADR-0164](../adr/0164-group-address-description-and-csv-encoding.md)).**

- The user chooses the import encoding explicitly: UTF-8 (default) or
  Windows-1252. Nothing is detected or guessed; the five undefined
  Windows-1252 bytes are refused with their offset. Every confirmation token
  binds bytes and encoding.
- Group addresses have a description (native schema 19). `.knxproj` imports
  (schema 20–23) map `GroupAddress/@Description`; CSV `Description` cells set it
  on create and update, blank leaves it unchanged.
- A profile `DatapointType` on an address the import creates becomes its
  declared type; for an existing address it stays a read-only comparison.

Remaining boundaries: header-less files stay refused (their layout cannot be
known); other code pages, UTF-16 and detection are unsupported; files saved
before native schema 19 keep imported descriptions only as retained source
evidence until the project is re-imported. Since 9 November 2026 a blank cell
removes a description only with the explicit, confirmed *clear blank
descriptions* option, and the web inspector edits descriptions
([ADR-0164 amendment](../adr/0164-group-address-description-and-csv-encoding.md#amendment--9-november-2026-editor-csv-removal-mcp)).

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
