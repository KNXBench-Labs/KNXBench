# Group-address description follow-ups (KL-038b)

Date: 2026-11-09. Decision record:
[ADR-0164 amendment](../adr/0164-group-address-description-and-csv-encoding.md#amendment--9-november-2026-editor-csv-removal-mcp).

## Delivered

- **Editor.** Web inspector field for a group address's description,
  `POST /api/group-address-description` (`null` removes, strings byte-exact,
  unknown fields refused), one undo step per change. The field writes only after
  the user typed and only when the text differs beyond CRLF/LF, so visiting an
  imported description never rewrites it; a CR-holding description shows a hint.
- **CSV removal.** `knx_csv::CsvImportOptions::clear_blank_descriptions` (CLI
  `--clear-blank-descriptions`, server `clearBlankDescriptions`, a toggle beside
  the CSV import in the group-address view; not a File menu row, whose short-window
  height budget it would overrun).
  Off by default, so the ADR-0164 "blank leaves unchanged" rule holds. With it, a
  blank cell on an existing address plans `SetGroupAddressDescription { None }`
  and a `ClearDescription` destructive change (no affected links), counted in
  `descriptions_cleared`; CLI, server and web previews require the confirmation
  token, which binds the option. Without a `Description` column the option is a
  file-level error and nothing is planned as cleared.
- **MCP.** `get_group_address` documents the description it returns (absent when
  none is known); `search` matches group-address descriptions.

## Verification

Synthetic tests only; no ETS, bus or device involved.

- `knx-csv`: removal with the option (written-empty included, kept and created
  rows untouched, one undo step), refusal without a column for native and
  name/address-only files, native readdress plus removal.
- `knx-server` HTTP: opt-in preview → confirmation → undo; crossed tokens between
  clearing and keeping previews refused; refusal without a column; the
  description route sets, keeps empty, removes, undoes and refuses unknown ids
  and fields.
- `knx-cli`: flag, printed removal, token bound to the flag, refusal without a
  column.
- `knx-mcp`: verbatim CRLF description, absent key without one, search hit.
- Web: editor (visit without write, CRLF hint, typed write, emptied removal,
  refused write reverts), CSV toggle (off by default, absent from the File menu, confirmation text,
  same choice on confirmation, disabled without project), API bodies.

## Follow-up for the next website stills

`apps/knx-web/e2e/release-media.shots.ts` still waits for the read-only
`.ga-description` row and is left unchanged on purpose: it captures the tagged
Alpha 20 server, and `website/media.json` pins its SHA-256 as the stills'
provenance. When stills are next captured from a release with this editor,
switch both description checks to `.ga-description-editor` with `toHaveValue`
(verified: the release-media capture passes 1/1 that way against this code) and
refresh the provenance together with the stills.

## Not claimed

No `Comment` field or column, no range descriptions, no rich-text editing, no
change to the native CSV format version or the `.knxproj` mapping.
