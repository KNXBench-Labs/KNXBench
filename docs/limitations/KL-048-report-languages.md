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
language selector is still absent.
<!-- END retained catalogue record -->
