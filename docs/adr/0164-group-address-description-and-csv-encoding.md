# ADR 0164: Group-address description, explicit CSV encoding and new-address types

Date: 2026-11-08
Status: Accepted

## Context

ETS6 6.4.1 group-address CSV exports
([research](../research/2026-11-08-ets6-group-address-csv-export.md)) match the
four full import profiles header for header, yet every real export was refused
([KL-038](../limitations/KL-038-csv-interoperability.md)):

- the files are single-byte text (`ä` = 0xE4), not UTF-8;
- a non-empty `Description` was refused because the project model had no
  group-address description;
- a `DatapointType` on an address the file creates was refused because the
  contract never assigned a type.

The owner decided all three on 8 November 2026
([OWN-01](../open-work/OWN-01-owner-decisions.md)).

`.knxproj` imports already met the same attribute: `GroupAddress/@Description`
(schema 20–23) was known but unmapped and therefore kept as a keyed retained
attribute in the opaque store; schema 11 does not list it, so there it was an
unknown retained attribute. A synthetic native-18 file written by the unchanged
v18 writer (`crates/knx-store/tests/fixtures/native18-before-ga-description.*`)
pins that behaviour.

## Decision

1. **Explicit encoding.** CSV import decodes the raw bytes in the encoding the
   user chooses: `utf-8` (default) or `windows-1252`. Nothing is detected or
   guessed. UTF-8 keeps the offset-naming refusal. Windows-1252 uses a small
   hand-written table in `knx-csv` that follows the Unicode Consortium's
   Microsoft mapping `CP1252.TXT`; its five undefined bytes (`0x81`, `0x8D`,
   `0x8F`, `0x90`, `0x9D`) are refused with their offset, and a leading UTF-8 BOM
   is refused as a contradicting choice. `encoding_rs` is only a transitive
   dependency and maps the undefined bytes to C1 controls (WHATWG), which the
   owner ruled out; 32 table entries do not justify a new direct dependency.
   Surfaces: CLI `--encoding`, server body field `encoding` (unknown value `400`),
   a web select beside the import action. Every confirmation token binds the raw
   bytes and the encoding; the CLI token for UTF-8 without the flag is unchanged.
2. **Group-address description.** `GroupAddressEntry::description:
   Option<String>`, byte-exact. `None` means no description is known; `Some("")`
   is one written empty. It is a plain entity attribute like
   `DeviceInstance::description`, not an `Override<Text>`: no product layer can
   supply a group-address description, and absent versus empty is already
   distinguishable. Native schema 19 stores it in a separate table
   `group_address_description` (one row per address that has one). A separate
   table, rather than a column, keeps the semantic identity of every project
   without descriptions unchanged, so recorded history baselines, undo/redo
   images and named versions stay valid, the rule earlier additive overlays
   followed. Changes go through the validated, undoable
   `Command::SetGroupAddressDescription`; creation carries it in the entry.
   `knx-diff`, `.knxproj` comparison, selective import matching, the projection
   (read-only `description`) and the documentation export include it.
3. **`.knxproj` mapping and opaque data.** The schema 20–23 parser takes
   `GroupAddress/@Description` into the field; it is no longer retained as an
   opaque attribute, so a fresh import holds it once. Schema 11 is unchanged.
4. **Migration 18 → 19 does not lift.** Lifting the retained attribute into the
   new table would change the saved root's semantic identity and mark every
   recorded workspace stale (history images hold no opaque data, so they could
   not be lifted alike). The migration only adds the empty table; the retained
   rows stay byte-exact. The field stays empty in such files until the project is
   imported again from its `.knxproj`. Should a user later set a description on
   such an address, the mapped field is authoritative; the old retained row is
   source evidence only and is never shown as the description.
5. **CSV application.** Profile shapes 1–4 and the native `Description` column
   apply a non-blank cell on create and update; blank means none on create and
   unchanged on update (as for flags), so a cell never clears a description.
   Range-row descriptions are reported and not imported. A valid profile
   `DatapointType` becomes the declared type (`Layer::UserEdit`) of an address
   the import creates; for an existing address it stays the read-only
   comparison. The native `DatapointType (read-only)` column never applies. The
   native format stays v1: the new column is optional and additive.

## Consequences

- Real ETS6 `CSV` exports with header import after choosing Windows-1252;
  header-less files stay refused by design.
- Older builds refuse a native-19 file as a newer schema, as before.
- A predecessor `.knxdb` shows no descriptions until re-imported; this is
  documented, not silently repaired.
- No editing UI for descriptions yet; the web inspector shows them read-only
  (superseded by the amendment below).
- Other Windows code pages, UTF-16 and automatic detection remain unsupported.

## Amendment — 9 November 2026: editor, CSV removal, MCP

The three follow-ups left open above are decided as follows; the field, its
storage and the migration rule are unchanged.

1. **Editor.** The web inspector edits a group address's description in a
   multi-line field through `POST /api/group-address-description`
   (`{ groupAddressId, description }`, unknown fields refused), which applies the
   existing undoable `SetGroupAddressDescription`. `null` removes the
   description; any string, empty included, is stored byte-exact. Because a
   textarea reports CRLF as LF, the field writes only after the user typed and
   only when the text differs ignoring that difference: visiting an imported
   description never rewrites it. A description holding CR shows a hint that an
   edit saves plain line breaks; emptying the field removes the description.
2. **Removal through CSV is an explicit, confirmed choice.** Decision 5 stays
   the default: a blank cell leaves a description unchanged, so re-importing an
   export never wipes one. The import option *clear blank descriptions* (CLI
   `--clear-blank-descriptions`, server body `clearBlankDescriptions`, a toggle
   beside the CSV import in the web group-address view) makes a blank or whitespace-only `Description` cell on an existing
   address remove its description (`Some("")` included). Each removal is a
   destructive change `ClearDescription` with no affected links, counted in
   `descriptions_cleared`, so the existing preview and confirmation apply: the
   CLI prints a token and writes only with `--confirm`, the server answers a
   preview with a token, and every token binds the option, so a preview with it
   cannot confirm an import without it, nor the other way round. A file without
   a `Description` column (native without one, the name/address-only profiles)
   cannot state a blank and is refused as a file-level error when the option is
   set. Creating an address with a blank cell still gives it none.
   The toggle is not a File menu row: that menu holds entries only, and in
   short windows its import submenu has no height to spare. The File menu
   import therefore keeps the safe default; the option resets after each
   import.
3. **MCP.** `get_group_address` documents the description it already returned
   through the projection (verbatim; absent when none is known), and `search`
   also matches group-address descriptions (`matchedIn: ["description"]`). The
   adapter stays read-only.
