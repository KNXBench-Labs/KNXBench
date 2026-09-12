# T33 — Language-aware communication-object text

## Problem

`knx-productdb`'s `translation` table stores every `TranslationElement` a
package declares, including the ones that target a `ComObject` or a
`ComObjectRef`. Nothing reads them. `crates/knx-productdb/src/query.rs`'s
`com_object_view()` takes no `language` argument at all, so a
communication object's `Text`, `FunctionText` and `VisibleDescription` are
always shown in the package's own untranslated wording — even when the user
has picked a product language in Settings and the parameter panel beside it
is already honouring that choice (T26, 2026-09-12).

Measured on `OriginalData/ProductDatabases/MDT_KP_AMI_AMS_03_Switch_Actuator_V31a.knxprod`,
application program `M-0083_A-0317-31-7DC6`: five `<Language>` blocks
(`de-DE`, `en-US`, `fr-FR`, `es-ES`, `it-IT`), each carrying **53
`ComObject/@Text` and 50 `ComObject/@FunctionText` translations** — 515
rows already ingested and read by nothing. The same program declares 13
top-level `ComObjectRef`s and translates none of them, so the
`ComObjectRef` overlay path has no coverage in this package; the
implementer must measure the remaining corpus packages rather than assume
either way.

This is the open half of `KNOWN_LIMITATIONS.md` §37 and the remaining data
side of `GAP_ANALYSIS_ETS.md` D10.

## Spec

A user who has selected a product language in Settings sees each
communication object's name and description in that language, in the device
Inspector, wherever the product package supplies a translation. Where it
does not, the package's own untranslated text is shown unchanged.

Translation is **display only**. The project file's content must not change
because a display language changed.

## Global Constraints

1. **No fallback chain between languages.** A missing `(ref_id,
   attribute_name)` row falls through to the package's own untranslated
   column, never to another language. Same rule T26 established.
2. **Only product-supplied text is ever translated.** A
   `ComObjectInstance`'s `text`/`description` is `Override<Text>` carrying
   a `Layer`. Only `Layer::Program` and `Layer::ProgramRef` values came
   from the product database and may be overlaid. `Layer::Instance`,
   `Layer::Inferred` and `Layer::UserEdit` are project-authored content —
   overlaying one would show a user their own words back in a different
   language, and `Layer::Instance`/`Layer::UserEdit` values *are* exported
   to `.knxproj`. This invariant is load-bearing and must have a test that
   fails if it is violated.
3. **`language: None` changes nothing.** The no-language path must issue no
   `translation` query at all and return byte-identical results to today.
4. **Device creation and `enrich()` keep baking untranslated text.** The
   project stays language-independent on disk; nothing in this plan passes
   a language into `enrich`/`apply`/`create_device`.
5. Keep the layering rule: `knx-projection` must not learn about
   `knx-productdb`. The overlay belongs in `apps/knx-server`, which already
   holds both the project and the product-database connection.
6. Every new or edited source file carries a version and a one-sentence
   purpose header, per the repository convention.
7. Commit messages in the repository's established gloomy style. Facts stay
   accurate. No `Co-Authored-By` trailer (CLAUDE.md).

## Task 1 — `com_object_view` learns a language

**Files:** `crates/knx-productdb/src/query.rs`,
`crates/knx-productdb/src/enrich.rs`, `apps/knx-server/src/domain.rs`
(caller update only).

Add a fourth parameter `language: Option<&str>` to
`pub fn com_object_view(conn, program_id, com_object_ref_id, language)`.

- Reuse the existing private `translation_overlay(conn, program_id,
  language)` and `overlay_text(...)` helpers verbatim. Do not add a second
  overlay loader.
