# Spreadsheet transformations

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-041)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

<a id="41-a-csv-file-saved-from-excel-under-a-german-locale-may-still-surprise-a-user"></a>
## 41. German-locale separators are supported; unverified spreadsheet transformations remain

**Limitation.** The importer auto-detects `,` and `;` as the field
separator per file, specifically because Excel's own CSV export/import
behavior depends on the OS list separator setting: under a German
(or otherwise comma-decimal) locale, Excel writes `;`-separated CSV and
expects `;` back on open, while under an English locale it uses `,`. Both
are accepted here. What is not handled is everything else Excel can do
to a file beyond the separator — most notably re-saving with an unsupported
encoding or altering address/boolean cells during a manual edit. RFC 4180
quoting, embedded separators/quotes/newlines, UTF-8 BOM, CRLF/LF and mixed
line endings all have direct regression tests; they are not part of the
remaining limitation.

**Cause.** The separator auto-detection in `crates/knx-csv/src/read.rs`
covers the one Excel behavior this project could concretely name and test
against (`parses_the_same_file_semicolon_separated`). Excel's broader
locale-dependent quirks are not enumerated anywhere in this codebase or
its research, and guessing at more of them without a concrete failing
sample would be exactly the kind of unverified assumption CLAUDE.md rules
out.

**Impact.** The known German-locale separator difference is covered and an
unchanged export/import round trip is tested. A user who hand-edits an exported
file in Excel and hits an import error on a cell Excel silently reformatted
should not assume the importer is broken — it is a known category of risk
with this specific tool, not a claim that every Excel edit is safe.

**Lifted when.** A concrete Excel-induced parse failure is reported with a
reproducing file, at which point it becomes a specific, testable case
rather than a general caution.
<!-- END retained catalogue record -->
