# Backend localization paths (AR10 trace)

Status: trace of 2026-10-05, read from code on `origin/main` (`6fb50af8` +
AR10 slice 1) and pinned where noted by tests. It is the input for AR10's
remaining slices, not a claim that anything is "fully localized".

Evidence labels: **[D]** licensed *Project Schema23 v01.00.00* (KNX Standard
v3.0.0, local `knx-spec-kb` extraction); **[C]** read from code; **[T]**
pinned by a named test; **[V]** aggregate census of the local corpus.

## 1. Where a language can come from

| Source | Where | Status |
|---|---|---|
| Project-wide language | none [D]: `ProjectInformation` declares `Name`, `GroupAddressStyle`, numbers, dates, `CodePage`, … but no language | Import stores the placeholder `en` (KNOWN_LIMITATIONS §14). |
| Per-device initial-value language | `DeviceInstance/@InitialValueLanguage`, optional `xs:language` [D] | Absent from all three local reference projects (75 device instances) [V]. Not modelled; reported as an unknown attribute and retained, never promoted to a project language [T `project_language.rs`]. |
| Program / catalogue / hardware default language | `ApplicationProgram/@DefaultLanguage` (and the `default_language` columns of `catalog_item`, `hardware`) | Stored verbatim [C]; read by **nothing** outside parsing and migration [C]. 991 occurrences in the product data embedded in the three local reference projects: `en-US` 912, `de-DE` 64, `en` 15 [V]. |
| Requested display language | `?language=` on parameter, device-detail and catalogue routes; the web Settings "Product data language" | User choice; `None` means the package's own text. |
| New-project language | `POST /api/project/new` `language` (BCP-47 validated) | User choice. |

## 2. Where translated text is read

All overlays resolve the requested language through one function,
`knx_productdb::query::best_matching_language` (exact match, else locale
prefix `de` → `de-DE`, ties broken by lexicographically smallest identifier).
A miss falls through to the package's own untranslated column, never to a
second language [C].

| Surface | Query | Scopes | Fallback visible to the caller? |
|---|---|---|---|
| Parameter panel text and enum labels | `parameter_views`, `parameter_type_enum_options` | Program | **Yes since AR10 slice 2a** — `text_language`/`name_language`/`enum_option_languages` name the stored language that answered, `None` on a fallback; the panel DTO adds `sourceLanguage` (`ApplicationProgram/@DefaultLanguage`) [T] |
| Communication-object text/description | `com_object_view(s)` | Program | Yes — `text_translated`, `function_text_translated`, `visible_description_translated`; the server only overwrites product-layer text on a hit (§37) |
| Catalogue browser | `catalog_items` | Catalog | **No** |
| Device detail product block | `device_product` | Hardware, Catalog, Program | **No** |
| Master vocabulary | `datapoint_types`, `function_types`, `function_points`, `space_usages` | Master | **No** |
| Project text (`StringTable`) | `StringTable::resolve` | — | Never consulted: no production path inserts an entry, imports leave it empty [T] |

Ambiguous prefix matches (`de` with both `de-AT` and `de-DE` stored) are
resolved by the documented tiebreak; the caller is not told which stored
language answered [C].

## 3. Backend diagnostics and prose

Structured, translatable kinds exist for `ParameterDiagnostic` and
`CreationDiagnostic`; the documentation report has EN/DE chrome. Other
server-composed prose stays English by the rule in KNOWN_LIMITATIONS §66 [C].

## 4. Consequences for AR10

- **§14** has no source to lift it from: the schema has no project language,
  the one per-device attribute is absent locally and is reported, and the
  placeholder is never consulted. Accepted as a boundary, pinned by tests.
- **§37/§64:** expose the fallback instead of hiding it — per overlaid
  text, which stored language answered (or that the package's own text was
  used), and the package's declared `DefaultLanguage` for that untranslated
  text. Done for the parameter panel (slice 2a:
  `parameter_views_name_the_language_that_answered_and_expose_the_fallback`,
  `parameter_panel_exposes_the_answering_language_and_the_fallback`); the
  catalogue, device-product and master surfaces are next. Server DTOs only;
  showing it is UI-owner work.
- **§66** stays the documented rule; new strings follow it.
