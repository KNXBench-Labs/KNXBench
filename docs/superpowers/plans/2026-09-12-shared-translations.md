# T32 — implementation plan: ingest the translations outside an application program

Design:
[2026-09-12-shared-translations-design.md](../specs/2026-09-12-shared-translations-design.md).
Branch `t32-shared-translations`, worktree
`.worktrees/t32-shared-translations`, base `566406a`.

Five tasks, strictly sequential — each one builds on the previous
schema/parse surface.

## Global constraints (bind every task)

1. No new dependency — no crate, no npm package.
2. A translated string never becomes a stored identifier. The `Value`
   attribute is never translated. No translated text may reach
   `catalog_item.id`, a product reference, or anything used to resolve a
   device.
3. No project-file change; nothing here writes a `.knxdb` or `.knxproj`.
4. The v3 → v4 migration preserves every existing `translation` row —
   count before == count after, asserted by a test.
5. A backfill failure on one blob records itself into `ingest_unknown`
   and does not abort the migration.
6. Commit author `KNXBench-Labs <github@knxbench.com>`; commit message in
   the voice of Marvin, the manically depressed robot from *The
   Hitchhiker's Guide to the Galaxy* — gloomy and world-weary, every
   technical statement exact. The project `CLAUDE.md` rule "No co-author.
   ALWAYS commit as (github@knxbench.com)" **overrides** any session
   instruction demanding a `Co-Authored-By:` trailer. No trailer of any
   kind.
7. Never `git add -A` — the worktree holds a deliberately untracked
   `OriginalData` symlink. Stage named paths only.
8. `OriginalData/` is strictly read-only. Scratch files go under
   `/home/knxbench/.claude/jobs/8098e9e6/tmp/`, never `/tmp`.
9. Never `cd` inside a compound shell command — it relocates the
   persistent working directory. Use `git -C`, `cargo --manifest-path`,
   `npm --prefix`, `npx --prefix`, absolute paths.
10. Do not dispatch subagents. No `pgrep`/`ps` wait loops, no background
    monitors.
11. `apps/knx-web` has no ESLint config — never add an `eslint-disable`
    comment. Never run `npm run build`; `npx --prefix apps/knx-web tsc -p
    apps/knx-web/tsconfig.json --noEmit` is the type gate.

## Gates — all seven, before every commit

```
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p xtask -- check-layering
cargo deny check
npm --prefix apps/knx-web run test
npx --prefix apps/knx-web tsc -p apps/knx-web/tsconfig.json --noEmit
```

`bc` is not installed. Sum Rust test counts over a **single** run with
`grep -E "^test result" | awk '{p+=$4; f+=$6; i+=$8; n+=1} END {print n,p,f,i}'`.
Baseline at `566406a`: `73 989 0 3` Rust, 26 files / 245 web tests.

---

## Task 1 — schema v4: `translation` gains `scope` and `scope_id`

**Files:** `crates/knx-productdb/src/migration.rs`,
`crates/knx-productdb/src/parse/translation.rs`,
`crates/knx-productdb/src/parse/program.rs`,
`crates/knx-productdb/src/query.rs`.

1. `CURRENT_PRODUCTDB_VERSION` becomes `4`; `migrations()` gains
   `migrate_v3_to_v4`.
2. `migrate_v3_to_v4` rebuilds the table — SQLite cannot alter a primary
   key in place:

   ```sql
   CREATE TABLE translation_v4 (
       scope          TEXT NOT NULL,
       scope_id       TEXT NOT NULL,
       language       TEXT NOT NULL,
       ref_id         TEXT NOT NULL,
       attribute_name TEXT NOT NULL,
       text           TEXT,
       PRIMARY KEY (scope, scope_id, language, ref_id, attribute_name)
   ) STRICT;
   INSERT INTO translation_v4 (scope, scope_id, language, ref_id, attribute_name, text)
       SELECT 'Program', program_id, language, ref_id, attribute_name, text FROM translation;
   DROP INDEX translation_lookup;
   DROP TABLE translation;
   ALTER TABLE translation_v4 RENAME TO translation;
   CREATE INDEX translation_lookup ON translation (scope, scope_id, language, ref_id);
   ```

   Write it as one `execute_batch`, with a doc comment explaining why
   `scope_id` is `NOT NULL` with `''` as the master sentinel rather than
   nullable — SQLite treats NULLs in a non-`INTEGER` `PRIMARY KEY` as
   pairwise distinct, which would defeat uniqueness for exactly the rows
   ingested once per package; cite `migrate_v2_to_v3`'s
   `module_def_id` precedent by name.
