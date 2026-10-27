# Report languages

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-048)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

<a id="48-project-documentation-export-renders-in-one-language-only"></a>
## 48. Project documentation export has English/German chrome but not a complete prose catalogue — partially resolved 2026-09-23 (T14)

**Current state.** `ReportOptions::language` selects English or German, sets
the HTML `lang` attribute, localizes the document title and primary navigation
and asks product-database queries for the same locale. The HTTP preview/export
API accepts `en`, `en-US`, `de`, or `de-DE`.

**Cause.** Product strings can reuse the existing language-aware queries, but
report-owned prose is not part of the web message catalogue and must remain
available to CLI callers. T14 added the two supported built-in locales without
coupling the crate to frontend language packs.

**Impact.** English and German reports use localized primary chrome and
product strings. Remaining detailed English prose can still appear in either.

**Remaining limitation.** Detailed table labels, enum/debug values and several
diagnostic sentences are still English. No third language or external report
language pack exists. The web documentation dialog (CT-2) requests the
report in the UI language (`de` for German, `en` otherwise); a separate report
language selector is still absent in the web dialog. `knx doc-export --language
en|en-US|de|de-DE` (2026-10-27) exposes the same two renderer languages natively.
<!-- END retained catalogue record -->

## Update 2026-10-14: the German report labels its tables too (Goal B8)

The *Remaining limitation* above named detailed table labels. Since Goal B8 the
German documentation report also translates the project header rows, the summary
counts, topology/building/group-address subheadings and empty states, the device
table rows (address, description, commissioning, references, manufacturer,
product, application program), the parameter/module-argument and communication
object table headers (flags as in the German ETS: L/S/Ü/A/K/I) and the orphan
table (`knx-report` `render.rs`; test
`german_report_labels_tables_and_headings_but_keeps_raw_values`). Still English,
on purpose: stored enum values (`ThreeLevel`, `Editing`, link directions) and
references, which are raw identifiers, and the diagnostic sentences that carry
raw references (*product database did not resolve …*, *… raw value shown*),
the KL-066 boundary. No third language, no separate report language selector.

## Update 2026-10-27: CLI language selector

`knx doc-export <store.knxdb> <out.html> [--language en|en-US|de|de-DE]` maps the
HTTP spellings onto the existing `ReportLanguage` and passes it through the same
`report_options` call, so German also asks the product database for German text.
Default stays English; arguments are validated before the store or output is
opened. This is the existing two-language renderer, not report parity and not a
new translation catalogue; no third language, sections option or PDF.
