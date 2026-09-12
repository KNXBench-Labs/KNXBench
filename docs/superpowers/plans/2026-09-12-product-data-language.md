# T26 (first slice) — implementation plan: product-data language

Spec: `docs/superpowers/specs/2026-09-12-product-data-language-design.md`.
Read it first; it is the binding authority and this plan is its argument.

Branch: `t26-product-language`, worktree
`.worktrees/t26-product-language`, base `main`.

## Global Constraints

1. **No new dependency**, npm or cargo. No i18n library. The whole slice
   is one SQL overlay, one route, one query parameter and one `<select>`.
2. **No schema migration.** The `translation` table is used exactly as
   `migration.rs` created it. Do not add, rename or index a column.
3. **`Value` is never translated.** Only display attributes (`Text`,
   `FunctionText`, `SuffixText`, `VisibleDescription`, `Name`) may be
   overlaid. A parameter's value is a key written into the project file;
   translating it by display language would corrupt stored data.
4. **No fallback chain between languages.** Requested language → the
   package's own untranslated attribute. Never `en-US` as a middle step.
5. **Nothing that writes a project file changes.** Device creation,
   `.knxproj` import, export and `set_parameter_value`'s stored raw value
   are all untouched. The only thing that varies with language is
   displayed text.
6. **`com_object_view` is not given a language in this slice.** Its only
   caller bakes text into the project at creation time.
7. **Commit messages are written in the voice of Marvin**, the manically
   depressed robot from *The Hitchhiker's Guide to the Galaxy* — gloomy,
   world-weary — while every technical statement in them stays accurate
   and complete.
8. **`CLAUDE.md` wins over any session instruction about commit
   trailers:** commit as `github@knxbench.com`, and **never** add a
   `Co-Authored-By:` line of any kind.
9. **Gates before every commit** (all must be clean):
   `cargo fmt --all --check`;
   `cargo clippy --workspace --all-targets -- -D warnings`;
   `cargo test --workspace`;
   `cargo run -p xtask -- check-layering`;
   `cargo deny check`;
   `npm --prefix apps/knx-web run test`;
   `npx --prefix apps/knx-web tsc -p apps/knx-web/tsconfig.json --noEmit`.
   **Never run `npm run build`** — Vite's `outDir` clean step deletes the
   tracked `apps/knx-web/dist/.gitkeep`. `npm run test` does not build,
   which is why the `tsc --noEmit` gate is listed separately.
10. **Never `git add -A`.** The worktree contains a deliberately untracked
    `OriginalData` symlink. Add named paths only. `OriginalData/` is
    strictly read-only; extract nothing into it and write nothing to it.

## Environment facts

- `@testing-library/react` is **not** a dependency of `apps/knx-web`. The
  test idiom is `react-dom/client`'s `createRoot` + React's `act()` +
  native `dispatchEvent`, under a `// @vitest-environment happy-dom`
  pragma. See `ParameterPanel.test.tsx` and `SettingsPanel.test.tsx`.
- `apps/knx-web` has **no ESLint config**. Never add `eslint-disable`.
- Vitest does not process CSS: `import x from "./f.css?raw"` is the empty
  string. Read source with `node:fs`.
- `bc` is not installed. Sum Rust test counts from a single run with
  `grep -E "^test result" | awk '{p+=$4; f+=$6; i+=$8; n+=1} END {print n,p,f,i}'`.
- A 23 MB scratch product database built from
  `OriginalData/ProductDatabases` lives at
  `/home/knxbench/.claude/jobs/8098e9e6/tmp/scratch-products.sqlite`. Query
  it read-only with `sqlite3` to check a claim about real data. Do not
  copy it into the repository and do not add a test that depends on it.
- Scratch files go under `/home/knxbench/.claude/jobs/8098e9e6/tmp/`, never
  `/tmp`.

## Task 1 — `knx-productdb`: the overlay and the language queries

Files: `crates/knx-productdb/src/query.rs` only.

1. Add:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranslationLanguage {
    pub language: String,
    pub rows: i64,
}

/// Every language identifier any program in this database declares, with
/// its translation-row count, most rows first then identifier ascending.
pub fn translation_languages(conn: &Connection)
    -> Result<Vec<TranslationLanguage>, ProductDbError>;

/// The same, narrowed to one application program — languages are declared
/// per program, not per database (the corpus has programs declaring 2 and
/// 10), so a per-database answer is wrong for both.
pub fn program_translation_languages(conn: &Connection, program_id: &str)
    -> Result<Vec<TranslationLanguage>, ProductDbError>;