- The `SELECT` must additionally return `co.id` (the `ComObject`'s own id).
  A `ComObject`-layer translation's `RefId` **is the `ComObject`'s id**, not
  the `ComObjectRef`'s — measured above. The `ComObjectRef`-layer
  translation's `RefId` is the `ComObjectRef`'s id (the function's existing
  `com_object_ref_id` argument).
- Apply the overlay **before** `pick()`, per layer, exactly as
  `parameter_views` does: a translated `ComObject/@Text` replaces
  `co.text` at the `Program` layer, a translated `ComObjectRef/@Text`
  replaces `cor.text` at the `ProgramRef` layer. `ValueLayer` keeps its one
  existing meaning.
- Overlay exactly three attributes: `Text` → `text`, `FunctionText` →
  `function_text`, `VisibleDescription` → `visible_description`. Nothing
  else. Flags, `object_size`, `priority`, `dpt_list` and `number` are
  values, not display text, and must stay untranslated — the same reason
  `Value` is unreachable in T26's allow-list.
- Update every existing caller to pass `None`: `enrich.rs` (two call
  sites, including its unit test) and `apps/knx-server/src/domain.rs:1446`.

Also export the module-ref-id reconstruction that the server will need:

```rust
/// The `ComObjectRef` id to look up for one device-level `RefId`.
pub fn com_object_lookup_id(program_id: &str, ref_id: &str, module_based: bool) -> String
```

in `enrich.rs`, wrapping the existing private `module_ref_id` with the same
`unwrap_or_else(|| ref_id.to_string())` fallback `enrich()` already uses, and
re-export it from `lib.rs` beside the rest of the crate's public surface.
`enrich()` itself must then call it rather than keeping a duplicate of that
`if`.

**Tests** (in `query.rs`'s existing test module, alongside the
`com_object_view` tests at ~line 1154):

1. `language = None` returns exactly what it returns today (the existing
   tests already assert this; add nothing that weakens them).
2. A `ComObject`-scope `Text` translation replaces the `Program`-layer text
   and reports `ValueLayer::Program`.
3. A `ComObjectRef`-scope `Text` translation replaces the `ProgramRef`-layer
   text and reports `ValueLayer::ProgramRef`.
4. A `ComObject`-scope `FunctionText` and a `VisibleDescription`
   translation each land in their own field.
5. A language with no rows for this program leaves every field untranslated
   (no fallback to another language's rows, Constraint 1).
6. A translation row whose `attribute_name` is `ObjectSize` (or any other
   non-display attribute) is **not** applied — the guard for Constraint 2's
   sibling rule on this layer.

**Acceptance:** `cargo test -p knx-productdb` passes; `cargo clippy -p
knx-productdb --all-targets -- -D warnings` clean.

## Task 2 — `GET /api/device/{id}?language=` overlays com-object text

**Files:** `apps/knx-server/src/domain.rs`,
`apps/knx-server/src/routes.rs`, new
`apps/knx-server/tests/http_com_object_language.rs`.

`device_detail(state, device_id)` and `device_detail_impl` gain
`language: Option<&str>`. The route gains `Query<...>` before the path/state
extractors as axum requires, mirroring the parameter-panel route.

With `language: None`, or with no product database open, the response is
byte-identical to today — build the projection and return it, touching no
product-database code path.

With `language: Some(lang)`:

1. Build the `DeviceDetail` as today via
   `knx_projection::build_device_detail`.
2. From the same `&Project`, read the device's `program_ref` and, for each
   `ComObjectInstanceId`, the instance's `source.ets_id`, its
   `module_instance.is_some()`, and the `Layer` of its `text` and
   `description` overrides.
3. Resolve the program with
   `knx_productdb::query::resolve_program(conn, program_ref)`. If it
   resolves to nothing, return the untranslated detail unchanged — a device
   whose program is not installed is ordinary project state, not an error.
4. For each com object, call `com_object_view(conn, &program_id,
   &com_object_lookup_id(&program_id, &ref_id, module_based), Some(lang))`.
   Overwrite `ComObjectNode::name` from `view.text` **only if** that
   instance's `text` override sits at `Layer::Program` or
   `Layer::ProgramRef`; likewise `description` from
   `view.visible_description` against the `description` override's layer.
   A `None` from the view leaves the existing value alone.

Follow `assemble_parameter_panel`'s locking idiom: take the project lock and
the `product_db` mutex the way it already does, and do not hold them across
anything that can block.

**Tests** (new integration test file, following
`apps/knx-server/tests/http_product_language.rs`'s shape — build a real
product database from a synthetic package fixture the way that file does):

1. `GET /api/device/{id}` with no `language` returns the untranslated
   name — the regression guard for Constraint 3.
2. `GET /api/device/{id}?language=de-DE` returns the translated name and
   description for a `Program`-layer com object.
3. **The load-bearing one for Constraint 2:** a com object whose `text`
   override sits at `Layer::Instance` (or `UserEdit`) keeps its
   project-authored text verbatim even when the product database has a
   translation for that same ref id. Name it so the invariant is legible,
   e.g.
   `an_instance_layer_text_is_never_translated_because_the_project_owns_it`.
4. A language the package does not carry returns the untranslated text
   (no cross-language fallback).
5. A device whose program is not in the product database returns `200`
   with the untranslated detail, not an error.

**Acceptance:** `cargo test -p knx-server` passes; clippy clean.

## Task 3 — the Inspector asks for the user's language

**Files:** `apps/knx-web/src/` — the device-detail fetch site(s), and
whichever component owns them. Find them; do not assume the path.

Thread the existing `useProductLanguage()` hook into the device-detail
request as `?language=`, exactly as `CatalogBrowser.tsx` and
`ParameterPanel.tsx` already do, **including `language` in the fetching
effect's dependency array** so that changing the setting while a device is
selected refetches instead of leaving the previous language's names on
screen. `useProductLanguage()` is already a `useSyncExternalStore`-backed
module-level store (T26 fix round 1), so no new plumbing is needed.

**Tests** (vitest, in the owning component's existing test file):

1. The device-detail fetch carries `?language=` when a language is set, and
   carries no `language` parameter when none is.
2. Changing the language under an already-mounted Inspector refetches.

**Acceptance:** `npm test` and `npx tsc --noEmit` clean in `apps/knx-web`.

## Task 4 — documentation reconciliation

**Files:** `docs/KNOWN_LIMITATIONS.md`, `docs/GAP_ANALYSIS_ETS.md`,
`docs/IMPLEMENTATION_STATUS.md`, `docs/COMPATIBILITY.md`,
`docs/ROADMAP.md` (only where a statement is now false).

- §37: the communication-object half is now read. Say precisely which
  surfaces translate and which still do not (`knx_core::string_table`'s
  `StringTable` still has no resolver outside `build_device_detail`'s
  default-language call, and the project-side `Language` field is still
  unread — do not overstate).
- D10: record T33; the row stays **open** for the chrome half (T25) and for
  the hardware/master-scope surfaces §64 still names.
- Add the `T33. Done (2026-09-12)` backlog entry in Tier 6 beside T32.
- `IMPLEMENTATION_STATUS.md`: dated entry plus the `Last updated:` line.
- Record the measured corpus numbers (53 `Text` / 50 `FunctionText` per
  language, five languages, `M-0083_A-0317-31-7DC6`) — measurement, not
  recollection. If the implementer's own measurement disagrees with this
  plan's, the implementer's measurement wins and the plan was wrong.
- Do **not** claim the project string table is translated, and do not
  downgrade §64.

**Acceptance:** no document asserts something the branch just made false.

## Gates before merge

`cargo fmt --all --check`; `cargo clippy --workspace --all-targets -- -D
warnings`; `cargo test --workspace`; `check-layering`; `cargo deny check`;
`npm test` and `npx tsc --noEmit` in `apps/knx-web`.
