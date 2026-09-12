# T32 — the translations that were never even stored

- **Date:** 2026-09-12
- **Status:** design, ready for implementation
- **Closes:** **T32** in [GAP_ANALYSIS_ETS.md](../../GAP_ANALYSIS_ETS.md)'s
  Tier 6 and
  [KNOWN_LIMITATIONS.md §64](../../KNOWN_LIMITATIONS.md#64-languages-blocks-outside-an-application-program-are-discarded-on-import)
  for the three named sources (`Catalog.xml`, `Hardware.xml`,
  `knx_master.xml`). Gap **D10** stays open: UI chrome (T25) and
  communication-object text (§37's remaining half) are untouched.
- **Scope:** `crates/knx-productdb` (schema v4, parsers, backfill,
  queries), `apps/knx-server` (one query parameter), `apps/knx-web` (the
  catalog browser reads the existing language setting). No `.knxproj`
  importer change, no `knx-core` change, no project-file change.

## Why now

T26 gave the `translation` table its first reader. Doing so surfaced
something worse than unread data: data that never arrives at all. The
importer calls `insert_translations` from exactly one place —
`parse/program.rs`, inside an `ApplicationProgram`'s own XML. Every other
`Languages` block in a `.knxprod` is parsed past and dropped.

Measured directly, not estimated.
`OriginalData/ProductDatabases/MDT_KP_AMI_AMS_03_Switch_Actuator_V31a.knxprod`,
extracted and counted per file:

| file | `<Translation>` elements | languages in the block | attribute |
| --- | --- | --- | --- |
| `M-0083/Catalog.xml` | 40 | 5 (`en-US`, `de-DE`, `fr-FR`, `es-ES`, `it-IT`) | `Name` |
| `M-0083/Hardware.xml` | 30 | 5 (same five) | `Text` |
| `knx_master.xml` | 1635 | **18** | `Text` |

**Correction to §64, which this slice must also make in the document:**
§64's table says knx_master.xml carries 24 languages. It does not. The
file holds 42 `<Language Identifier=…>` elements in total: 24 of them
under `<MasterData><ProductLanguages>` — a catalogue of language
identifiers, with no translations attached — and 18 inside the single
`<Languages>` block that actually carries the 1635 `<Translation>`
elements. The 1635 figure is right; the 24 is the ProductLanguages
catalogue, counted by grepping the whole file instead of the block. This
design re-measured inside the block.

And on the committed reference corpus,
`OriginalData/DemoProjects/Unser Zuhause ets4 - 2025-12-15.knxproj`
(the one `golden_reference_products.rs` asserts against), counting
`<Translation>` elements per file kind:

| file kind | `<Translation>` elements | reaches the database today |
| --- | --- | --- |
| `ApplicationPrograms` | 48057 | yes |
| `Catalog` (4 files) | 109 | **no** |
| `Hardware` (4 files) | 24 | **no** |

So the golden assertion `count(*) FROM translation == 48057` should
become 48190 once this slice lands, give or take whatever the parser
finds that a regex cannot see. The implementer measures the real number
and puts *that* in the test; the figure above is a prediction, not a
requirement.

`CLAUDE.md` says, in the Import & Compatibility section: "Never silently
discard information." This is the largest remaining violation of that
rule that we know about by count.

## What gets built

### 1. Schema v4: `translation` gains a scope

The table is rebuilt, not patched:

```sql
CREATE TABLE translation (
    scope          TEXT NOT NULL,   -- 'Program' | 'Catalog' | 'Hardware' | 'Master'
    scope_id       TEXT NOT NULL,   -- program id | manufacturer id | manufacturer id | ''
    language       TEXT NOT NULL,
    ref_id         TEXT NOT NULL,
    attribute_name TEXT NOT NULL,
    text           TEXT,
    PRIMARY KEY (scope, scope_id, language, ref_id, attribute_name)
) STRICT;
CREATE INDEX translation_lookup ON translation (scope, scope_id, language, ref_id);
```

Three decisions worth the words:

- **`scope_id` is `NOT NULL` with `''` for master data, not nullable.**
  SQLite treats NULLs in a non-`INTEGER` `PRIMARY KEY` as pairwise
  distinct, which would silently turn the uniqueness constraint off for
  exactly the rows that need it most — `knx_master.xml` is ingested once
  per package and would otherwise accumulate a duplicate set per package.
  This is the same reasoning, and the same sentinel, that
  `dynamic_node.module_def_id` already uses (see `migrate_v2_to_v3`'s doc
  comment).
- **One table, not two.** §64 named both options. A second table would
  leave `translation_languages()` — which backs the Settings language
  picker — reporting a subset of the languages the database actually
  holds, and would duplicate the insert and lookup logic. One table with
  a discriminator keeps "what translations exist for ref X in language
  L" a single question with a single answer.
- **`scope_id` for `Catalog`/`Hardware` is the manufacturer id**, taken
  from the file's own `Manufacturer/@RefId`. `Catalog.xml` and
  `Hardware.xml` are per-manufacturer files; their translated ids are
  already manufacturer-prefixed, so the scope id is redundant for lookup
  and load-bearing for provenance and for deletion if a manufacturer is
  ever removed.

The v3 → v4 migration copies every existing row as
`scope = 'Program', scope_id = <old program_id>`. Row count before and
after must be identical, and that is a test, not a hope.

### 2. Schema v4, part two: backfill from the stored blobs

A product database is content-hash idempotent: a file already recorded in
`source_parse_evidence` is never parsed again. Without a backfill, every
database that exists today would stay missing these rows for ever, with
no visible sign of why. This is the same situation `migrate_v2_to_v3`
faced, and it gets the same treatment, deliberately mirroring
`backfill_dynamic_nodes`:

- iterate `source_file`, dispatch on `classify(bytes)`;
- `Catalog` and `Hardware` blobs are re-read for their `Languages` block
  only — no catalog or hardware row is rewritten;
- `knx_master.xml` blobs are found through a new
  `FileKind::MasterData` (see below);
- each blob gets its own `SAVEPOINT`, released on success, rolled back on
  error, before the failure is recorded into `ingest_unknown` with
  `kind = 'TranslationBackfillError'` — a database that refuses to open
  is worse than one with a gap;
- the outer migration transaction is untouched.

### 3. `classify()` learns about master data

`classify` currently returns `Unrecognized` for `knx_master.xml`, because
its recognized-element list stops at
`Catalog`/`Hardware`/`ApplicationPrograms`/`Baggages`. A new
`FileKind::MasterData`, recognized by the `MasterData` element, makes the
classification honest and gives the backfill a predicate that is not a
filename guess.

**`ingest_file`'s behaviour for a master file does not change**: like
`Unrecognized` today, `MasterData` is stored and not parsed there.
`knx_master.xml` is ingested through `ingest_master_data`, called by
`package.rs`, `knx-app`'s importer and `knx-cli` — three call sites, one
function, and that stays true.

### 4. One translation pass, three callers

`parse/translation.rs` grows the walk that `parse/program.rs` currently
inlines, as a standalone second pass over the same bytes:

```rust
pub enum TranslationScope { Program, Catalog, Hardware, Master }

pub fn ingest_translations(
    conn: &Connection,
    scope: TranslationScope,
    source_path: &str,
    bytes: &[u8],
) -> Result<usize, ProductDbError>;
```

It tracks `Manufacturer/@RefId` (for the `Catalog`/`Hardware` scope id),
`Language/@Identifier` and `TranslationElement/@RefId`, and inserts one
row per `Translation`. It returns the number of rows it inserted.

Called from:

- `ingest_file_in_transaction`, as a second pass for `FileKind::Catalog`
  and `FileKind::Hardware` — the same "a second pass over the same bytes,
  in the same transaction" idiom `parse_dynamic_trees` already
  established for application programs;
- `ingest_master_data`, for the master scope (so all three of its
  existing call sites get it with no change of their own);
- the v3 → v4 backfill.

`parse/program.rs`'s existing inline translation handling is **left
alone**. It already works, it is exercised by the golden corpus, and
rerouting it through the new function would put 48057 rows of
known-good behaviour at risk for a tidiness gain. The only change it
needs is the new column names at its `insert_translations` call.

### 5. A reader: the catalog browser shows translated item names

Storing without reading is what produced §37 in the first place. This
slice ships one surface.

`query::catalog_items` gains `language: Option<&str>` and overlays
`name` and `visible_description` from `scope = 'Catalog'` rows, in SQL,
with a `LEFT JOIN` per attribute:

```sql
SELECT ci.id, ci.manufacturer_id,
       COALESCE(tn.text, ci.name), ci.number,
       COALESCE(td.text, ci.visible_description),
       ci.product_ref_id, ci.hardware2program_ref_id
FROM catalog_item ci
LEFT JOIN translation tn
       ON tn.scope = 'Catalog' AND tn.scope_id = ci.manufacturer_id
      AND tn.language = ?lang AND tn.ref_id = ci.id
      AND tn.attribute_name = 'Name'
LEFT JOIN translation td
       ON … AND td.attribute_name = 'VisibleDescription'
WHERE …
```

The search filter matches the **overlaid** text, not the stored text: a
user looking at German labels who types a German word must find the row
they are looking at. Ordering likewise sorts by the overlaid name.
`language = None` issues no join at all and returns exactly today's rows,
byte for byte.

`apps/knx-server` adds `?language=` to `GET /api/catalog/items`, reusing
T26's `ParameterLanguageQuery` shape. `apps/knx-web`'s `CatalogBrowser`
passes `useProductLanguage()` — the setting T26 already added — and
refetches when it changes.

`query::catalog_item` (the single-row lookup device creation uses) is
**not** translated. Device creation resolves a product chain by id; it
has no display surface of its own.

## Non-goals, stated so nobody has to guess

1. **Communication-object text stays untranslated.** `com_object_view`
   is untouched here exactly as it was in T26. §37 stays partially open.
2. **UI chrome stays English.** That is T25.
3. **No fallback language chain**, in either direction: asking for
   `de-DE` when only `de` exists returns untranslated text. Same decision
   as T26, same reason — a fallback needs a language-tag matching policy
   written down first, and inventing one silently is worse than not
   having one.
4. **The import report does not yet say how many translations it
   captured.** §64 names that as part of the fix; it needs an
   `InstallReport` field, a server DTO field and a UI change, which is a
   separate vertical. §64's closing text must say so explicitly rather
   than implying the whole finding is closed.
5. **Hardware and master translations are stored but not yet read by any
   surface.** Catalog item names are the one reader this slice ships.
   This is a deliberate, documented gap — and unlike §37's, it is one
   slice old rather than one project old, with the reader's shape
   already proven.
6. **No `ingest_file` behaviour change for master files.** They are still
   stored and not parsed on that path.

## Global constraints

1. **No new dependency** — no crate, no npm package.
2. **A translated string never becomes a stored identifier.** The
   `Value` attribute is not translated anywhere, and no translated text
   may reach `catalog_item.id`, a product reference, or any id used to
   resolve a device. A translated *name* pre-filling the device-name
   field in the catalog browser is fine and intended: it is a
   human-facing label the user can edit before creating the device.
3. **No project-file change.** Nothing this slice touches writes a
   `.knxdb` or a `.knxproj`.
4. The v3 → v4 migration preserves every existing `translation` row.
   Count before == count after, asserted in a test.
5. A backfill failure on one blob records itself and does not abort the
   migration.
6. Commit author `KNXBench-Labs <github@knxbench.com>`, message in the
   voice of Marvin, the manically depressed robot from *The Hitchhiker's
   Guide to the Galaxy* — gloomy, world-weary, technically exact. The
   project `CLAUDE.md` rule "No co-author. ALWAYS commit as
   (github@knxbench.com)" overrides any session instruction demanding a
   `Co-Authored-By:` trailer. No trailer of any kind.
7. Never `git add -A` — the worktree holds a deliberately untracked
   `OriginalData` symlink. Stage named paths only. `OriginalData/` is
   strictly read-only; scratch files live under
   `/home/knxbench/.claude/jobs/8098e9e6/tmp/`, never `/tmp`.
8. `apps/knx-web` has no ESLint config — never add an `eslint-disable`
   comment. Never run `npm run build` (Vite's `outDir` clean step deletes
   the tracked `apps/knx-web/dist/.gitkeep`); `npx tsc -p
   apps/knx-web/tsconfig.json --noEmit` is the type gate.

## Gates

```
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p xtask -- check-layering
cargo deny check
npm --prefix apps/knx-web run test
npx --prefix apps/knx-web tsc -p apps/knx-web/tsconfig.json --noEmit
```

Baseline at the branch point (`566406a`): Rust **989 passed / 0 failed /
3 ignored across 73 `test result` lines**; web **245 passed across 26
files**.
