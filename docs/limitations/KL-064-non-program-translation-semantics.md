# Non-program translation semantics

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-064)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 64. `Languages` blocks outside an application program are discarded on import

**Current reading, 2026-10-08:** the heading describes the pre-T32 defect,
not current ingestion. Program/catalog/hardware/master translations are
ingested; supported overlays and fallback markers have consumers. Remaining
gaps concern specific unmodelled master semantics/consumers, selected project
language and English technical prose, not blanket loss of non-program Languages.
The dated updates below retain the evidence and their narrower scope.

**Resolved for ingestion (2026-09-12, T32); reading closed for every
entity family this project's corpus has found a `Master`-scope
translation for (2026-09-14, T13) — the residue below is what is left.**
The heading is kept verbatim because five documents link to its anchor;
read the status here, not in the title.

**Ingested now.** `translation` was widened in schema v4 to `(scope,
scope_id, language, ref_id, attribute_name)`
(`crates/knx-productdb/src/migration.rs`), with `''` as the master-scope
`scope_id` sentinel — `knx_master.xml` has no owning element, and SQLite
treats NULLs in a non-`INTEGER` primary key as pairwise distinct, so the
one thing a sentinel is needed for is the one thing NULL will not do. A
single `ingest_translations` pass (`parse/translation.rs`) now reads the
`Languages` block of `Catalog.xml` and `Hardware.xml` (keyed by
`Manufacturer/@RefId`) and of `knx_master.xml` (`FileKind::MasterData`,
master sentinel). `parse/program.rs` keeps its own inline handling
unchanged. A v3→v4 backfill replays the blobs already stored, so a
database installed before this slice does not stay translation-less;
a blob that fails to parse records itself into `ingest_unknown` as a
`TranslationBackfillError` and the migration continues.

Re-measured on the same package this section first cited,
`MDT_KP_AMI_AMS_03_Switch_Actuator_V31a.knxprod`, by installing it with
`knx products ingest` and counting `translation` rows per scope:

| scope | rows | distinct languages |
|---|---|---|
| `Catalog` | 40 | 5 (`de-DE`, `en-US`, `es-ES`, `fr-FR`, `it-IT`) |
| `Hardware` | 30 | 5 (same five) |
| `Master` | 1635 | 18 |
| `Program` | 18546 | 5 |
| **total** | **20251** | |

The 1705 rows this section was opened for — 40 + 30 + 1635 — are in the
database. The golden corpus assertion moved with them: 48,190 rows for
the reference project (48,057 program + 109 catalog + 24 hardware),
re-measured rather than predicted.

**Correction to the earlier figure.** The table previously published here
said `knx_master.xml` carried 1635 translations in **24** languages. The
row count was right; the language count was not. The file's single
`<Languages>` block holds those 1635 translations across **18**
languages. The file declares 42 `<Language>` elements in total, and the
other 24 sit in a separate `<MasterData><ProductLanguages>` block — a
catalogue of language identifiers with no translations attached to them
at all. The earlier number came from grepping the whole file instead of
the block. Measured wrongly here first, corrected here now.

**Still open, narrowed once (2026-09-13, T16).** Ingestion is no longer
the gap; reading is. Two surfaces now read these rows: the catalog
browser, whose item `Name` and `VisibleDescription` are overlaid by
`query::catalog_items(conn, …, language)` behind `GET
/api/catalog/items?language=` (T32 Task 4), and — new — the device
detail panel's product/hardware block, whose `product.text` (`Product`
scope `Hardware`) is overlaid by the new `query::device_product(conn, …,
language)` behind `GET /api/device/{id}?language=` (T16, branch
`t16-device-product`), which also overlays `catalog_item.name` (same
join `catalog_items` already uses) and `application_program.name`
(`Program` scope) for the same response. `Hardware`-scope translation
rows therefore have a reader now, but the claim only narrows, it does
not close: `Master`-scope translations — the entire shared KNX
vocabulary of `knx_master.xml` — are still stored, queryable, and read
by nothing. A second, orthogonal observation, empirically
checked rather than assumed while building T16 and re-checked in review:
across nine `Hardware.xml` files from seven manufacturers — the five
packages under `OriginalData/ProductDatabases/` plus the manufacturer
packages inside the two reference ETS exports, which are one installation
exported from ETS 4 and ETS 6 rather than two independent ones — no
`Hardware.xml` places a `Hardware/@Id` inside a
`TranslationElement/@RefId`; every one of them is a `Product/@Id`. So
`hardware.name` has nothing to read rather than a missing reader, in
every package seen so far. This is a statement about the corpus, not
about the format: the schema does not forbid a `Hardware`-keyed
translation row, and one package carrying one would overturn it. And no
translated string is ever allowed to become a stored identifier —
`query::catalog_item`, the single-row lookup device creation uses, is
deliberately untranslated.