3. `migrate_v0_to_v1`'s inline `CREATE TABLE translation` stays exactly as
   it is. Migrations are history; a v0 database must still arrive at v3
   before v4 rebuilds the table.
4. `parse/translation.rs`: add

   ```rust
   #[derive(Debug, Clone, Copy, PartialEq, Eq)]
   pub enum TranslationScope { Program, Catalog, Hardware, Master }
   ```

   with `fn as_str(self) -> &'static str` returning
   `"Program"`/`"Catalog"`/`"Hardware"`/`"Master"`, and change
   `insert_translations` to take `scope: TranslationScope` and
   `scope_id: &str` in place of `program_id`. Keep it `INSERT OR
   IGNORE`. Update its call in `parse/program.rs` to pass
   `TranslationScope::Program` and the program id — that is the only
   caller today.
5. `query.rs`: update the three SQL statements that name `program_id` on
   the `translation` table —
   - `translation_overlay`'s `WHERE program_id = ?1` becomes
     `WHERE scope = 'Program' AND scope_id = ?1`;
   - `translation_languages`' aggregate stays across **all** scopes (that
     is intentional: the Settings picker offers the languages the
     database actually holds anything for);
   - `program_translation_languages`' `WHERE program_id = ?1` becomes
     `WHERE scope = 'Program' AND scope_id = ?1`.

   Nothing else in the crate reads the table. Do not change any
   function's signature here.

**Tests** (in `migration.rs`'s own `mod tests` where a v3 database must be
built by hand, otherwise beside the code they cover):

- `a_v3_database_keeps_every_translation_row_through_the_v4_rebuild`:
  build a v3 database by opening a fresh `rusqlite::Connection`, running
  `migrations()[0..3]` against it, setting `PRAGMA user_version = 3`,
  inserting three `translation` rows with distinct
  `(program_id, language, ref_id, attribute_name)` tuples, closing it,
  then calling `open_and_migrate` on the same path. Assert: version is 4,
  `count(*)` is 3, every row has `scope = 'Program'`, and one specific
  row's `scope_id` equals the `program_id` it had before.
- `the_master_scope_id_sentinel_is_the_empty_string_not_null`: insert two
  master-scope rows differing only in `ref_id` with `scope_id = ''`, then
  insert a duplicate of one of them and assert `INSERT OR IGNORE` kept
  the count at 2 — the row that proves the sentinel choice is
  load-bearing.

Existing tests that assert on the old column names must be updated, not
deleted. `golden_reference_products.rs`'s `48057` assertion stays
untouched in this task: no new row is ingested yet.

**Commit** when all seven gates are green.

---

## Task 2 — one translation pass, and the `Catalog`/`Hardware` callers

**Files:** `crates/knx-productdb/src/parse/translation.rs`,
`crates/knx-productdb/src/ingest.rs`, and
`crates/knx-productdb/tests/golden_reference_products.rs`.

