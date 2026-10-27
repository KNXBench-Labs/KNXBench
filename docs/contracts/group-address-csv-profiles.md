# Group-address CSV import profiles

## Scope

`parse_group_addresses` (`crates/knx-csv`) stays the single import entry point. Besides the
native KNXBench v1 format ([IMPORT_EXPORT.md](../IMPORT_EXPORT.md) section 11), it recognizes
four fixed, complete header shapes. This is a bounded, **address-only** import adapter:

- It creates and updates group-address leaves through the existing planner and commands.
- It never creates, updates, deletes or renames group ranges and is not a hierarchy-preserving
  roundtrip.
- There is no export profile. The native writer is unchanged.
- It makes no claim of interoperability with any particular application. Only the shapes below
  are supported; neighbouring variants are refused, never guessed.

Native format behavior is unchanged: arbitrary column order, `,`/`;`, optional BOM, optional
columns, `Action`/`NewAddress`, read-only validation, duplicate-header errors and row line
numbers.

## Framing

UTF-8 text, optional leading BOM, CRLF or LF, separator `,` or `;` auto-detected from the
header line, RFC 4180 quoting. Tabs, UTF-16, legacy code pages and spreadsheet-format guessing
are not supported; invalid UTF-8 is rejected by the callers before parsing.

## Recognized header shapes

Cells are compared case-insensitively after trimming outer whitespace. All columns must be
present, in this exact order:

| # | Columns |
| --- | --- |
| 1 | `Group name; Address; Central; Unfiltered; Description; DatapointType; Security` |
| 2 | `Group name; Main; Middle; Sub; Central; Unfiltered; Description; DatapointType; Security` |
| 3 | `Main; Middle; Sub; Address; Central; Unfiltered; Description; DatapointType; Security` |
| 4 | `Main; Middle; Sub; Main; Middle; Sub; Central; Unfiltered; Description; DatapointType; Security` |

In shape 4 the first triplet holds names and the second the address components. `Main`,
`Middle` and `Sub` may repeat only at those positions of the complete shape 4.

The required native `Name` and `Address` columns take precedence, preserving the
existing native format, including its reporting of extra columns as unknown.
Otherwise a header containing `Group name`, or a standalone `Main`/`Middle`/`Sub`
cell, must match exactly one shape. Reordered, duplicated, incomplete or extended
profile headers (including an added `Action`) are file-level errors. Headers with
neither native required columns nor profile markers retain native missing-column
diagnostics. This distinction is structural, not a guess from row contents.

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
| `Description` | any non-empty content, including whitespace only, is an error | same |
| `DatapointType` | blank = unspecified; else `DPT-N` or `DPST-N-N` with N plain ASCII digits (signs such as `+1` are refused), normalized (`DPT-01` becomes `DPT-1`), then compared by the existing read-only check with the type the project derives; never applied or inferred, so a new address with a type is refused | any non-blank value is an error |
| `Security` | empty accepted; exactly `Auto` accepted with a warning; everything else is an error | same |

The `Auto` warning reads `Security Auto is not imported; project security is unchanged`.
Security is never interpreted as secure or unsecure and no security field is added.

## Range rows

All supplied fields are validated first. A valid range row is skipped with a row warning that
the range name and address are not imported. Invalid range rows keep their errors.

## Outcome

Errors anywhere in the file suppress the whole command, as for native files. Leaf rows are
normal upserts (create, update or unchanged); `Action`-style readdress or delete cannot be
expressed.
