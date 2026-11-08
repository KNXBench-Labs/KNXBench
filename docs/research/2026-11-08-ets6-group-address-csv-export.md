# ETS6 group-address CSV export observations (8 November 2026)

Clean-room observation under the [evidence routes](../RESEARCH.md#evidence-routes).
The project added a few group addresses to its own fictional ETS6 6.4.1 project
(Demo licence) and exported them with ETS's "export group addresses" command in
every offered combination. The 40 CSV files, two XML files and the project
export stay private. The group addresses varied in their non-ASCII names
(German umlauts, `ß`), a description containing a semicolon and double quotes,
missing and present datapoint types, and two main groups with three middle
groups.

## The dialog

- **[V]** Output formats: `XML`, `XML (ETS4 Format)`, `CSV`, `CSV (ETS3 Format)`.
- **[V]** CSV layouts: `3/1`, `1/3`, `1/1` and `3/3`. The first figure is the
  number of name columns (one name, or Main/Middle/Sub names) and the second the
  number of address columns.
- **[V]** Options: export with a header row (checkbox), and a separator of tab,
  comma or semicolon. The dialog offers no encoding or quoting choice.

## The files

- **[V]** CSV is single-byte text without a byte-order mark and with CRLF line
  ends. The non-ASCII characters were written as one byte each (`ä` as 0xE4,
  `ß` as 0xDF), so the file is not UTF-8. These characters cannot tell
  Windows-1252 from ISO-8859-1; the system code page of other Windows locales
  was not observed. Both XML exports are UTF-8 with a byte-order mark.
- **[V]** Every cell is quoted, and embedded quotes are doubled. A `;` inside the
  quoted description stayed literal in all three separator variants; other
  separators inside cells were not tried. The tab, comma and semicolon
  variants of one layout differ only in the separator byte.
- **[V]** Without the header option, the file is the header file minus its first
  line.
- **[V]** The `CSV` header rows are, with `;` shown as separator:
  - 1/1: `Group name;Address;Central;Unfiltered;Description;DatapointType;Security`
  - 1/3: `Group name;Main;Middle;Sub;Central;Unfiltered;Description;DatapointType;Security`
  - 3/1: `Main;Middle;Sub;Address;Central;Unfiltered;Description;DatapointType;Security`
  - 3/3: `Main;Middle;Sub;Main;Middle;Sub;Central;Unfiltered;Description;DatapointType;Security`

  These are exactly the four full shapes of the
  [import profiles](../contracts/group-address-csv-profiles.md), which until now
  had not been checked against a real file.
- **[V]** `CSV (ETS3 Format)` writes the same four layouts with only the name and
  address columns (for example `Group name;Address`). It has no flag,
  description, datapoint type or security column.
- **[V]** Main and middle groups get their own rows, before their children. A
  composite address is written as `0/-/-` or `0/0/-`, and split cells are left
  empty below the range's depth. Names in the triplet layouts sit only at the
  row's depth.
- **[V]** Datapoint types are written as `DPST-<main>-<sub>`, and empty when
  none is set. Security is `Auto` on every row, ranges included. `Central` and
  `Unfiltered` were empty because no flag was set; their non-empty spelling was
  not observed.

## Consequences for the importer

Before this note, the importer refused every one of the 40 files, all at the
UTF-8 check. After converting them to UTF-8, the remaining refusals were:

- tab-separated files (separator not recognized);
- `CSV (ETS3 Format)` headers (not one of the four shapes);
- a non-empty `Description` (the project model has no group-address
  description, so the contract refuses rather than dropping it);
- a datapoint type on an address that does not exist yet (the contract never
  assigns a type).

Files without a header stay refused by design: nothing in the data rows says
which layout they use. The [contract](../contracts/group-address-csv-profiles.md)
records what changed in response.

**Follow-up (8 November 2026, KL-038b).** Owner decisions recorded in
[ADR-0164](../adr/0164-group-address-description-and-csv-encoding.md) lift the
remaining refusals: the import encoding is now an explicit choice (Windows-1252
reads these files), descriptions are modelled and applied, and a type on a new
address becomes its declared type.

## Limits

One ETS version, one locale and one small project. No flag values, no
two-level or free addresses, no XML import. It says nothing about other
applications' CSV files.