**Narrowed further, still not closed (2026-09-13, D10 slice 1, branch
`d10-master-translations`).** Three of this section's own open items
move. First, the import report now does state how many translations a
package contributed: `InstallReport`/`IngestOutcome` gained a
`TranslationCounts { program, catalog, hardware, master }`, counted from
`INSERT OR IGNORE`'s own affected-row count — a repeated `Translation`
element that the parser walks past but SQLite ignores as a duplicate key
contributes nothing to the count, exactly as `unknown_count` already
counted writes rather than sightings. `knx-cli`'s `install` output
prints the total and the per-scope breakdown. A package already
installed under the previous schema (v4) reports zero for all four
counters on a retried install rather than a guess — re-deriving the true
figure would mean re-parsing bytes this migration has no access to, so
it names the gap instead of inventing a number; a package installed
from this slice onward always gets its real count. Second,
locale-prefix matching is no longer absent — see the correction to §37
cross-referenced there; it is implemented once, in `query.rs`'s
`best_matching_language`, and reused by every overlay this function has
ever had plus the new one described next, but nothing in `apps/knx-web`
sends a bare primary-language tag yet, so the backend half closes and
the round trip does not. Third, `query::datapoint_types`/
`query::datapoint_type` is a new `Master`-scope reader — the first one —
for `datapoint_type` rows (measured non-empty on installation across all
five sampled packages: 383, 354, 234, 234 and 234 rows), overlaying a
`Master`-scope, `Text`-attribute translation onto each row's `text` when
one resolves for the requested language. It is deliberately narrow: it
only ever has rows for the `RefId` families `datapoint_type` itself
holds data for (`DPST-*`, `DPT-*`). Two of the five sampled packages'
`knx_master.xml` also carry `Master`-scope translations for `FT-*`
(function types), `SU-*` (space usages) and `FP-*_DR-*`
(functional-profile/datapoint pairs) — confirmed independently in both
(`MDT_KP_AMI_AMS_03_Switch_Actuator_V31a`: DPST-\*=328, DPT-\*=47,
FP-\*\_DR-\*=738, FT-\*=180, SU-\*=342 of 1635 master rows;
`Dummy_Applikation_Secure`: DPST-\*=314, DPT-\*=45, FP-\*\_DR-\*=697,
FT-\*=170, SU-\*=323 of 1549) — but `parse/master.rs` parses none of
`FunctionType`/`FunctionPoint`/`SpaceUsage`: there is no table for those
`RefId`s to join against, so no reader, this one included, can surface
them. The other three sampled packages' `knx_master.xml` predates that
scheme and carries only `DPST-*`/`DPT-*` master translations, nothing
this gap touches. A translated function-type or space-usage name stays
unavailable until a later slice gives those constructs their own
tables — tracked here, not silently narrowed out of this section's
claim.

