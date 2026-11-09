# Group-address CSV import profiles

## Scope

`parse_group_addresses` (`crates/knx-csv`) stays the single import entry point. Besides the
native KNXBench v1 format ([IMPORT_EXPORT.md](../IMPORT_EXPORT.md) section 11), it recognizes
eight fixed, complete header shapes. This is a bounded, **address-only** import adapter:

- It creates and updates group-address leaves through the existing planner and commands,
  including their description and, for addresses it creates, their declared datapoint type
  ([ADR-0164](../adr/0164-group-address-description-and-csv-encoding.md)).
- It never creates, updates, deletes or renames group ranges and is not a hierarchy-preserving
  roundtrip.
- There is no export profile. The native writer only gained the `Description` column
  (IMPORT_EXPORT §11).
- Only the shapes below are supported; neighbouring variants are refused, never guessed. The
  shapes match the header rows ETS6 6.4.1 writes in its `CSV` and `CSV (ETS3 Format)` group
  address exports, observed on one fictional project
  ([research](../research/2026-11-08-ets6-group-address-csv-export.md)). That is a checked
  header match, not a general interoperability claim.

Native format behavior is otherwise unchanged: arbitrary column order, optional BOM, optional
columns, `Action`/`NewAddress`, read-only validation, duplicate-header errors and row line
numbers. One edge case changed with tab detection: a header whose unquoted tabs outnumber its
commas and semicolons (for example `Address,Name` followed by two stray tabs) is now read as
tab-separated and refused for missing columns, where it was read as comma-separated before.
Nothing is imported in that case.

## Framing

The user chooses the file's encoding explicitly: **UTF-8** (the default) or **Windows-1252**.
Nothing is detected or guessed, because the same bytes mean different characters in different
Windows locales. This applies to native files too.

- UTF-8: optional leading BOM. Invalid UTF-8 is refused before parsing with the offset of the
  first invalid byte; the message names the Windows-1252 choice and conversion as the two ways
  forward.
- Windows-1252: decoded with the complete table of the Unicode Consortium's Microsoft mapping
  (`CP1252.TXT`): `0x00`–`0x7F` and `0xA0`–`0xFF` are the code points of the same value,
  `0x80`–`0x9F` the table's 27 defined characters (for example `0x80` is `€`). The five bytes
  that table leaves undefined (`0x81`, `0x8D`, `0x8F`, `0x90`, `0x9D`) are refused with their
  offset, never mapped to control characters. A file that starts with a UTF-8 BOM is refused
  under Windows-1252 because the choice contradicts the file's own marker.

After decoding: CRLF or LF, separator `,`, `;` or tab auto-detected from the header line (the
most frequent unquoted one; a tie goes to `,`, then `;`), RFC 4180 quoting. UTF-16 and other
code pages are not supported. ETS6 6.4.1 writes its CSV export in a single-byte code page whose
umlauts match Windows-1252 (`ä` = 0xE4); other Windows locales were not observed.

Surfaces: CLI `knx ga-import … --encoding utf-8|windows-1252`, server
`POST /api/group-addresses/csv-import` body field `encoding` (optional, default `utf-8`; any
other value is a `400`), and a select next to the web import action. Labels are compared
ASCII case-insensitively; no alias such as `cp1252` or `latin1` is accepted. A destructive
preview's confirmation token binds the raw file bytes and the chosen encoding, so a token from
one encoding cannot confirm an import in the other. The MCP tool `validate_ga_csv` receives
text, not bytes, so it has no encoding choice.

## Recognized header shapes

Cells are compared case-insensitively after trimming outer whitespace. All columns must be
present, in this exact order:

| # | Columns |
| --- | --- |
| 1 | `Group name; Address; Central; Unfiltered; Description; DatapointType; Security` |
| 2 | `Group name; Main; Middle; Sub; Central; Unfiltered; Description; DatapointType; Security` |
| 3 | `Main; Middle; Sub; Address; Central; Unfiltered; Description; DatapointType; Security` |
| 4 | `Main; Middle; Sub; Main; Middle; Sub; Central; Unfiltered; Description; DatapointType; Security` |
| 5 | `Group name; Address` |
| 6 | `Group name; Main; Middle; Sub` |
| 7 | `Main; Middle; Sub; Address` |
| 8 | `Main; Middle; Sub; Main; Middle; Sub` |

