# ADR-0110 — Retained raw-header candidates, not RTF interpretation

- **Status:** Accepted, bounded implementation; final admission pending
- Status update (2026-10-13): on main since 2026-10-10, checkpoint `68922cc4`; ledger KL-175 DONE. The line above is kept as written; it predates the integration.
- **Date:** 2026-10-10
- **Owners:** application/import report and session-log presentation

## Context and evidence boundary

KL-175 originally preserved rich-looking values without distinguishing raw text
from plain descriptions. F09 is integrated (`367405f1` ancestor). The existing
`opaque::from_retained_attribute` stores the complete decoded logical string
as UTF-8 bytes; original project XML is retained separately. No verified complete
ETS rich-text field catalogue or field/format validation contract was available.

The owner explicitly chose: RTF-like values as unchanged raw text, **no field or
format validation and no plain-text extraction**. This is a product boundary,
not an ETS/RTF conformance claim. No proprietary format semantics are invented.

## Decision

1. At the infrastructure report boundary, only `RetainedAttribute` values with
   valid UTF-8 and the exact common-looking prefix `{\rtf` immediately followed
   by an ASCII digit receive optional `raw_text_preview` metadata. This project
   defines that detection rule; it does not verify an RTF version or body.
   Malformed/incomplete bodies can be candidates, intentionally.
2. Preserve the full logical attribute value, owner, digest and original XML
   without changes. The optional report preview keeps literal control words,
   escapes and markup, at most 512 Unicode scalar values, safely on UTF-8
   boundaries. `truncated` explicitly distinguishes a preview from full data.
   This is prefix clipping, not extraction, parsing, validation or rewriting.
3. Preserve the pre-existing opaque hash/size log row. Add a separate info row
   `import:opaque:raw-text`, with explicit candidate/not-validated/not-interpreted,
   original-retained and, when applicable, preview-truncated disclosures.
4. UI is a thin view: localized en/de raw labels and literal React `<pre>` text,
   wrapping/scroll bounds, no HTML execution or editing. Raw server messages and
   exported logs remain raw evidence, not translated engineering values.
5. No new dependency, domain field, native model migration, RTF library or
   renderer. `has_losses()`, unknown reporting and capability warnings unchanged.

## Deliberate limitations

- Only already retained attribute values are classified. Modeled strings,
  retained whole elements/XML, opaque files and manufacturer/product text are
  not retroactively inspected or assigned rich-text field semantics.
- Leading whitespace/BOM, alternative capitalization and other header styles
  are outside the rule. A missing tag does not establish text without formatting.
- No complete field catalogue, format validation, plain extraction, markup
  interpretation or ETS editing/roundtrip compatibility claim.
- A preview may end inside a control sequence. Full source remains authoritative;
  the clipped preview must never be used to regenerate or write a project value.

## Verification

Synthetic service/native, clip/UTF8/malformed/content-kind, HTTP/project-unchanged
and literal UI cases; opt-in private boolean native/preview comparison with
missing-file/env refusal; real Chromium en/de narrow/wide. Final source-bound
quality/corpus/repository admission and evidence receipt remain required.
