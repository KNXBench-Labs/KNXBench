# WP-08: authentic Color/Time upgrade witness — 3 November 2026

## Result and boundary

A fictional package installed by actual product-database v14 code reproduces
an incorrect same-blob, duplicate-program promotion after the current public
opener migrates its database to v31. Two normalized type rows take their kind
from the losing program declaration. A fresh current installation of the
identical package bytes keeps the first declaration authoritative.

This completes the authentic-upgrade prerequisite recorded in
[WP-08](../open-work/WP-08-parameter-value-codecs.md), not a repair, parameter
codec, compatibility census or whole-WP-08 acceptance. No existing product
source changed. The released Color/Time pass was not rewritten.

[Compact observations and identities](../evidence/wp08-color-time-upgrade-2026-11-03.json)
accompany this record. Complete original databases, source manifests, the
scratch-only harness, executed binaries, reports and raw invocation receipts
are retained privately under
`<profile>/evidence/2026-11-03-wp08-authentic-color-time-upgrade/` after closeout.

## Actual producer and consumer

- Historical source `91d319ea4aef481abc0f533296bab20d2e1cb318` is the parent of
  `f6c4b2766`, reachable from the tested current source. Its version constant,
  actual producer process and closed database all report **14**.
- Current source `176a9b7997d7f3dcbdd973d114287fc13346752c` reports **31**.
  Its actual `open_and_migrate` migrates a separate copy of the v14 original;
  its actual `install_package` independently installs the same package fresh.
- The identical scratch-only Rust harness in both trees calls those public
  APIs, reports the real install result and explicitly closes the connection.
  It does not construct database rows or reset `user_version`.
- Both builds ran offline and locked with two build jobs, a task-local
  scratch target directory used sequentially, and scratch XDG homes. Both builds, both installs, migration and
  the additional reopen returned exit 0. No workspace/package gate, Clippy
  or product-regression pass is inferred from these executions.

## Fictional input and independent expected result

One authored scheme-14 ZIP contains a master member and a program XML member.
The XML contains these declarations in order:

1. `M-0001_A-1`, named `First authoritative declaration`, defines `PT-C` and
   `PT-T` using deliberately unknown fictional `TypeHologram` children.
2. The same program ID, named `Losing declaration`, defines those type IDs
   with `TypeColor Space="RGB"` and `TypeTime SizeInBit="16"`, bounds
   `3`/`303`, `Unit="Seconds"` and `UIHint="FictionalDuration"`.
3. Unique program `M-0001_A-2` supplies a positive Color/Time control: HSV,
   Time size 32, bounds `7`/`707`, Minutes and `FictionalControl`.

The expected first-winner rows were derived from XML declaration order
before the product runs, not copied from the fresh database. Both real
installers accept the fixture. `TypeHologram` is intentionally unsupported
input, not an asserted KNX type; this is not an XSD-conformance claim.

## Persisted observations

| Program / type | Authentic v14 | Migrated v31 | Fresh v31 / expected |
| --- | --- | --- | --- |
| A-1 / PT-C | Other | **Color** | Other |
| A-1 / PT-T | Other; size/bounds NULL | **Time; 16; `3`/`303`** | Other; size/bounds NULL |
| A-2 / PT-C | Other | Color | Color |
| A-2 / PT-T | Other; size/bounds NULL | Time; 32; `7`/`707` | Time; 32; `7`/`707` |

Program and type names remain the first declaration's names. The upgraded
rows therefore mix winner names with loser type metadata. The unique-program
control promotes correctly, ruling out a general absence of Color/Time support.
The parent independently read all four databases, checked versions, rows,
`integrity_check`, foreign keys and unchanged bytes through read-only SQLite.

### Current evidence is not historical encounter history

The migrated and fresh `ingest_unknown` rows agree in every semantic column
when technical row IDs are excluded. Both retain two `TypeHologram`
occurrences and two occurrences each of Color Space, Time Unit and Time UIHint;
the first samples come from the earlier losing declaration. Losing normalized
rows do not imply losing declarations contributed no unknown evidence.

Historical measured install reports stay historical: v14 and migrated v31
retain three distinct unknown constructs / six occurrences, whereas fresh v31
measures four / eight. Their corresponding package and count rows differ too.
This is the documented v15 historical-report boundary, not classified here
as a second defect or silently replaced with fresh measurements. Application
program encounters remain three read, two stored, one deduplicated. Diagnostic
tables are present and empty in all observed databases.

Whole-database equality is **not** claimed: raw differences also include
technical unknown IDs and the historical-unavailable versus observed master
history. Schema DDL agrees between migrated and fresh databases.

## Attribution, preservation and limits

The executed opener traverses the complete v14-to-v31 chain. The v14-to-v15
pass selects Color/Time source evidence and calls
`parse::program::backfill_type_child_kinds`. In the tested source its update
uses `first_program_declaration || name != "TypeDate"`: Date respects that
same-blob first-declaration guard; Color/Time do not. The source-SHA guard
cannot distinguish two declarations in one blob. Later promotion and Date
repairs do not remove the observed persisted contamination.

There is **no separately executed intermediate-v15 snapshot**. Attribution to
that pass combines the full genuine upgrade with its concrete source path.
Other initial schemas, already contaminated v15+ databases and a safe successor
repair remain independent work. A future correction must handle existing
contaminated non-NULL fields, not merely fill NULLs or change old code.

The closed historical original and independent backup have identical hashes
and were retained read-only; only a separate, non-hardlinked copy was migrated.
All four complete snapshots have integrity `ok`, no foreign-key violations,
no sidecars and unchanged bytes across the read-only observer. Retained blobs
match their recorded hashes and lengths. One further current public open leaves
the migrated database's full logical state **and file hash** unchanged. This
is bounded persistence/idempotence evidence, not crash-durability proof or an
OS immutable-flag claim.

The scratch harness's `rustfmt --check` returned **1** for formatting. Its
actually compiled/executed bytes remain retained unchanged; neither a formatting
pass nor a source acceptance gate is claimed. The observed product invocations
all returned 0 while still producing the incorrect normalized rows.

Only fictional inputs were used. No private corpus, real project, bus, VM,
native UI, download, release or deployment was exercised. WP-08's Date memory
encoding and Colour/Picture editor/codec prerequisites remain unchanged.