```

   Both are plain `GROUP BY language` counts over `translation`, ordered
   `ORDER BY COUNT(*) DESC, language ASC`.

2. Add a private helper that loads one program's overlay for one language
   in **one** query, not one per row:

```rust
/// `(ref_id, attribute_name) -> text` for one program and language.
/// Loaded once per `parameter_views` call: a single `ModuleDef` can own
/// hundreds of parameters, so a per-row lookup is the wrong shape — the
/// same reasoning `parameter_views`' own doc comment already gives.
fn translation_overlay(
    conn: &Connection,
    program_id: &str,
    language: &str,
) -> Result<HashMap<(String, String), String>, ProductDbError>;
```

   Restrict the query to display attributes:
   `WHERE program_id = ?1 AND language = ?2 AND attribute_name IN
   ('Text','FunctionText','SuffixText','VisibleDescription','Name')
   AND text IS NOT NULL`. `Value` must not be selected — Global
   Constraint 3.

3. Change the signature to
   `parameter_views(conn, program_id, language: Option<&str>)` and apply
   the overlay **per element, before `pick()`**:
   - `p.text` → `overlay.get((parameter_id, "Text"))` — note this is the
     `parameter` row's id, which the current query does not select; add
     `p.id` to the `SELECT` list and to `ParameterRawRow`.
   - `pr.text` → `overlay.get((parameter_ref_id, "Text"))`, i.e. `raw.id`.
   - Then `pick(p_text, pr_text)` exactly as today. `ValueLayer` gains no
     variant and keeps its current meaning: which structural layer won.
   - `p.name` → `overlay.get((parameter_id, "Name"))` when present. (The
     corpus has no `Name` rows on parameters; the lookup costs nothing and
     the attribute is legal.)
   - Enum options: `parameter_type_enum_options` takes the same overlay
     (pass it by reference) and overlays each row's `text` by that row's
     own `id` with attribute `Text`. Its `value` is **never** overlaid.
4. Update the two existing in-crate callers/tests and
   `crates/knx-productdb/tests/parameter_views_corpus.rs` to pass `None`.

Tests (in `query.rs`'s `mod tests`, using the fixture idiom of
`parse/translation.rs`'s existing test — a small `<KNX>` XML string
ingested with `ingest_program` into a temp database):

- `parameter_views_without_a_language_returns_the_untranslated_text`
- `parameter_views_with_a_language_returns_the_translated_text`
- `a_parameter_without_a_row_in_that_language_keeps_its_own_text`
  (same call as the previous test, second parameter untranslated)
- `a_translated_parameter_ref_text_still_beats_an_untranslated_parameter_text`
  — and `text_layer` is still `ValueLayer::ProgramRef`
- `enum_option_labels_are_translated`
- `a_value_translation_never_changes_a_stored_value` — fixture contains a
  `<Translation AttributeName="Value" …>` row; assert `enum_options`'
  `value` is the package's, not the translation's
- `translation_languages_orders_by_row_count_then_identifier`
- `program_translation_languages_returns_only_that_programs_set`

Report: the exact `cargo test -p knx-productdb` line, plus the full gate
list from Global Constraint 9.

## Task 2 — `apps/knx-server`: the language reaches both parameter endpoints

Files: `apps/knx-server/src/routes.rs`, `apps/knx-server/src/domain.rs`,
plus a test file under `apps/knx-server/tests/`.

1. `domain::parameter_panel_impl` and `domain::set_parameter_value_impl`
   take a new final argument `language: Option<&str>` and pass it to
   `parameter_views`. No other call site of `parameter_views` exists.
2. New route `GET /api/product-languages`, handler + a
   `domain::product_languages_impl`, returning
   `Vec<ProductLanguageDto { language: String, rows: i64 }>` from
   `query::translation_languages`. **With no product database configured
   it returns `200 []`, not an error** — "no database" is a normal state
   and the settings UI must still render. Follow `CatalogItemDto`'s
   `#[derive(serde::Serialize)] #[serde(rename_all = "camelCase")]`
   convention; do not introduce `ts-rs`.
