# RTF-bearing text — bounded adapter and editing contract

## Evidence and current package

Project Schema23 v01.00.00 §1.2.5.1 p.44 explicitly permits plain or RTF text
for `DeviceInstance/@InstallationHints`. `xs:string` and formatted ETS comments
alone do not establish RTF serialization for every text field.
Microsoft RTF 1.9.1 (19 March 2008), Basic Entities/Reader/Syntax/Unicode
pp.7–16, defines scoped groups, case-sensitive control words, escaped characters
and Unicode fallback. This is a supported-subset design, not an ETS dialect claim.
Primary public copies retrieved and text-inspected on 10 October 2026:
[Schema23](https://support.knx.org/hc/nl/article_attachments/17389755651474),
[Microsoft RTF 1.9.1](https://officeprotocoldoc.z19.web.core.windows.net/files/Archive_References/%5bMSFT-RTF%5d.pdf).
The charset subset is deliberately conservative: font tables/font selection
make non-ASCII codepage bytes ambiguous and block their preview rather than
inventing glyphs. Unicode controls still provide an independently decoded view.
Plain values beginning exactly with `{\rtf` are RTF candidates; other plain
values remain literal. ASCII source CR/LF inside RTF are ignored; paragraph and
line controls both produce LF. Therefore writing promises text/basic-style
semantics, not preservation of paragraph-vs-line structure or original encoding.

This first AP2 subpackage owns only `crates/knx-etsproj/src/rtf.rs`, its `lib.rs`
module declaration, `crates/knx-etsproj/tests/rtf.rs`, this contract and its own
handover/log. A native preservation witness additionally extends the existing
`crates/knx-app/tests/import_integrity_roundtrip.rs` (tests only; no foreign
active modifications found before ownership). No importer/storage change or
new test-only dependency edge. The parser belongs to the external-text adapter, not KNX Core.
No manufacturer heuristic, protocol, native schema, UI shell or new dependency.

## Adapter contract

- Input is a borrowed original logical string; it is never mutated. Original
  XML/attribute bytes retain their existing separately preserved identity.
- Plain text is a different format. RTF candidates must validate their header,
  braces, tokens, supported Unicode and budgets before yielding a view.
- A view contains literal text runs and only bold/italic/underline flags;
  derived preview is never the saved original.
- A bounded subset covers groups, ASCII/escaped literal syntax, paragraphs,
  lines/tabs, Unicode signed UTF-16 code units and scoped `uc` fallback,
  explicit Windows-1252 escapes, and basic formatting/reset.
- Unknown controls/destinations are explicitly listed. Unsafe/unsupported
  destinations (including objects, pictures and fields) are not interpreted,
  executed or fetched. Binary payloads and unverified codepages refuse preview.
- Limits are adapter budgets, not claims that longer KNX fields are invalid:
  1 MiB source, 64 KiB emitted text, 64 nested groups, 100,000 tokens, 256
  distinct unsupported constructs, 4,096 runs, Unicode fallback <=16, control
  names <=32 letters and numbers <=10 digits. Failure leaves the caller's
  original unchanged.
- Unknown formatting blocks supported-subset rewriting. Untouched data stays
  byte/logical-string exact; raw editing or explicit conversion requires the
  user's deliberate choice and a separate persistence admission contract.
- Canonical writing emits only the supported subset, escapes syntax and
  non-ASCII UTF-16 units, validates budgets and never claims original-byte or
  ETS export equivalence. Do not serialize an unsupported preview as a document.

## Acceptance and remaining work

Named RED-first adapter tests cover plain text, Unicode/surrogates/fallback,
escapes, paragraph/style/group restoration, unknown and ignorable destinations,
unsafe payloads, malformed roots/tokens, codepages, budgets and canonical
semantic roundtrips. Expanded adapter target: 22 executed tests. The existing
native roundtrip target includes one dedicated witness with five synthetic
Schema23 inputs (plain Unicode, supported RTF, partial/unknown RTF, malformed
RTF, output-budget refusal). It asserts exact retained logical attribute and
whole XML bytes, nonempty imported model, repeated saves, close/reopen and full
model/opaque equality. Its first attempt refused on `NotSaved` because the test
omitted the caller's explicit model save; the corrected fixture passes. No
production importer/storage change was needed.

This subpackage does not provide edited-content persistence or a visible
application feature. AP2 remains open for original-bound editing, application
commands/history, edited-value Save-Reopen and isolated editor/browser acceptance.
Session1 owns shell/navigation/catalogue integration; supply a precise handoff.
