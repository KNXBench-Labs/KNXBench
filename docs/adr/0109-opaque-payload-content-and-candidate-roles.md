# ADR 0109: Opaque payload content is evidence, not authenticated App identity

Date: 2026-10-10
Status: Accepted (owner-approved bounded scope)
Amends: ADR-0042 for project-import report projection only

## Context and verified sources

The existing product adapter `sniff_media` already classifies bytes as PNG,
JPEG, GIF, BMP, PDF, ZIP, PE executable, OLE2 compound, XML, empty or unknown.
KL-174/F08 already applied that classification to manufacturer baggage in the
application import report. Project UserFiles/AddinData lacked equivalent evidence.

Registered official KNX Association references (2026-10-10):

- [Install ETS6 Apps](https://support.knx.org/hc/en-us/articles/4402814860434-Install-ETS6-Apps): an official-domain indexed excerpt supports the `.etsapp` naming hint.
- [App validation rules](https://support.knx.org/hc/en-us/articles/360001508820-App-validation-rules): registered reference only; current direct text retrieval and the exact-article indexed AddInData query did not provide verified text.

**Retrieval correction:** earlier candidate notes overstated full-page retrieval
and page update dates. The configured extractor is search-only; browser retrieval
was blocked by an existing profile lock, which was not disturbed. Only the
Install indexed excerpt is verified here. No page update dates, full contents,
App manifest/signature validation algorithm or complete Add-in semantics were
verified. The supplied review/location names and owner-approved candidate scope,
not an invented proprietary format contract, define the descriptive roles.
Nothing here installs an App or infers validity from ZIP/PE magic.

The owner chose **unverified role candidates, separate content classes, explicit
retention of unknown bytes** rather than guessing complete ETS-App/Add-in support.

## Decision

1. Reuse `knx_productdb::sniff_media` at the application/report boundary; no
   new byte recognizer, archive parser, dependency, normalized model field,
   native/product schema version or destination change.
2. Annotate whole-member opaque sources for project UserFiles/AddinData,
   manufacturer Baggage and `.etsapp`-named members. XML subfragments are not
   reclassified as files. Role-hint matching is case-insensitive description
   only; it never changes exact source-path/owner resolution.
3. `.etsapp` gives **ETS-app-named candidate (identity unverified)**;
   AddinData location gives **add-in-state candidate (semantics unverified)**.
   Both hints may coexist. Neither alters the separately sniffed content class:
   a non-ZIP `.etsapp` remains `unknown`, a PNG-named PE remains `pe-executable`,
   a DLL-named image remains `png`, weak `MZ` text remains `unknown`.
4. Preserve original kind, path, bytes and source ownership. Retain unknown,
   malformed or misleading payloads byte-exact. Describe them as uninterpreted,
   not rendered or executed, with contents not unpacked. Do not read nested
   archive contents, manifests or application binaries.
5. Surface classification in both opaque summary and unsupported diagnostics,
   reaching existing HTTP session-log messages. Replace only the parser's known
   generic manufacturer opaque-payload notice; preserve independent same-member
   capability/validation diagnostics. Repeated annotation is idempotent.
6. This does **not** alter ADR-0042's existing standalone product-package
   baggage inventory or directory-only nested-ZIP measurement. Project-import
   evidence only calls the existing byte sniffer, not that inventory reader.

## Verification and boundaries

Synthetic service imports run with and without a product DB, exercise misleading
names and raw ZIP/PE/image/unknown bytes, assert exact native/product retention
and native Save/Reopen. HTTP import plus `/api/log` checks the same warnings.
An independent-diagnostic unit regression prevents overwriting unrelated refusal
text. Mutations that omit enrichment or invent verified App identity are killed
by assertions, not compiler failures, and restored sources pass.

No authenticity, manifest/schema validity, licensing, Add-in-state semantics,
media rendering, execution, native App support, ETS compatibility or hardware
behavior is claimed. Persisted historical reports are not silently rewritten;
a fresh import supplies the new description. Core/parser-only entry points keep
existing generic reports; the application's real service performs enrichment.