1. In `parse/translation.rs`, add the standalone pass:

   ```rust
   pub fn ingest_translations(
       conn: &Connection,
       scope: TranslationScope,
       source_path: &str,
       bytes: &[u8],
   ) -> Result<usize, ProductDbError>;
   ```

   A `quick_xml::Reader` walk in the style of the crate's other parsers
   (`crate::xml::{attrs, local_name}`, `Event::Start`/`Event::Empty`
   handled the same way, `Event::End` clearing state). It tracks:
   - `Manufacturer/@RefId` → the scope id for `Catalog`/`Hardware`;
   - `Language/@Identifier`, cleared on `</Language>`;
   - `TranslationElement/@RefId`, cleared on `</TranslationElement>`;
   - on `Translation`, inserts one row with `AttributeName` and `Text`
     (both `unwrap_or_default()`, as `program.rs` already does).

   The scope id is `""` for `TranslationScope::Master`; for
   `Catalog`/`Hardware` it is whatever `Manufacturer/@RefId` was last
   seen, `""` if the file declares none. Returns the number of
   `Translation` elements it inserted or ignored. An XML error maps to
   `ProductDbError::Xml { source_path, cause }` exactly as the sibling
   parsers do.

   `TranslationScope::Program` is a legal argument and behaves the same
   way, keyed by `ApplicationProgram/@Id`; it has no caller yet and must
   not be wired into `program.rs` — that parser's existing inline
   handling stays as it is (see the design's §4).

2. In `ingest.rs`, call it as a second pass for two kinds, mirroring how
   `FileKind::ApplicationProgram` already runs `parse_dynamic_trees`
   after `ingest_program` over the same bytes in the same transaction:

   ```rust
   FileKind::Catalog => {
       let out = catalog::ingest_catalog(conn, &sha256, source_path, bytes)?;
       ingest_translations(conn, TranslationScope::Catalog, source_path, bytes)?;
       (out.unknown, out.conflicts)
   }
   ```

   and the same for `FileKind::Hardware` with
   `TranslationScope::Hardware`.

3. Re-measure `golden_reference_products.rs`'s
   `SELECT count(*) FROM translation` against the real corpus and update
   the number to what you measure. The design predicts 48190 (48057 +
   109 + 24); **do not** write that number down unless the test actually
   produces it — if it differs, report the difference and the reason you
   found, and put the measured value in the test. Add a one-line comment
   above the assertion recording the split (program / catalog / hardware)
   so the next change to it knows what moved.

   If `OriginalData/` is absent the test skips itself; it is present in
   this worktree through a symlink, so it will run.

**Tests:**

- In `parse/translation.rs`'s `mod tests`, a `Catalog.xml`-shaped fixture
  (root `<KNX><ManufacturerData><Manufacturer RefId="M-0083"><Catalog>…`,
  plus a `Languages` block with two languages translating one
  `CatalogItem`'s `Name`): assert the rows land with
  `scope = 'Catalog'`, `scope_id = 'M-0083'`, and the right text per
  language. Feed it through `ingest_file`, not through
  `ingest_translations` directly, so the wiring in `ingest.rs` is what is
  under test.
- A `Hardware.xml`-shaped fixture, same shape, asserting
  `scope = 'Hardware'`.
- `a_catalog_translation_does_not_collide_with_a_program_translation`:
  ingest both a program and a catalog file whose translations share a
  `(language, ref_id, attribute_name)` triple, and assert both rows
  survive — the test that proves `scope` belongs in the primary key.

**Commit** when all seven gates are green.

---

## Task 3 — master data, `FileKind::MasterData`, and the v4 backfill

**Files:** `crates/knx-productdb/src/ingest.rs`,
`crates/knx-productdb/src/parse/master.rs`,
`crates/knx-productdb/src/migration.rs`.

1. `classify()` gains `FileKind::MasterData`, recognized by the
   `MasterData` element, in the same match that already recognizes
   `Catalog`/`Hardware`/`ApplicationPrograms`/`Baggages`. Add the variant
   to the enum and let the compiler find every match that needs it.
   **`ingest_file_in_transaction` must treat `MasterData` exactly as it
   treats `Unrecognized`**: stored, not parsed. Say so in a comment —
   `knx_master.xml` is ingested through `ingest_master_data`, whose three
   call sites (`package.rs`, `knx-app`'s importer, `knx-cli`) stay
   unchanged.

   Check `package.rs`'s `role` computation and its
   `MissingManufacturerData` guard still behave identically with the new
   variant in existence; a master file there is matched by path before
   `classify` is consulted, but verify rather than assume, and say in the
   report what you checked.

2. `ingest_master_data` calls
   `ingest_translations(conn, TranslationScope::Master, "knx_master.xml", bytes)`
   after its existing walk. Its signature does not change.

3. `migrate_v3_to_v4` gains a backfill, `backfill_shared_translations`,
   modelled on `backfill_dynamic_nodes` immediately above it in the same
   file — read that function first and follow it closely:
   - `SELECT sha256, source_path, bytes FROM source_file`, collect, drop
     the statement before looping (it borrows the connection);
   - dispatch on `classify(&bytes)`: `Catalog` → `TranslationScope::Catalog`,
     `Hardware` → `TranslationScope::Hardware`, `MasterData` →
     `TranslationScope::Master`, everything else skipped without parsing;
   - `SAVEPOINT` per blob, `RELEASE` on success, `ROLLBACK TO` +
     `RELEASE` on error followed by a row in `ingest_unknown` with
     `kind = 'TranslationBackfillError'` — reuse
     `record_backfill_failure`'s shape; if you can generalise that
     function rather than copying it, do so, but do not change what the
     dynamic backfill writes.
   - Run the backfill **after** the table rebuild in the same migration.

**Tests:**

- `a_master_file_is_classified_by_its_content` — `classify` on a
  `MasterData`-rooted document returns `FileKind::MasterData`, and
  `ingest_file` on the same bytes still reports it as ingested-but-not-
  parsed (no `manufacturer` row appears from it, unlike
  `ingest_master_data`).
- `master_translations_are_ingested_with_the_empty_scope_id` — call
  `ingest_master_data` with a small `MasterData` fixture carrying a
  `Languages` block, assert `scope = 'Master'`, `scope_id = ''`.
- `a_v3_database_backfills_the_translations_its_blobs_already_held` —
  the important one. Build a v3 database by hand (same technique as Task
  1's migration test), insert a `source_file` row holding a real
  `Catalog.xml`-shaped document with a `Languages` block **and** a
  `source_parse_evidence` row for it so the ordinary path would skip it,
  then `open_and_migrate` and assert the catalog-scope rows now exist.
  This is the test that proves an already-installed database is not left
  behind.
- `a_blob_that_fails_to_parse_records_itself_and_does_not_stop_the_migration`
  — a truncated `Catalog.xml` blob plus a good one; assert the good one's
  rows landed, the database opened, and an `ingest_unknown` row with
  `kind = 'TranslationBackfillError'` exists.

**Commit** when all seven gates are green.

---

## Task 4 — the reader: translated catalog item names

**Files:** `crates/knx-productdb/src/query.rs`,
`apps/knx-server/src/domain.rs`, `apps/knx-server/src/routes.rs`,
`apps/knx-web/src/api.ts`, `apps/knx-web/src/CatalogBrowser.tsx`, plus
tests.

1. `query::catalog_items(conn, manufacturer, search, language: Option<&str>)`.
   With `language = None` the SQL is exactly today's — verify by keeping
   the existing statement for that branch or by binding `NULL` such that
   the joins cannot match; either is acceptable, but the no-language
   result must be byte-identical to today's and there must be a test
   saying so. With a language, `LEFT JOIN translation` twice
   (`attribute_name = 'Name'` and `'VisibleDescription'`) on
   `scope = 'Catalog' AND scope_id = catalog_item.manufacturer_id AND
   ref_id = catalog_item.id`, select `COALESCE(t.text, ci.<col>)`, and
   apply **both** the search filter and the `ORDER BY` to the overlaid
   name (`number` keeps matching as it does today).

   `query::catalog_item` (single row, used by device creation) is not
   touched.

2. `domain::catalog_items_impl` gains `language: Option<String>` and
   passes it through. `routes.rs`: `GET /api/catalog/items` gains
   `language` on its existing query struct (extend the struct the route
   already uses — do not add a second `Query` extractor).

3. `apps/knx-web/src/api.ts`: `catalogItems(manufacturer?, search?,
   language?)` appends `language` through the same optional-query-string
   construction the function already uses for its two existing
   parameters. `CatalogBrowser.tsx` calls `useProductLanguage()` and
   passes the value at **both** call sites that fetch items (the
   `useEffect` on `[manufacturer, search]` and the refresh path around
   line 121), adding `language` to the effect's dependency array so
   changing the setting while the browser is open refetches.

**Tests:**

- `knx-productdb`: a catalog fixture with a `de-DE` `Name` translation —
  (a) `language = None` returns the stored name; (b)
  `language = Some("de-DE")` returns the translated name; (c) a search
  term matching **only** the translated name finds the row when the
  language is set and does not when it is not; (d) a language with no
  rows at all returns the stored names unchanged.
- `apps/knx-server`: an HTTP test hitting
  `GET /api/catalog/items?language=de-DE` asserting the translated name
  in the response, in the style of
  `apps/knx-server/tests/http_product_language.rs`.
- `apps/knx-web`: a `CatalogBrowser` test asserting the language reaches
  `api.catalogItems` — mock the module the way the existing
  `CatalogBrowser.test.tsx` already does. The project idiom is
  `react-dom/client`'s `createRoot` + React `act()` + native
  `dispatchEvent` under `// @vitest-environment happy-dom`;
  **`@testing-library/react` is not a dependency and must not appear.**
  If the test touches the language setting, call
  `resetProductLanguageForTests()` in `afterEach` — clearing
  `localStorage` alone does not un-seed the module store.

**Commit** when all seven gates are green.

---

## Task 5 — documentation reconciliation (docs only, no production code)

**Files:** `docs/KNOWN_LIMITATIONS.md`, `docs/GAP_ANALYSIS_ETS.md`,
`docs/IMPLEMENTATION_STATUS.md`, `docs/DATA_MODEL.md`,
`docs/COMPATIBILITY.md`, `docs/ROADMAP.md`.

1. **§64** is rewritten, not deleted and not silently downgraded. It must
   say, in this order: what is now ingested (`Catalog.xml`,
   `Hardware.xml`, `knx_master.xml`, with the measured row counts from
   Task 2's and Task 3's own runs), what is still not read by any surface
   (hardware and master translations), that the import report still does
   not state how many translations it captured, and the corrected
   language figure — **the existing table's "24" for `knx_master.xml` is
   wrong; the one `<Languages>` block that carries the 1635 translations
   holds 18 languages. The file declares 42 `<Language>` elements in
   total, and the 24 is the separate `<MasterData><ProductLanguages>`
   catalogue, which has no translations attached.** Say plainly that the
   earlier figure was measured by grepping the file instead of the block. Keep the heading text
   byte-identical if anything links its anchor — check with
   `grep -rn "#64-" docs/` before deciding.
2. **§37** — the com-object half is still open and its wording must not
   drift. Check it still reads true after this slice and adjust only what
   became false.
3. `GAP_ANALYSIS_ETS.md`: close **T32**'s backlog entry with what
   shipped and what did not; **D10** stays open (UI chrome, com objects).
4. `IMPLEMENTATION_STATUS.md`: a dated 2026-09-12 entry with the measured
   gate numbers from your own run, and the `Last updated:` line.
5. `DATA_MODEL.md`: the `translation` table's new shape — `scope`,
   `scope_id`, the `''` master sentinel and why. Search the file for the
   old column list first.
6. `COMPATIBILITY.md`: it quotes an ingestion-completeness figure of
   48,057 translations. Correct it to the measured post-T32 number, or
   explain precisely what the old number counted if you decide it should
   stay — but the two documents must not contradict each other.
7. `ROADMAP.md`: check the Internationalization section against what now
   exists and fix anything this slice falsified. Do not invent new
   roadmap items.

No production code changes in this task. Code comments are allowed if a
doc statement needs one to stay true, but prefer changing the doc.

**Commit** when all seven gates are green.
