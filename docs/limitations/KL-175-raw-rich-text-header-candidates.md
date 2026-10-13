# Raw rich-text header candidates

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-175)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->


<a id="175-rich-text-attribute-values-are-kept-verbatim-not-rendered"></a>
## 175. Rich-text header candidates are kept raw, not validated or rendered

**Historical limitation.** Rich-looking attribute values were preserved without
an explicit raw-format label. The original wording incorrectly called
`InstallationHints` unknown; it is recognized schema vocabulary, unmodeled and
retained. No surface renders RTF.

**Owner-approved bounded follow-up (2026-10-10).** No complete verified rich-text
field catalogue is available. The owner selected unchanged raw-header candidates,
no field/format validation and no plain-text extraction (ADR-0110). Only already
retained attribute UTF-8 values starting with `{\rtf` plus an ASCII digit get an
optional literal preview: at most 512 Unicode scalars, explicit `truncated`, full
logical value/owner/digest and original XML unchanged. Malformed bodies can be
candidates; this is not an RTF version/validity or ETS editing claim. Existing hash
log rows remain; additive raw rows say not validated/interpreted and original
retained. UI labels en/de and renders literal text, never HTML/RTF or edits.

Code checkpoint `68922cc4` includes source feature and main integration. Named
synthetic unit/service/native/HTTP/UI focuses, original boolean raw-preview/native
comparison and missing-env refusal pass. Real Chromium en/de at400/1440 passes,
with inspected screenshots; two omission mutations assertion-killed/restored.
Frozen source `d8cde545` admitted 2026-10-11: Rust2403/0/182, Web2612/0,
production build, strict/repository/documentation gates, nonempty all-pass corpus
and explicit original reconciliation/RefId/raw-preview gates pass; original bytes
unchanged. Bounded raw-label scope delivered/read back `6ef0b2d8` on
2026-10-11; remaining interpretation/rendering is separate scope.

**Remaining boundary.** Modeled strings, whole retained XML/elements, file bodies,
leading whitespace/BOM/alternative headers and older logs are not reclassified.
Absence of a marker does not mean plain text. Full retained source, not a clipped
preview, remains authoritative for preservation/export. No full field catalogue,
format validation, plain extraction or RTF rendering support. Ledger `KL-175`.
<!-- END retained catalogue record -->
