# Checked Date text editor (WP-08)

## Bounded requirement

Offer the existing opt-in **Checked editor** for one source-authenticated Date
form. This is a project-value text editor, not a calendar picker or parameter
memory codec. The [existing value decision](../research/2026-10-28-typedate-value-form.md)
and server `is_schema_date` remain authoritative.

## Source admission

Reuse the existing ProductDB/source endpoint, host snapshot and source-identity
checks. Admit only direct Static `TypeDate Encoding="DPT 11"`, with no bounds
and no additional child attributes other than XML namespace declarations.
The parent admits `Id`, optional `Name` and namespace declarations. The exact
encoding token identifies the evidenced declaration; it does **not** establish
DPT 11 memory bytes, byte order, placement width or a 1990–2089 value restriction.
Absent/different/qualified encoding, extra hints, source conflicts and module
scopes keep the visible specialized fallback. The ordinary text field is not
newly forbidden. No parser, database or native-format migration is needed.

## Text and mutation contract

- Exactly ten ASCII characters `yyyy-mm-dd`, naming a day in the proleptic
  Gregorian calendar. Mirror the existing server's four-digit-year domain,
  including `0000` and `9999`; no timezone, browser date arithmetic, locale
  conversion, trimming, clamping or automatic repair.
- Use a text input, not `input[type=date]`. Show translated format guidance and
  invalid-date feedback. Null, empty and malformed historical values stay
  distinguishable; opening the editor never rewrites them.
- Apply sends the exact valid draft through the existing revision-bound
  parameter mutation. Render, blur, Enter, unchanged/invalid/restricted/stale
  drafts do not send a write. Existing reset, pending, refusal and late-result
  handling remain shared with the other checked editors.
- Preserve raw pasted line breaks as an invalid draft instead of silently
  accepting the browser's stripped replacement. A full selection replaces the
  entire raw draft. If raw and DOM offsets differ, partial paste leaves the
  draft unchanged and requests a complete replacement.

## Acceptance boundary

Pure admission/calendar tests, mounted component/host regressions and the
registered intercepted-HTTP browser spec cover the new presentation, explicit
mutation request, source/authority failures, raw paste and EN/DE wide/narrow
layout. Browser callbacks/history are synthetic and do not prove native
persistence or backend execution. Existing server validation and storage commands
are reused, not reimplemented or newly certified by those UI tests.

The maintained change-scoped package gate supplies final build/typecheck/unit/
browser/static acceptance. Its actual selected stages, exits and frozen-input
proofs belong in the delivery evidence, not an assumed workspace-wide test count.

No real bus, device, ETS, native WebKitGTK, image writer, release or deployment
acceptance follows. Date image generation stays refused by type; Colour/Picture
codecs, module mapping and broader WP-08/PDB-04 remain open.
