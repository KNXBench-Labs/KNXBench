# 2026-09-12 — T26: the product-data translations finally get a reader

Architecture log for the T26 cycle, branch `t26-product-language`
(`c19aa94`..`0c51ca9`, nine commits), merged to `main` as `0912ea5`
(`--no-ff`, 20 files changed, 2253 insertions, 119 deletions).

Design: `docs/superpowers/specs/2026-09-12-product-data-language-design.md`.
Plan: `docs/superpowers/plans/2026-09-12-product-data-language.md`.

Closes the data half of gap **D10** and partially resolves
`docs/KNOWN_LIMITATIONS.md` §37. Opens **§64** and backlog item **T32**.

## The problem this slice attacks

`knx-productdb`'s `translation` table has been populated on every product
import since the database layer existed — 48,057 rows on the reference
corpus — and nothing has ever read a single one of them. Every parameter
label, every enum option text, every suffix the user has ever seen came
from the application program's own untranslated attributes, whatever
language those happened to be written in. The data was imported, stored,
migrated, and ignored.

## What changed architecturally

One overlay function at the bottom of the stack, one optional query
parameter threaded through the middle, one persisted setting at the top,
and one shared store so the setting actually reaches the surface that
renders it.

### 1. `knx-productdb` — the overlay, and where it is allowed to apply

`crates/knx-productdb/src/query.rs` gained a private
`translation_overlay(conn, program_id, language) -> HashMap<(String, String), String>`
keyed by `(ref_id, attribute_name)`. Its SQL carries an explicit allow-list:

```sql
attribute_name IN ('Text', 'FunctionText', 'SuffixText', 'VisibleDescription', 'Name')
  AND text IS NOT NULL
```

That list is the architectural decision of the whole slice. `Value` is a
translatable attribute in the KNX schema, and translating it would rewrite
the stored content of a parameter — an enum option whose `Value` became
"An" instead of `1` would be written into a project file as the string
"An". The allow-list makes that structurally impossible rather than
merely unusual: there is no code path by which a `Value` row can be
loaded into the overlay at all, so no later caller can reintroduce the
bug by forgetting a convention.

`parameter_views(conn, program_id, language: Option<&str>)` builds the
overlay once per call and applies it per element **before** `pick()`
chooses the winning candidate, so a translated element competes on the
same footing as an untranslated one. `parameter_type_enum_options` gained
an `overlay: Option<&HashMap<(String, String), String>>` argument and now
selects `id, value, text`, overlaying only `text`.

Two public listing functions were added:
`translation_languages(conn)` and
`program_translation_languages(conn, program_id)`, both returning
`TranslationLanguage { language, rows }` ordered by row count descending
then language ascending. `program_translation_languages` has no caller
outside its own tests — a deliberate, documented seam for the later
communication-object slice, not dead weight.

`language: Option<&str>` throughout, with `None` meaning "the program's
own attributes". There is **no fallback chain**: asking for `de-DE` when
only `de` exists returns the untranslated text, not `de`. That is a
decision, not an oversight — a fallback chain needs a language-tag
matching policy (RFC 4647 lookup, or something narrower), and inventing
one silently would be exactly the kind of undocumented behaviour this
project's rules forbid. Recorded as such in the design.

### 2. `apps/knx-server` — one query parameter, both assembly paths

`domain.rs` threads `language: Option<&str>` through
`assemble_parameter_panel`, `parameter_panel_impl` and
`set_parameter_value_impl` — the last one on **both** of its assembly
paths, so the panel returned after a write is translated exactly like the
panel returned by a read. `create_device_impl` still calls the untouched
`com_object_view` with no language; communication objects are out of this
slice and say so.

`routes.rs` gained `ParameterLanguageQuery { language: Option<String> }`
on both the `GET` and the `POST`. On the `POST` the `Query` extractor
goes **before** the `Json` body extractor — axum requires the body
extractor last, and putting `Query` after it fails to compile with an
error that does not name the real cause.

`GET /api/product-languages` returns `ProductLanguageDto { language, rows }`
(camelCase). With no product database configured it returns `200 []`
rather than the `Err("no product database configured")` that
`catalog_manufacturers_impl` returns for the same precondition. The
asymmetry is intentional and commented at the call site: a Settings panel
must render on a fresh install. A genuine database error still propagates
as `Err` through `translation_languages`' own `?`.

### 3. `apps/knx-web` — and the one seam no task-level review could see

`productLanguage.ts` persists the choice under
`knx-desktop:product-language`; `api.ts` appends `?language=` through a
`languageQuery()` helper built on `URLSearchParams` (nothing appended for
`null`/`undefined`/`""`); `SettingsPanel.tsx` gained a fourth
`settings-field` select fed by `App.tsx`'s mount-only
`api.productLanguages()` fetch; `ParameterPanel.tsx` reads the language
and lists it in its load effect's dependency array.

That shipped as five tasks, each reviewed clean. The whole-branch review
then found the blocker none of them could have found alone:
`useProductLanguage()` was a plain `useState` + `useEffect`, called
independently at `App.tsx:81` and `ParameterPanel.tsx:204`.
`Inspector.tsx:504` renders `<ParameterPanel deviceId={detail.id} />`
with no `key`, so the panel is reused across device switches and never
remounts. Picking a language in Settings updated App's copy and
`localStorage`, and the already-open panel kept showing English — for
every subsequent device the user clicked, until the whole Inspector
unmounted. The one thing the slice exists to deliver.

