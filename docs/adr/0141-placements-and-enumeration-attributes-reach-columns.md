# ADR-0141: Placements and enumeration attributes reach columns; the report names the rest

Status: accepted.
Date: 2026-10-18.
Amends: [ADR-0081](0081-parameter-attributes-are-reported.md) (a union
member's own `Offset`/`BitOffset` is now stored, so it is no longer reported).

## Context

[Census 3](../research/2026-10-18-product-database-programmability-census.md)
(G10, G13) found the unknown-construct report wrong in both directions:

- **Over-reporting.** A parameter's own `Offset`/`BitOffset` (76,747 occurrences
  in census 3, on union members) and the `Parameter/Property` placement with its
  attributes (2,815) were reported as unknown. ADR-0081 kept the offsets reported
  on purpose: the report describes what the product database *stored*, and
  neither had a column, although the image builder reads both from the retained
  bytes.
- **Silence.** `Enumeration` attributes other than `Id`, `Value`, `Text` and
  `DisplayOrder` were neither stored nor reported (KL-128 already noted this for
  `BinaryValue`). Across the three local collections:

| `Enumeration` attribute | Occurrences | Programs |
|---|---:|---:|
| `BinaryValue` | 34,757 | 477 |
| `Name` | 1,587 | 13 |
| `Icon` | 559 | 15 |
| `PictureAlignment` | 126 | 3 |

- **[V]** Census 3 traced 70 refused `0705h` programs to a hidden 48-bit
  `BinaryValue` parameter placed on Device Object property 78
  (`PID_HARDWARE_TYPE`); neither its octets nor its placement were in a column.
- **[C]** `Property` placements in the collections use `ObjectIndex` (98,015) or
  `ObjectType` (27,988), with `PropertyId`, `Offset`, `BitOffset` (126,003 each)
  and `Occurrence` (39,298).

## Decision

1. **Product DB v25** adds typed columns. `parameter`: `own_offset` and
   `own_bit_offset` (the parameter's own attributes, without asserting more
   meaning than the schema name), and `property_object_index`,
   `property_object_type`, `property_id`, `property_offset`,
   `property_bit_offset`, `property_occurrence` for a `Property` placing the
   parameter or the union it belongs to. `parameter_type_enum`: `binary_value`,
   `name`, `icon`, `picture_alignment`, kept as their raw lexemes.
2. **The report keeps ADR-0081's rule**: an attribute that reaches no column is
   reported. Numbers that are not integers leave their column empty and stay
   reported, with the original value as the sample. `Enumeration` now reports
   every attribute outside its stored set. A `Property` anywhere other than
   directly inside a `Parameter` or a `Union` stays an unmodelled element, with
   its attributes reported. The targeted scheme-evidence rule for
   `Property/@Occurrence` (PDB-5, schemes 12/14 and package-level 21/23) no
   longer covers a placing `Property`, whose `Occurrence` is now a column; the
   same precedent as separator `Access` under ADR-0139.
   `SuffixText`, `DefaultUnionParameter` and `ParameterCalculation` stay
   reported: their meaning is not stored.
3. **Migration** (ADR-0020, v23's promotion pattern, generalised to several
   `(kind, name, host)` row kinds). Per program blob whose bytes still match
   their key, the columns are filled through the parser's own column rules, and
   every row of the promoted kinds retires. Attribute rows retire only on a
   placing `Property` (xpath suffix `Parameter/Property` or `Union/Property`). Then the v21 pass runs over the
   blobs carried forward and adds back what a current ingest still reports (a
   non-integer value; an unstored `Enumeration` attribute). Retiring everything
   and re-deriving the remainder makes a migrated database equal a fresh
   install. A blob that is not carried forward keeps its rows and gets a
   `PlacementBackfillError` record; the measured reports counting it become
   `unavailable`. Legacy EX-IM payloads are skipped, not failed.
4. Nothing downstream changes meaning: the image builder still refuses property
   placements (KL-007) and `BinaryValue` encodings, and the columns are not yet
   read by any consumer.

## Consequences

- Install reports shrink by the promoted rows and grow by the `Enumeration`
  attributes that were silent; corpus pins of unknown counts move with them.
- The legacy importer's `BINARY_VALUE` (KL-128) could fill `binary_value` too;
  not done here.

## Evidence

`crates/knx-productdb/tests/placement_columns.rs` (stored columns, remaining
rows, v24 rewind reproducing a fresh install, damaged blob),
`parameter_attribute_unknowns.rs` (the v20 -> v25 chain still reproduces a fresh
install), the private corpus gate, and a real-database migration from a v24
install to v25 compared with a fresh install (census-3 dossier, results section).