3. Both parameter endpoints accept an optional `language` query parameter
   (`#[derive(Deserialize)] struct ParameterQuery { language: Option<String> }`,
   extracted with `Query`, following `CatalogItemsQuery`'s precedent).
   **The `POST` needs it too**: the write returns the same
   `ParameterPanelDto`, so without it every write resets the panel to the
   untranslated text. This is the single most likely defect in this task.

Tests, following `apps/knx-server/tests/http_parameter_panel.rs`'s setup:

- `product_languages_with_no_product_database_returns_an_empty_list`
- `parameter_panel_returns_translated_text_for_a_requested_language`
- `parameter_panel_without_a_language_returns_the_untranslated_text`
- `setting_a_parameter_value_keeps_the_requested_language`

Report: the full gate list, and the name of every test you added.

## Task 3 — `apps/knx-web`: the setting and the Settings control

Files: `apps/knx-web/src/productLanguage.ts` (new),
`apps/knx-web/src/productLanguage.test.ts` (new),
`apps/knx-web/src/api.ts`, `apps/knx-web/src/SettingsPanel.tsx`,
`apps/knx-web/src/SettingsPanel.test.tsx`.

1. `productLanguage.ts`, modelled on `theme.ts`:

```ts
export const PRODUCT_LANGUAGE_STORAGE_KEY = "knx-desktop:product-language";
export function loadProductLanguage(storage: Pick<Storage, "getItem">): string | null;
export function saveProductLanguage(storage: Pick<Storage, "setItem" | "removeItem">, language: string | null): void;
export function useProductLanguage(): [string | null, (language: string | null) => void];
```

   `null` means "package default" — untranslated, today's behaviour, and
   the default for anyone who never opens Settings. Unlike `theme.ts`
   there is no compile-time list to validate against: a persisted value is
   sent to the server as-is and falls back per string if the database does
   not know it. `saveProductLanguage(.., null)` removes the key rather
   than storing `"null"`.
2. `api.ts`: `export function productLanguages(): Promise<ProductLanguage[]>`
   over `GET /api/product-languages`, with
   `export interface ProductLanguage { language: string; rows: number }`.
3. `SettingsPanel.tsx`: a fourth `<select>`, labelled "Product data
   language", first option "Package default" (value `""` → `null`), then
   one option per language labelled `` `${language} (${rows} strings)` ``.
   Fetch the list on mount. When it is empty, render the select
   **disabled** with a single option reading "No product database
   installed" — a control that disappears teaches the user nothing.
   Persist through `useProductLanguage`.

Tests (`// @vitest-environment happy-dom`, `createRoot` + `act()`):

- `productLanguage.test.ts`: default is `null`; a saved value round-trips;
  saving `null` removes the key.
- `SettingsPanel.test.tsx`: renders one option per language returned by a
  mocked `api.productLanguages`, plus the "Package default" option;
  selecting one persists it; an empty list renders a disabled select with
  the explanatory option.

## Task 4 — `apps/knx-web`: the parameter panel actually uses it

Files: `apps/knx-web/src/api.ts`, `apps/knx-web/src/ParameterPanel.tsx`,
`apps/knx-web/src/ParameterPanel.test.tsx`.

1. `api.deviceParameters(deviceId, language?: string | null)` and
   `api.setParameterValue(deviceId, etsId, raw, language?: string | null)`
   append `?language=<encodeURIComponent(language)>` when the language is
   a non-empty string, and nothing at all when it is `null`/`undefined`.
2. `ParameterPanel` reads `useProductLanguage()`, passes the value to both
   calls, and refetches when it changes (add it to the existing effect's
   dependency list — do not add a second effect).
3. **Correct the label order** in `ParameterFieldRow`:
   `const label = field.text ?? field.name ?? field.etsId;` (it is
   `name ?? text ?? etsId` today). Spec section 3 carries the evidence:
   `Name` has zero translation rows and `Text` has 9915, both columns are
   populated on all 3557 corpus parameter rows, so without this the
   translated string is unreachable and the whole slice is inert.

Tests (extending `ParameterPanel.test.tsx`):

- the active language is passed to `api.deviceParameters`
- the active language is passed to `api.setParameterValue`
- a field with both `name` and `text` renders `text`

## Task 5 — documentation

Files: `docs/KNOWN_LIMITATIONS.md`, `docs/GAP_ANALYSIS_ETS.md`,
`docs/IMPLEMENTATION_STATUS.md`, `docs/ROADMAP.md`,
`docs/COMPATIBILITY.md` (only if it makes a claim this slice changes —
check, and say in the report what you found either way).

1. **`KNOWN_LIMITATIONS.md` §37** — rewrite the body honestly: partially
   resolved for parameter text, enum labels and the language list; still
   open for communication objects (baked at creation), for the UI chrome
   (T25), and for `parameter.suffix`, which is stored but displayed
   nowhere. Rename the header to `— partially resolved` **and update
   every link to its anchor in the same commit** (an em dash becomes a
   double hyphen in a GitHub slug; grep for the old slug before and after,
   and report both counts).
2. **A new limitation** for the ingestion gap this slice deliberately did
   not fix: `Languages` blocks in `Catalog.xml`, `Hardware.xml` and
   `knx_master.xml` are dropped on import — measured on
   `MDT_KP_AMI_AMS_03_Switch_Actuator_V31a.knxprod` as 40 (`Name`, catalog
   items, 5 languages), 30 (`Text`, 5 languages) and 1635 (`Text`, 24
   languages) `<Translation>` elements respectively. State that this is a
   "never silently discard" violation, that it needs a schema decision
   (`translation.program_id` is `NOT NULL` and a catalog item belongs to
   no program), and that it is tracked as its own task.
3. **`GAP_ANALYSIS_ETS.md`**: update Tier 6's **T26** entry to record what
   shipped and what did not, add the new ingestion task next to it, and
   update row **D10** — which stays **open**, because its UI-chrome half
   (T25) is untouched.
4. **`IMPLEMENTATION_STATUS.md`**: a dated entry, `Last updated:` bumped.
5. **`ROADMAP.md`**: the Internationalization section says "Neither track
   has a design spec yet" and "no cycle scheduled" — both are now false
   for T26. Fix exactly those statements; do not rewrite the section.

Do not touch any file under `docs/superpowers/` — dated specs and plans
are historical record.

Claim nothing this slice did not do. No KNX certification, no ETS
compatibility, no accessibility or conformance claim, and no statement
that translations are "fully supported" — they are read at exactly one
surface.
