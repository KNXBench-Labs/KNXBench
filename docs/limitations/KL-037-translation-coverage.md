# Translation coverage

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-037)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

<a id="37-imported-translations-are-stored-but-never-read-and-the-ui-is-english-only--partially-resolved-2026-09-12"></a>
## 37. Translations reach selected surfaces, not every imported text or UI output

**Resolved.** Parameter text, parameter-ref text, and enum option labels
are read from the `translation` table at exactly one surface: the device
parameter panel. `knx-productdb`'s `parameter_views(conn, program_id,
language)` and `parameter_type_enum_options` take an `Option<&str>`
language and, when set, overlay the requested language's `Text`,
`FunctionText`, `SuffixText`, `VisibleDescription`, and `Name` rows over
the package's own untranslated attribute before `pick()` runs —
`ValueLayer`'s meaning is unaffected. `apps/knx-server` exposes `GET
/api/product-languages` (the database-wide language list, `200 []` with
no product database installed) and an optional `?language=` on both the
GET and the POST of `/api/device/{id}/parameters`. `apps/knx-web`
persists the chosen language as a per-user setting
(`productLanguage.ts`, `knx-desktop:product-language` in
`localStorage`, default `null` meaning "package default"), surfaces it
as a "Product data language" select in the Settings panel, and
`ParameterPanel` sends it on every load and every write.

**Also resolved, 2026-09-12 (T33).** Communication-object text is read
too now, at a second, narrower surface. `knx-productdb`'s
`com_object_view(conn, program_id, com_object_ref_id, language)` gained
the same `Option<&str>` overlay `parameter_views` already had, applied
before `pick()`: a `ComObject`-scope translation is keyed by the
`ComObject`'s own id, a `ComObjectRef`-scope one by the `ComObjectRef`'s
id, and exactly `Text`, `FunctionText` and `VisibleDescription` are ever
overlaid on `ComObjectView` — the same three attributes, nothing new.
Of those three, only `Text` and `VisibleDescription` go anywhere:
`apps/knx-server`'s `GET /api/device/{id}?language=` reads `view.text`
and `view.visible_description` and overwrites
`ComObjectNode::name`/`description`, but **only** where the stored
`Override<Text>`'s layer is `Layer::Program` or `Layer::ProgramRef` —
values the product database itself supplied. **Narrowed further on
2026-09-12 (T34, finding M6):** that layer condition is necessary but no
longer sufficient. The overwrite also requires `view.text_translated` /
`view.visible_description_translated`, so an overlay *miss* — a requested
language with no `translation` row for that attribute — now leaves the
project's own resolved text standing instead of replacing it with the
product database's untranslated column. `view.function_text` is
computed and then discarded at that call site: `ComObjectNode`
(`crates/knx-projection/src/lib.rs`) has no field to hold it, `enrich()`'s
`apply()` (`crates/knx-productdb/src/enrich.rs`) never stores it into a
project either. `knx-report` now accepts an English/German report language
and caller-composed product data, but its communication-object path
(`crates/knx-report/src/render.rs`) still renders `ComObjectNode::name`/
`description` straight through `build_device_detail`, which takes no
report language. Communication-object text and description therefore remain
language-insensitive even though report chrome and selected product data are
language-aware. `FunctionText` is still unread at every
surface, the same honest status this section already gives `SuffixText`
below. `Layer::Instance`, `Layer::Inferred` and `Layer::UserEdit` are
project-authored (the first and third are exported to `.knxproj`) and
are shown back verbatim regardless of the selected language, never
translated. `apps/knx-web`'s Inspector sends the persisted
product-language setting on every device-detail fetch and refetches when
it changes mid-selection, guarded against an older language's response
landing after a newer one's.

The Inspector's own description editor
(`apps/knx-web/src/Inspector.tsx`'s `ComObjectDescriptionField`, around
lines 112-125) seeds its input from that same displayed value, which
with a language selected is the translated `VisibleDescription`. Saving
it issues `Command::SetComObjectDescription`, which writes
`Layer::UserEdit` — and `UserEdit` values are among the layers exported
to `.knxproj`. So a translated string can become project data, but only
through an explicit save; this is not new behaviour (the same field
pre-filled from untranslated text before T33) and not a bug, just the
one place display and storage meet.

Re-measured, not assumed, on the same package §64 uses:
`MDT_KP_AMI_AMS_03_Switch_Actuator_V31a.knxprod`'s application program
`M-0083_A-0317-31-7DC6` carries 53 `ComObject`-scope `Text` and 50
`ComObject`-scope `FunctionText` translations per language, across all
five declared languages (`de-DE`, `en-US`, `fr-FR`, `es-ES`, `it-IT`) —
already ingested since T26/T32, now finally read (`Text` and
`VisibleDescription` only, per the `FunctionText` correction above). That
same package carries zero `ComObjectRef`-scope `Text` and 39
`ComObjectRef`-scope `FunctionText` translations per language — a
per-package figure, not the whole corpus. Re-measured directly against
the installed database for this pass
(`~/.local/share/knx/products.sqlite`, `sqlite3`, joining `translation`
against `com_object_ref` by `(scope_id, ref_id) = (program_id, id)`, of
12 installed application programs total): 3,781 `ComObjectRef`-scope
`Text` rows spanning 8 programs and 692 `ComObjectRef`-scope
`VisibleDescription` rows spanning 6. A handful translate a
`ComObjectRef` that declares no structural text of its own — e.g. program
`M-006A_A-0001-22-26C0-O0079`, ref `_O-0_R-10001`, whose fr-FR `Text` row
reads "sortie - Lumière" while that `ComObjectRef`'s own `Text` column is
empty and only its parent `ComObject` supplies "Ausgang - Licht" —
so the overlay resolves at the `ProgramRef` layer where the untranslated
value would otherwise have come from `Program`. Whether ETS treats a
`ComObjectRef`-scope translation of an attribute the `ComObjectRef`
itself never declared the same way is unattested; nothing here claims it
does.

**Narrowed again, backend only (2026-09-13, D10 slice 1, branch
`d10-master-translations`).** The locale-prefix gap named a few
paragraphs below — a stored `de` not matching a package's `de-DE` rows
— is closed at the query layer: `crates/knx-productdb/src/query.rs`'s
`best_matching_language` now lives in exactly one place and is the
resolution step behind every overlay in the file, old and new alike
(`translation_overlay`, `catalog_overlay`, `master_text_overlay`, and
anything built on top of them), with its own dedicated unit tests —
exact match preferred over a prefix match, `de` matches `de-DE`, and a
hypothetical `deX` does not, proving the match requires the `-`
separator rather than a bare string prefix. No caller sends a bare
primary-language tag yet: `apps/knx-web`'s language pickers populate
their options from the exact tags a package actually stored, so nothing
today exercises the new prefix path end to end. This closes the backend
half of the gap, not the full round trip — a frontend that let a user
type or detect a bare `de` would light the rest of it up for free. See
[§64](../KNOWN_LIMITATIONS.md#64-languages-blocks-outside-an-application-program-are-discarded-on-import)
for this same slice's other two pieces: a reader for `Master`-scope
`DPST-*`/`DPT-*` translations, and per-scope translation counts in the
import report.

**Still open.** Device creation and `enrich()` still bake untranslated
text into the project file — deliberately: translating there would make
the *stored project* depend on a display setting, the same integrity
line T26 was careful not to cross for parameter values, and T33 did not
cross it either. The UI chrome itself, tracked separately as **T25**, is
no longer hard-coded English — it shipped 2026-09-12, see the T25 entry
in [GAP_ANALYSIS_ETS.md](https://github.com/KNXBench-Labs/KNXBench/blob/994ac08c0328/docs/GAP_ANALYSIS_ETS.md)'s Tier 6 — but this closes
only the *chrome* half of D10; the data half's own residue below is
unaffected. `knx_core::string_table`'s `StringTable`
still has no resolver anywhere except `build_device_detail`'s own
`project.strings.default_language()` call (`crates/knx-projection/src/lib.rs`)
— a fixed default, not a user choice — so `LocalizedString` resolution
against the *selected* product language does not exist; the com-object
overlay above works entirely by substituting `knx-productdb` text before
it reaches that call, not by teaching the string table anything. A
project's own `Language` field is still the placeholder `"en"` both
importers hand `Project::new`, and remains unread by anything. `Value`
translations are deliberately never applied, for the identical
stored-data-integrity reason: a parameter's value is a key written into
the project file, not display text. `parameter.suffix` is stored but
displayed nowhere, so all 879 `SuffixText` rows measured for this
slice's design spec remain unread. The backend query layer now matches a
stored `de` selection against a package's `de-DE` rows (D10 slice 1,
above), but no caller exploits it and there is still no
`navigator.language` detection — the setting defaults to "package
default" and stays there until a user picks explicitly.

**Lifted when.** Partially, 2026-09-12: first the parameter panel (T26's
first slice), then communication-object text (T33, same day), then the
UI chrome itself (T25, same day — see its own entry in
[GAP_ANALYSIS_ETS.md](https://github.com/KNXBench-Labs/KNXBench/blob/994ac08c0328/docs/GAP_ANALYSIS_ETS.md)'s Tier 6, closing more of gap
**D10**). What remains is a later T26/T33 follow-up: the project's own
`Language` field, and `StringTable`/`LocalizedString` resolution against
a user-selected language — neither touched by any slice so far; every
overlay added is `knx-productdb`-side only. The ingestion gap T26's first
slice deliberately did not fix has since been closed by T32 (2026-09-12):
`Catalog.xml`, `Hardware.xml` and `knx_master.xml` translations are
ingested, and the catalog browser reads the catalog-scope ones. What is
ingested but still read by nothing — hardware and master text — is
recorded in
[§64](../KNOWN_LIMITATIONS.md#64-languages-blocks-outside-an-application-program-are-discarded-on-import).

**Update 2026-10-05 (AR10 slice 2a): the parameter panel no longer hides
its fallback.** `parameter_views` now says, per field text, name and enum
label, which stored language answered a request (`de` → `de-DE`) or that the
package's own text was kept (`None`); `GET/POST /api/device/{id}/parameters`
carries this as `textLanguage`, `nameLanguage`, `enumOptions[].language` and
the panel-level `sourceLanguage` (`ApplicationProgram/@DefaultLanguage`,
verbatim, `null` when undeclared). No text, value or fallback rule changed.
Catalogue, device-product and master surfaces still fall back silently.
The parameter panel shows the markers (UI owner, 2026-10-05): with a product
language selected, a field whose shown label is the package's own text gets
an "Untranslated (<sourceLanguage>)" badge, one whose label is translated
but some option labels are not gets "Options untranslated", and a panel line
counts them; with *Package default* nothing is marked. Trace:
[research/backend-localization-paths.md](../research/backend-localization-paths.md).

**Update 2026-10-05 (AR10 slice 2b): no backend overlay hides its fallback
any more.** The catalogue (`GET /api/catalog/items`: `nameLanguage`,
`visibleDescriptionLanguage`, `sourceLanguage`), the device-detail product
block (`product.catalog`: `product_text_language`,
`catalog_item_name_language`, `application_name_language`, and the three
`*_source_language` declared by `Product`, `CatalogItem` and
`ApplicationProgram`) and the master readers (`text_language`; on the wire as
the com-object `dpt_text_language`) now say which stored language answered or
that the package's own text was kept. Additive; no text or fallback rule
changed. Still open here: the parameter panel shows its markers (UI owner,
same day); the catalogue, product-block and DPT markers have no web consumer
yet (UI owner), and the documentation report's communication-object text
stays language-insensitive (above).

**Update 2026-10-05 (AR10 slice 3): the documentation report follows the
device detail's rule.** `knx_app::com_object_language` now holds the one rule
both use — only `Program`/`ProgramRef`-layer text is replaced, only where a
translation answered — and `report_options` passes the result to the renderer
as `ReportDeviceData::com_object_texts`. A German report therefore shows the
package's German communication-object name and description; project-authored
text and language misses keep the project's words
(`documentation_composition.rs`: `com_object_text::*`). Two statements above
are stale and corrected here: `ComObjectNode` has carried `function_text`
since §146, so `FunctionText` is read on the device detail. **Residue,
accepted:** device creation and `enrich()` keep storing package text
(project data must not depend on a display setting); the project's own
language and `StringTable` resolution have no source (§14). **Waiting on the
UI owner:** showing the slice-2b markers (catalogue, product block, DPT text).

**Update 2026-10-05 (UI owner, KL-37): the markers are shown.** One rule and
one badge (`apps/knx-web/src/languageFallback.tsx`) serve every surface: with a
product language selected, a shown text whose answering language is `null` or
absent gets "Untranslated (<declared source language>)", or just
"Untranslated" when nothing is declared — never guessed. Catalogue entries mark
name and description (`sourceLanguage`); the device product block marks
product text, catalogue name and application name with their own
`*_source_language`; a communication object's DPT text is marked without a
source, because the master data declares none on this wire. *Package default*
marks nothing. Manufacturer name and hardware name have no marker on the wire
and get none. What remains is the accepted residue above.
<!-- END retained catalogue record -->
