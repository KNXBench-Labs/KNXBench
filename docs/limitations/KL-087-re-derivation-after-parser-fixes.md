# Re-derivation after parser fixes

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-087)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 87. A parse fix does not reach rows that were already ingested, and only a migration can go back for them

**Fixed for `linkable`, on 2026-09-14, by product-database schema v8, and for
`parameter_type`'s `Float`/`Text` bounds, the same day, by schema v9**
([ADR-0020](../adr/0020-migrations-may-rederive-from-stored-bytes.md)). The
general shape of the defect is not fixed and cannot be, so this section stays
— rewritten to describe the class rather than either instance.

**Limitation.** Every parsed row in `products.sqlite` is a derived value, and
the derivation happens exactly once: at ingest. Three independent
short-circuits then make sure it never happens again for the same bytes —
`install_package` returns `skipped: true` on a package whose sha256 is already
on record (`crates/knx-productdb/src/package.rs`), `ingest_file` skips a blob
already in `source_parse_evidence` (`ingest.rs`), and `ingest_program` sets
`already_present` when a program id is already in the table
(`parse/program.rs`). They are all correct, and they are why a parse-layer fix
shipped today changes nothing about a database ingested yesterday. The bytes
are never lost — every member's XML is in `source_file`, which is the whole
point of ADR-0011 — but nothing re-reads them of its own accord.

**Consequence.** After any fix to the parse layer, a row that the old code
derived wrongly keeps the old answer, and a reader who checks whether the fix
worked by querying an existing database concludes that it did not. There is no
marker on a row saying which build derived it, so a stale value and a current
one are indistinguishable by inspection.

**Workaround.** Ingest into a fresh database, which re-derives everything
from the packages. Measured on the repository corpus (five `.knxprod`, three
`.knxproj`, debug build): 17.0 s for a 128 MB database. This needs the
original files, which the blob store exists precisely so a user need not keep
— see ADR-0020's alternatives for why that is a fallback rather than the
answer.

**The one instance that was repaired, and how.** `bool_flag`
(`crates/knx-productdb/src/parse/mod.rs`) accepted only `"1"` and `"0"` until
2026-09-13, so every word-spelled `Linkable` was stored as `NULL`
**[V]**. Not only in schema-20/21 packages, as this section previously
claimed: the spelling belongs to the tool that wrote the file, and
`grep -o 'Linkable="[^"]*"'` over the corpus finds word form in
`project/11`, `20` and `21` archives and numeric form in `project/11` and `23`
ones. `migrate_v7_to_v8` re-reads `ApplicationProgram/@Linkable` out of each
affected blob and fills the column, only where it is `NULL`, only for the row
that blob's bytes produced, and retires the now-false "attribute not
understood" `ingest_unknown` row it supersedes. Measured: 34 of 34 corpus
programs refilled in 1.14 s, with values identical to a fresh ingest (27
false, 7 true); a v6 database with nothing to fill opens in 0.038 s, the same
as one already at v8.

**The second instance, and how it differs.** T18 slice 5 (2026-09-13) gave
`parameter_type` real bounds — `TypeFloat/@minInclusive`/`@maxInclusive` and
`TypeText/@SizeInBit` — but, same as `Linkable` before it, only for rows
ingested from that day forward; every program ingested earlier keeps
`min_inclusive`/`max_inclusive`/`size_in_bit` `NULL` regardless of what its
own `source_file` blob actually says. `migrate_v8_to_v9` re-reads those three
attributes out of each affected blob and fills the columns, only where
`Float`'s pair is `NULL` together or `Text`'s size is `NULL`, only for the row
that blob's bytes produced (`backfill_parameter_type_bounds` in
`crates/knx-productdb/src/migration.rs`, mirroring `backfill_linkable`'s
shape exactly — `parameter_type` has no `source_sha256` of its own, so the
scoping check joins through `application_program`, which does). One respect
in which it does not mirror `linkable`: there was no stale `ingest_unknown`
row to retire, because the pre-T18 parser never asked for these attributes at
all — it neither read them nor rejected them, so nothing was ever reported.
A separate, unrelated fix landing the same day
(`insert_parameter_type` now calls `report_unknown_attrs` on `TypeFloat`'s
children) means a freshly-ingested row and a v9-backfilled row still differ
in one way this migration does not close: the fresh row also gets
`ingest_unknown` entries for `Encoding`/`Increment`/`DisplayFormat`, and the
backfilled one does not. Named here rather than silently left different.

This is the class's **second** occurrence, not its third.

**Lifted when.** For the class: never entirely, by construction — a
derivation that already ran cannot know it should run again. What is missing
is only the *detection*, and ADR-0020 records the shape of it (a parse
generation recorded per `source_file` row, reported by `knx products verify`),
deliberately not built for a single column. Reach for it if the class turns up
a third time — two is not yet that threshold. For an individual instance: a
v-next backfill migration, under ADR-0020's rule — permitted when the value
is a pure function of bytes the database already holds, forbidden when it
depended on the install event.
<!-- END retained catalogue record -->