Shapes 5–8 are shapes 1–4 without the five metadata columns. Their rows carry no flags,
description, datapoint type or security, so leaves behave as if those cells were blank and no
column is reported as ignored. In shapes 1–4 only `Security` is reported as an ignored
(read-only) column; `Description` and `DatapointType` can be applied. In shapes 4 and 8 the first triplet holds names and the second
the address components. `Main`, `Middle` and `Sub` may repeat only at those positions.

The required native `Name` and `Address` columns take precedence, preserving the
existing native format, including its reporting of extra columns as unknown.
Otherwise a header containing `Group name`, or a standalone `Main`/`Middle`/`Sub`
cell, must match exactly one shape. Reordered, duplicated, incomplete or extended
profile headers (including an added `Action`) are file-level errors. Headers with
neither native required columns nor profile markers retain native missing-column
diagnostics. Header-less files therefore stay refused: their first data row is read as a
header. This distinction is structural, not a guess from row contents.

Profiles require a `ThreeLevel` project; `TwoLevel` and `Free` projects are refused.

## Addresses

- Composite cell: `n/n/n` is a leaf; `n/-/-` is a main range; `n/n/-` is a middle range.
- Split cells: three digits is a leaf; `n`, empty, empty is a main range; `n`, `n`, empty is a
  middle range.
- Rejected: gaps, hyphens in split cells, mixed hyphens, non-numeric or out-of-range components
  and the broadcast leaf `0/0/0`. Numeric validation uses `GroupAddress::parse`.
- Values are never carried down from earlier rows.

## Names

Single-name shapes use `Group name`. Triplet shapes need exactly one non-blank name at the
depth of the address (Main for a main range, Middle for a middle range, Sub for a leaf); the
other two must be blank. Names are never concatenated or filled from context. Names are kept
byte-exact, including non-ASCII, delimiters and newlines; blank means empty after trimming and
is an error.

## Cells

| Column | Leaf row | Range row |
| --- | --- | --- |
| `Central`, `Unfiltered` | native boolean spellings; blank = default on create, unchanged on update | any non-blank value is an error |
| `Description` | kept byte-exact (separators, quotes, line breaks, non-ASCII); non-blank sets the address's description on create and on update; blank (empty after trimming) = none on create, unchanged on update; a cell can never clear a description | not imported; a non-blank cell adds the warning `group range description is not imported; only group-address descriptions are applied` |
| `DatapointType` | blank = unspecified; else `DPT-N` or `DPST-N-N` with N plain ASCII digits (signs such as `+1` are refused), normalized (`DPT-01` becomes `DPT-1`). On a row that **creates** an address the type becomes its declared datapoint type (provenance `UserEdit`). On an **existing** address it stays the read-only comparison with the type the project derives; a difference is a row error | any non-blank value is an error |
| `Security` | empty accepted; exactly `Auto` accepted with a warning; everything else is an error | same |

The `Auto` warning reads `Security Auto is not imported; project security is unchanged`.
Security is never interpreted as secure or unsecure and no security field is added.

## Range rows

All supplied fields are validated first. A valid range row is skipped with a row warning that
the range name and address are not imported, plus the range-description warning above when its
`Description` cell is non-blank. Invalid range rows keep their errors.

## Outcome

Errors anywhere in the file suppress the whole command, as for native files. Leaf rows are
normal upserts (create, update or unchanged); `Action`-style readdress or delete cannot be
expressed. A description change is its own undoable command
(`SetGroupAddressDescription`) inside the import's single batch, so a row that only changes
the description does not rewrite the name or flags. A blank `Description` cell leaves an
existing description unchanged; with the explicit *clear blank descriptions* option it
removes it as a confirmed destructive change, and the name/address-only shapes, which have
no `Description` column, are refused with that option
([ADR-0164 amendment](../adr/0164-group-address-description-and-csv-encoding.md#amendment--9-november-2026-editor-csv-removal-mcp)).