`useThemeId`/`useMotion` escape this only because they broadcast through
a `document.documentElement` attribute that CSS reads; every consumer
sees one value through the DOM. A server-side text selection cannot use
that mechanism — no stylesheet can pick a translation.

Fixed in `e26ce82` by giving `productLanguage.ts` a single source of
truth: a module-level `cached: string | null | undefined`
(`undefined` = not yet seeded) plus a subscriber `Set`, read through
React 19's built-in `useSyncExternalStore`. `getSnapshot` seeds from
`localStorage` lazily on its first call, not at module-evaluation time,
so a test that seeds storage before rendering still sees its value. The
setter writes `localStorage`, updates `cached` and notifies every
subscriber. `useProductLanguage()`'s signature is unchanged, so neither
call site changed shape, and `loadProductLanguage`/`saveProductLanguage`
keep their exported signatures.

Prop drilling through `Inspector` was rejected: `Inspector` has no use
for the value and would have become a pass-through for it. A context
provider was rejected as more machinery for the same result. No new
dependency was added — `useSyncExternalStore` is part of `react`.

`resetProductLanguageForTests()` is exported and wired into the
`afterEach` of `ParameterPanel.test.tsx` and `SettingsPanel.test.tsx`,
because clearing `localStorage` no longer un-seeds the module cache; a
test that passed only by running first would have been a defect.

## Tests

- `crates/knx-productdb`: 8 new, including
  `a_value_translation_never_changes_a_stored_value`, which pins the
  allow-list's whole purpose.
- `apps/knx-server/tests/http_product_language.rs`: 6 tests. The last,
  `enum_write_with_a_language_keeps_the_raw_value_but_translates_its_label`,
  POSTs `{"etsId": "P-2_R-1", "raw": "1"}` to
  `/api/device/1/parameters?language=de-DE` and asserts the returned
  `value` is exactly `"1"` **and** the matching option's `text` is
  `"An"` — the exact shape a `Value` translation would break. Proved
  meaningful by injection: overlaying the option's `value` made the write
  fail validation with `left: 400, right: 200`.
- `apps/knx-web`: 9 new plus 1 renamed. `productLanguage.test.ts` was
  renamed to `productLanguage.test.tsx` — the regression test renders
  real JSX, and esbuild only parses JSX in `.tsx`/`.jsx` regardless of
  the `@vitest-environment` pragma. Git records it as a rewrite rather
  than a rename at the default similarity threshold (31%); `-M10%`
  detects it.
- The blocker's regression test mounts a reader and a writer under one
  `createRoot`, clicks via native `dispatchEvent`, and asserts
  `readerMounts === 1` so a remount cannot make it pass. Revert check
  against the pre-fix implementation:
  `AssertionError: expected '(default)' to be 'de-DE'`.

## Gates on merged `main` (`0912ea5`)

`cargo fmt --all --check` clean; `cargo clippy --workspace --all-targets
-- -D warnings` clean; `cargo test --workspace` **989 passed / 0 failed /
3 ignored across 73 `test result` lines**; `cargo run -p xtask --
check-layering` ok; `cargo deny check` ok; `npm --prefix apps/knx-web run
test` **245 passed across 26 files**; `npx --prefix apps/knx-web tsc -p
apps/knx-web/tsconfig.json --noEmit` clean.

## What this slice did **not** do, and the new limitation it opened

- **Communication objects are still untranslated.** `com_object_view` was
  deliberately untouched. §37 is therefore *partially* resolved, not
  closed, and D10 stays open.
- **UI chrome is still English-only.** That is T25, a separate track.
- **No fallback chain, no validation of a requested language.** An
  unknown language tag yields untranslated text rather than an error.
- **§64, new:** `Languages` blocks that sit outside an application
  program are discarded on import, because `translation.program_id` is
  `TEXT NOT NULL` and those blocks have no program to belong to. Measured
  on `MDT_KP_AMI_AMS_03_Switch_Actuator_V31a.knxprod`: Catalog.xml 40
  translations across 5 languages (`Name`), Hardware.xml 30 across 5
  (`Text`), knx_master.xml 1635 across 24 (`Text`) — **1705 rows
  silently dropped**. `CLAUDE.md` says never silently discard
  information, so this is written down rather than tolerated quietly, and
  tracked as backlog item **T32**.

## Parked findings

Two rulings were recorded rather than driving a second fix round.

1. **No test pins `getSnapshot`'s caching contract.** The re-reviewer
   showed by experiment that a `getSnapshot` re-reading `localStorage` on
   every call passes all four tests with no React warning, because the
   snapshot is a primitive compared by `Object.is`. The brief's claim
   that re-reading "makes React loop forever" is true for object
   snapshots, not this one. A test pinning the cache would pin an
   implementation detail and would fire spuriously the day someone adds a
   `storage`-event listener for multi-tab sync.
2. **`IS_REACT_ACT_ENVIRONMENT` unset in the renamed test file**, and
   **`resetProductLanguageForTests()` has no precedent.** 13 of the 17
   rendering test files under `apps/knx-web/src` already leave the flag
   unset, so setting it in one file would make that file the outlier; the
   warning appears only under the verbose reporter. The reset export has
   no precedent only because this codebase has no hook-testing utility at
   all.