**Closed, 2026-09-14 (T13, branch `d10-language-data`).** The residue
above is what this slice closes: `FunctionType`, `FunctionPoint` and
`SpaceUsage` each get a table now (`function_type`, `function_point`,
`space_usage`; schema v9 → v10, `migrate_v9_to_v10`), filled by
`parse/master.rs`'s `ingest_master_data` the same `INSERT OR IGNORE` way
`datapoint_type` already was. `query.rs` gained `function_types`/
`function_type`, `function_points` (scoped to one `function_type_id`)
and `space_usages`/`space_usage`, each overlaying `text` from a
`Master`-scope translation through the same `master_text_overlay` every
other Master reader here already used — that function never filtered by
`RefId` prefix, so the translations these three families needed were
already sitting in `translation` since T32; only the join target was
missing. A v9 database is backfilled the same way a v3 one was for T32:
its `knx_master.xml` blob is replayed through `ingest_master_data` inside
the migration (`a_v9_database_backfills_function_and_space_usage_rows_and_their_translations`,
`migration.rs`), so a database that already existed before this slice
does not stay short these three tables' worth of data. An end-to-end
test through `install_package`
(`hardware_and_master_scope_translations_survive_install_with_their_text_intact`,
`tests/standalone_packages.rs`) reads a planted `Hardware`-scope and a
planted `Master`-scope translation's actual text back out of `translation`
after a real package install, not merely a row count. Neither reader
gained an HTTP route or a UI element — surfacing stops exactly where
`datapoint_types` already stopped (no route in
`apps/knx-server/src/routes.rs`), per this project's "surface only as far
as existing machinery already reaches" rule; a caller inside the backend
can call these functions today, nothing outside it can yet.

**Residue restated, not claimed closed.** Three things this slice does
not touch, stated plainly rather than left implicit: first, the other
eight `MasterData` child sections `parse/master.rs`'s own module doc
names (`DatapointRoles`, `InterfaceObjectTypes`,
`InterfaceObjectProperties`, `PropertyDataTypes`, `MediumTypes`,
`MaskVersions`, `FunctionalBlocks`, `ProductLanguages`) stay unparsed;
none of them carried a `Master`-scope translation in any of the five
sampled packages, but that is a corpus observation, not a schema
guarantee, and a package that did translate one would have that
translation's row sit in `translation` unread by anything, exactly as
`FunctionType`/`SpaceUsage` did before this slice. Second, the
`function_type`/`function_point`/`space_usage` tables inherit
`datapoint_type`'s uncounted-collision gap outright — see §86's residue,
extended 2026-09-14 to name them — a second package's `knx_master.xml`
drops its restated rows with nothing recording that it happened. Third,
`apps/knx-web` still sends no bare primary-language tag (§37's own open
item, unchanged by this slice): the backend-side locale-prefix matching
these new readers reuse has had a caller-reachable surface since D10
slice 1, and still has none from the frontend.

**Lifted when.** Ingestion: lifted 2026-09-12 (T32, branch
`t32-shared-translations`). The `Hardware`-scope half of the reading
residue: lifted 2026-09-13 (T16, branch `t16-device-product`). The
`Master`-scope residue for `datapoint_type`, translation-count
reporting, and backend locale-prefix matching: lifted 2026-09-13 (D10
slice 1, branch `d10-master-translations`). `FunctionType`/
`FunctionPoint`/`SpaceUsage`: lifted 2026-09-14 (T13, branch
`d10-language-data`). This section's own residue (other `MasterData`
sections, collision counting, frontend locale tags) stays open; see
**D10** in [GAP_ANALYSIS_ETS.md](https://github.com/KNXBench-Labs/KNXBench/blob/a584007fc05a/docs/GAP_ANALYSIS_ETS.md) and §37's own
"still open" list. Not scheduled.

**Update 2026-10-05 (AR10 slice 2b): accepted boundary.** Every reader of
these rows now names the stored language that answered, or reports that the
package's own text was kept (see §37's update of the same day and
[research/backend-localization-paths.md](../research/backend-localization-paths.md)).
What remains is stated above and accepted rather than scheduled: the eight
unparsed `MasterData` sections carried no translation in any sampled package;
collision counting belongs to §86; a bare locale tag from the frontend is the
UI owner's choice, and the backend already matches one.
<!-- END retained catalogue record -->
