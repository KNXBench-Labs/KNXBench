# T26 (first slice) — reading the translations that have been sitting in the database all along

- **Date:** 2026-09-12
- **Status:** design, ready for implementation
- **Closes:** the parameter half of **T26** in
  [GAP_ANALYSIS_ETS.md](../../GAP_ANALYSIS_ETS.md)'s Tier 6; downgrades
  [KNOWN_LIMITATIONS.md §37](../../KNOWN_LIMITATIONS.md#37-imported-translations-are-stored-but-never-read-and-the-ui-is-english-only)
  from "never read" to "read at one surface". Gap **D10** stays open: its
  UI-chrome half is T25 and is not touched here.
- **Scope:** `crates/knx-productdb`, `apps/knx-server`, `apps/knx-web`.
  No `.knxproj` importer, no `knx-core` change, no schema migration — the
  `translation` table already exists and is already populated.

## Why now

§37 is the cheapest open gap in the backlog to close *partially* and the
most expensive to keep claiming is fine: the data is already on disk,
already parsed, already indexed, and read by nothing. Measured on the
scratch product database built from `OriginalData/ProductDatabases`
(6 application programs, 10 catalog items, 401 communication objects):

| Element the `ref_id` points at | rows | attributes present |
| --- | --- | --- |
| `parameter` | 10794 | `Text` 9915, `SuffixText` 879 |
| `parameter_type_enum` | 9701 | `Text` 9701 |
| `com_object` | 2513 | `Text`, `FunctionText` |
| `com_object_ref` | 1067 | `Text`, `FunctionText` |
| `parameter_ref` | 475 | `Text` 475 |
| `application_program` | 35 | `Name` |
| *nothing stored* | 1299 | parameter blocks/pages/channels (`_BI-`, `_PB-`, `_PS-`) |
| **total** | **25884** | |

Per-language: `de-DE` 7385, `en-US` 7327, `it-IT` 3726, `fr-FR` 3723,
`es-ES` 3703, then a long tail of eight identifiers with 4 rows each
(`sv-SE`, `ru-RU`, `nl-NL`, `nb-NO`, `el-GR`, …). Languages are declared
per application program, not per database: the six programs in the corpus
declare 10, 5, 5, 5, 4 and 2 languages respectively. Any design that
assumes one language set for a whole `.knxdb` is wrong on the first real
package.

`parameter` + `parameter_type_enum` + `parameter_ref` is **20970 of the
25884 rows, 81%** — and all three are read *live* by the parameter panel
(`query::parameter_views` → `domain::parameter_panel_impl` →
`/api/device/{id}/parameters`), which is why this slice starts there
rather than with communication objects.

## What gets built

### 1. `knx-productdb`: a translation overlay, applied where `pick()` already is

`parameter_views` gains a language:

```rust
pub fn parameter_views(
    conn: &Connection,
    program_id: &str,
    language: Option<&str>,       // None = the package's own untranslated text
) -> Result<Vec<ParameterView>, ProductDbError>;
```

The overlay is applied **per element, before `pick()`**, not after. A
`Translation` in a `.knxprod` overrides one attribute of one element
(`parse/translation.rs`'s own doc comment: "per-language overrides of one
attribute on one program element, keyed by `(program_id, language,
ref_id, attribute_name)`"), so the translated value of `parameter.text`
belongs at the `Program` layer and the translated value of
`parameter_ref.text` at the `ProgramRef` layer. Overlaying first and
`pick()`ing second keeps `ValueLayer`'s existing meaning exactly — which
structural layer won — and needs **no new `ValueLayer` variant**. A
translated `ParameterRef/@Text` still beats an untranslated
`Parameter/@Text`, which is what the package says should happen.

Resolution per attribute, in order:

1. `translation(program_id, language, ref_id, attribute_name)` when
   `language` is `Some` and a row exists,
2. otherwise the column already parsed from the non-translated attribute.

There is no second-language fallback chain. If the user asks for `it-IT`
and one string has no Italian row, that string renders in whatever the
package's untranslated attribute holds — which is what ETS packages
themselves treat as the default text. Falling back to `en-US` instead
would invent a preference the package never stated.

**Only display attributes are overlaid.** The allowed set for this slice
is exactly `Text` and, where the view exposes it, `FunctionText`,
`SuffixText`, `VisibleDescription` and `Name`. **`Value` is never
overlaid.** 424 `Value` translation rows exist in the corpus and match no
stored element at all, but the rule matters independently of that count:
a parameter's value is a key written into the project file
(`ParameterFieldDto.value`, `set_parameter_value`), and translating a key
by display language would corrupt stored project data the moment someone
switched languages. Data integrity outranks completeness of display.

Two new queries, both read-only:

```rust
/// Every language identifier any program in this database declares, with
/// how many translation rows it has, most rows first then identifier.
pub fn translation_languages(conn: &Connection)
    -> Result<Vec<TranslationLanguage>, ProductDbError>;

/// The same, narrowed to one application program.
pub fn program_translation_languages(conn: &Connection, program_id: &str)
    -> Result<Vec<TranslationLanguage>, ProductDbError>;

pub struct TranslationLanguage { pub language: String, pub rows: i64 }
```

The row count is part of the API, not decoration: a language with 4 rows
out of 25884 is *offered by the package* and will render as ~100%
fallback, and a user who picks it deserves to see why nothing changed.

`com_object_view` is **not** given a language in this slice — see
non-goals.

### 2. `apps/knx-server`: the language reaches the two parameter endpoints

- `GET /api/product-languages` → `[{ "language": "de-DE", "rows": 7385 }, …]`,
  served from `translation_languages`. Returns `[]` — not an error — when
  no product database is configured, because "no database" is a normal
  state of this application and the settings UI must still render.
- `GET /api/device/{id}/parameters?language=de-DE` — optional; absent
  means untranslated, exactly as today.
- `POST /api/device/{id}/parameters?language=de-DE` — the write returns
  the same `ParameterPanelDto` (D24's "same response, no second GET"), so
  it needs the same parameter or every write would silently reset the
  panel to English. This is the easiest thing in this slice to forget and
  the most visible when it is forgotten.

`domain::parameter_panel_impl` and `domain::set_parameter_value_impl` take
`language: Option<&str>` and pass it through. No other call site of
`parameter_views` exists.

### 3. `apps/knx-web`: one setting, one select, one refetch

- **`productLanguage.ts`**, modelled on `theme.ts` (`loadThemeId`/
  `saveThemeId`/`useThemeId` with a storage-injection signature so it is
  testable without touching real `localStorage`). Storage key
  `knx-desktop:product-language`. The default is **`null`, "Package
  default"** — untranslated, which is today's behaviour, so nothing
  changes for anyone who never opens Settings. Unlike `theme.ts` the valid
  set is not a compile-time constant, so a persisted value is not
  validated against a list at load time: it is sent to the server as-is
  and falls back per-string if the database does not know it.
- **`SettingsPanel.tsx`** gains a fourth `<select>`, "Product data
  language", populated from `/api/product-languages`, each option labelled
  `de-DE (7385 strings)`. When the list is empty the select renders
  disabled with a "no product database installed" option, rather than
  vanishing — a control that disappears teaches the user nothing.
- **`ParameterPanel`** sends the active language on both the GET and the
  POST, and refetches when it changes.
- **`ParameterFieldRow`'s label order is corrected to `text ?? name ??
  etsId`** (it is `name ?? text ?? etsId` today). Without this the slice
  ships inert: `Name` is the authoring identifier and carries **zero**
  translation rows in the corpus, while `Text` is the display label and
  carries 9915. Both columns are populated on every one of the 3557
  parameter rows measured, so `name` always wins today and the translated
  string would never be reachable. The package's own data makes the point
  — one row reads `name = "General"`, `text = "Allgemein"`: the panel
  currently shows the internal label and hides the one the manufacturer
  wrote for users. This is a visible change for every existing user with a
  product database, in every language including none, and it gets its own
  test rather than riding along unremarked.

### 4. What this does not fix

Stated plainly so §37's rewritten text is honest:

- **Communication-object text stays untranslated, including in new
  devices.** The only consumer of `com_object_view` is device creation
  (`domain.rs`), which *bakes* the text into the project at creation time.
  Translating there would make stored project content depend on a display
  setting, and re-translating existing devices on a language switch is a
  data-model question (which text is authoritative?) that this slice does
  not answer. §37 stays open for it.
- **The UI chrome stays English.** That is T25, a separate task, with its
  own undecided library question.
- **No locale-prefix matching.** `de` does not resolve `de-DE`. Only
  identifiers the database actually declares are offered, so there is
  nothing to guess.
- **No auto-detection from `navigator.language`.** The default is
  deliberately "package default", not "whatever the browser says", so the
  displayed text never changes without a user action. Detection belongs
  with T25's locale work.
- **`parameter.suffix` is still not displayed anywhere**, so its 879
  `SuffixText` translations remain unread even after this slice.
- **1299 translation rows (5%) target elements this database does not
  store** — parameter blocks, pages and channels from the dynamic UI
  structure. They stay unread and this slice does not change that.
- **The project's own `Language`** (`Project::new`, handed a placeholder
  `"en"` by both importers) is untouched. An active *display* language is
  not a claim about the project's language.

## Testing

`crates/knx-productdb` (unit, in `query.rs`, against a fixture program
ingested exactly as `parse/translation.rs`'s existing test does):

1. `parameter_views(.., None)` returns the untranslated text — the
   existing behaviour, asserted so the overlay cannot change it.
2. `parameter_views(.., Some("de-DE"))` returns the German `Text` for a
   parameter that has one.
3. A parameter with **no** row in the requested language keeps its
   untranslated text in the same call that translates its neighbour.
4. A translated `ParameterRef/@Text` still beats an untranslated
   `Parameter/@Text`, and `text_layer` is still `ProgramRef` — the
   layering is unchanged by translation.
5. Enum option labels are translated through `parameter_type_enum.id`.
6. A `Value` translation row does **not** change `enum_options`' value or
   any stored value — the data-integrity rule, asserted, not just
   documented.
7. `translation_languages` orders by row count descending, and
   `program_translation_languages` returns only that program's set (the
   corpus has programs declaring 2 and 10 languages; a per-database answer
   would be wrong for both).

`apps/knx-server` (HTTP, following `tests/http_parameter_panel.rs`):

8. `GET /api/product-languages` with no product database configured
   returns `200 []`.
9. `GET /api/device/{id}/parameters?language=<x>` returns the translated
   field text; without the parameter, the untranslated one.
10. `POST /api/device/{id}/parameters?language=<x>` returns a DTO whose
    text is still translated — the regression this slice is most likely
    to ship with.

`apps/knx-web` (vitest, `createRoot` + `act()` + native `dispatchEvent`
under `// @vitest-environment happy-dom`; `@testing-library/react` is not
a dependency of this app):

11. `productLanguage.ts`: default is `null`, a persisted value round-trips,
    and a cleared value returns to `null`.
12. `SettingsPanel`: renders one option per language from a mocked
    `/api/product-languages`, and persists the choice on change.
13. `SettingsPanel`: renders a disabled select with an explanatory option
    when the list is empty.
14. `ParameterPanel`: the active language is sent on both the GET and the
    POST.
15. `ParameterPanel`: a field with both `name` and `text` renders `text`
    — the label-order correction, asserted directly, since it is the only
    reason any of the rest of this slice is visible.

## Non-goals

- No new npm dependency, and no i18n library — this slice adds one
  `<select>` and one query parameter, not a message catalogue.
- No schema migration. The `translation` table, its primary key and its
  `translation_lookup` index are used exactly as they were created.
- No change to device creation, `.knxproj` import, export, or anything
  that writes to a project file.
- No claim of ETS parity, and no fix for the **ingestion** gap this design
  work uncovered. This project's parser only ingests `Languages` blocks
  inside `ApplicationProgram`: `parse/catalog.rs`, `parse/hardware.rs` and
  `parse/master.rs` never call `insert_translations` (verified by grep).
  Every `.knxprod` carries more than that. Measured by extracting
  `OriginalData/ProductDatabases/MDT_KP_AMI_AMS_03_Switch_Actuator_V31a.knxprod`
  and counting `<Translation>` elements per file:

  | File | `<Translation>` elements | languages | attribute |
  | --- | --- | --- | --- |
  | `M-0083/Catalog.xml` | 40 | 5 | `Name` (catalog-item names) |
  | `M-0083/Hardware.xml` | 30 | 5 | `Text` |
  | `knx_master.xml` | 1635 | 24 | `Text` (master data) |
  | `M-0083/*_A-*.xml` | ingested today | 5 | `Text`, `SuffixText`, … |

  Those 1705 rows are **dropped on import** — not preserved-and-unread
  like the program-level ones, genuinely discarded — which is a
  `CLAUDE.md` "never silently discard information" violation and
  outranks, in principle, the reading gap this slice closes. It is not
  folded in here because storing them needs a schema decision the
  `translation` table cannot express as it stands (`program_id TEXT NOT
  NULL`, and a catalog item belongs to no program), i.e. a migration, a
  parser change and an import-report change — a slice of its own. This
  slice's documentation task records it as a new limitation and a new
  backlog task rather than leaving it in a design document nobody reads
  twice.
