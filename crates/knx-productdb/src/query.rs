//! The read side: resolving a device's program reference, and the merged
//! `ComObject` + `ComObjectRef` view the enrichment consumes.
//!
//! The merge keeps the layer that supplied each value. That is the whole
//! point of the override chain (DATA_MODEL §3): a value without its layer
//! cannot be written back correctly, so this view never returns one.

use std::collections::{HashMap, HashSet};

use rusqlite::{Connection, OptionalExtension};

use crate::ProductDbError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueLayer {
    Program,
    ProgramRef,
}

/// Picks the `ComObjectRef` value when it has one, otherwise the
/// `ComObject`'s, reporting which layer won.
fn pick(program: Option<String>, program_ref: Option<String>) -> (Option<String>, ValueLayer) {
    match program_ref {
        Some(v) => (Some(v), ValueLayer::ProgramRef),
        None => (program, ValueLayer::Program),
    }
}

/// Resolves a requested display language (e.g. `de`) against the set of
/// language identifiers a translation actually has rows for (e.g.
/// `["de-DE", "en-US"]`), per R2. This is the **one place** the rule lives
/// — every overlay in this file (`translation_overlay`, `catalog_overlay`,
/// `overlay_one`, `master_text_overlay`) resolves its candidates through
/// this function rather than repeating the comparison.
///
/// Rule, in order:
/// 1. An exact match always wins, even when a prefix match also exists
///    (a requested `de-DE` must not be redirected to `de-AT` just because
///    prefix-matching exists as a fallback).
/// 2. Otherwise, a stored identifier matches by locale prefix when it
///    equals `requested` followed by a `-` and at least one more byte —
///    `de` matches `de-DE`, but not `de` itself (already handled by the
///    exact case above) and not `deX` (no separator). One-directional, as
///    specified: a longer requested identifier is never shortened to match
///    a shorter stored one.
/// 3. Two or more stored identifiers can legally prefix-match the same
///    request (`de-DE` and `de-AT` both match `de`) and the KNX App XML
///    schema gives no rule for preferring one over the other, so the
///    tiebreak is simply the lexicographically smallest identifier
///    (`Ord` on `&str`, i.e. plain byte order) — deterministic and
///    documented, per R2's "pick one, implement it, document which and
///    why", not a claim that `de-AT` is somehow the "right" default.
/// 4. An empty or otherwise non-matching requested string resolves to
///    `None`, exactly like a `language` with zero candidates — a miss
///    here is never an error, only the call site decides what the
///    untranslated fallback is.
fn best_matching_language<'a>(requested: &str, available: &[&'a str]) -> Option<&'a str> {
    if requested.is_empty() {
        return None;
    }
    if let Some(&exact) = available.iter().find(|&&candidate| candidate == requested) {
        return Some(exact);
    }
    let prefix = format!("{requested}-");
    available
        .iter()
        .copied()
        .filter(|candidate| candidate.starts_with(&prefix))
        .min()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComObjectView {
    pub number: Option<i64>,
    pub text: Option<String>,
    pub text_layer: ValueLayer,
    /// `true` iff `text` came out of the translation overlay rather than
    /// the product database's own column — see `overlaid_pick`'s doc
    /// comment for exactly what "came out of" means once a layer is
    /// involved.
    pub text_translated: bool,
    pub function_text: Option<String>,
    pub function_text_layer: ValueLayer,
    pub function_text_translated: bool,
    pub visible_description: Option<String>,
    pub description_layer: ValueLayer,
    pub visible_description_translated: bool,
    pub object_size: Option<String>,
    pub object_size_layer: ValueLayer,
    pub priority: Option<String>,
    pub dpt_list: Option<String>,
    pub dpt_layer: ValueLayer,
    pub read: Option<String>,
    pub read_layer: ValueLayer,
    pub write: Option<String>,
    pub write_layer: ValueLayer,
    pub transmit: Option<String>,
    pub transmit_layer: ValueLayer,
    pub update: Option<String>,
    pub update_layer: ValueLayer,
    pub communication: Option<String>,
    pub communication_layer: ValueLayer,
    /// `ReadOnInitFlag` — the sixth flag. Measured in the local corpus only
    /// on `ComObject` (2533 rows, all `"Disabled"`), never on
    /// `ComObjectRef`; the `ProgramRef` column exists anyway because the
    /// package format allows the override even where no shipped package has
    /// used it yet.
    pub read_on_init: Option<String>,
    pub read_on_init_layer: ValueLayer,
}

pub fn resolve_program(
    conn: &Connection,
    hardware2program_id: &str,
) -> Result<Option<String>, ProductDbError> {
    let id: Option<String> = conn
        .query_row(
            "SELECT ap.id
             FROM hardware2program h2p
             JOIN application_program ap ON ap.id = h2p.application_program_ref
             WHERE h2p.id = ?1",
            [hardware2program_id],
            |r| r.get(0),
        )
        .optional()?;
    Ok(id)
}

struct RawRow {
    /// `com_object_ref.id` — the row's own key in `com_object_views`'
    /// returned map. `com_object_view` (the one-element wrapper) already
    /// has this as its `com_object_ref_id` argument, so it is not selected
    /// there; `com_object_views` selects it because a batch call is the
    /// only source it has for which ref each row belongs to.
    cor_id: String,
    /// `com_object.id` — not exposed on `ComObjectView`, but needed as the
    /// overlay lookup key for `co.text`/`co.function_text`/
    /// `co.visible_description`, since the translation table is keyed by
    /// `ref_id` and a `ComObject`-layer translation's `ref_id` is the
    /// `ComObject`'s own id, not its `ComObjectRef`'s (measured against
    /// shipped packages: a `ComObjectTable/ComObject`'s `TranslationElement
    /// RefId` matches `ComObject/@Id`, never the referencing
    /// `ComObjectRef/@Id`).
    co_id: String,
    number: Option<i64>,
    co_text: Option<String>,
    co_function_text: Option<String>,
    co_visible_description: Option<String>,
    co_object_size: Option<String>,
    co_priority: Option<String>,
    co_dpt_list: Option<String>,
    co_read: Option<String>,
    co_write: Option<String>,
    co_transmit: Option<String>,
    co_update: Option<String>,
    co_communication: Option<String>,
    co_read_on_init: Option<String>,
    cor_text: Option<String>,
    cor_function_text: Option<String>,
    cor_visible_description: Option<String>,
    cor_object_size: Option<String>,
    cor_priority: Option<String>,
    cor_dpt_list: Option<String>,
    cor_read: Option<String>,
    cor_write: Option<String>,
    cor_transmit: Option<String>,
    cor_update: Option<String>,
    cor_communication: Option<String>,
    cor_read_on_init: Option<String>,
}

/// `pick()`, plus whether the value it returned came out of the
/// translation overlay rather than the product database's own column.
///
/// This is deliberately **not** "did an overlay entry exist for either
/// layer" — it is "did an overlay entry exist for the layer `pick()`
/// actually chose". A `ComObject`-scope translation that loses to a raw,
/// untranslated `ComObjectRef` override must report `false`; the reverse
/// (a `ComObjectRef`-scope translation winning over a `ComObject`-scope
/// one) must report `true`. Folding the overlay lookup into `pick()`'s own
/// two inputs, rather than computing "translated" as a separate pass over
/// the finished `ComObjectView`, is what keeps that correct without a
/// second branch mirroring `pick()`'s own.
fn overlaid_pick(
    overlay: Option<&HashMap<(String, String), String>>,
    co_id: &str,
    cor_id: &str,
    attribute_name: &str,
    program_value: Option<String>,
    program_ref_value: Option<String>,
) -> (Option<String>, ValueLayer, bool) {
    let co_overlay = overlay_text(overlay, co_id, attribute_name);
    let cor_overlay = overlay_text(overlay, cor_id, attribute_name);
    let co_translated = co_overlay.is_some();
    let cor_translated = cor_overlay.is_some();
    let (value, layer) = pick(
        co_overlay.or(program_value),
        cor_overlay.or(program_ref_value),
    );
    let translated = match layer {
        ValueLayer::Program => co_translated,
        ValueLayer::ProgramRef => cor_translated,
    };
    (value, layer, translated)
}

/// Every `ComObjectView` a program's `com_object_ref_ids` resolve to, keyed
/// by `com_object_ref.id`. One query for the whole slice (chunked, see
/// below) and the overlay loaded exactly once for the whole call — the
/// server used to call `com_object_view` in a loop, once per communication
/// object, reloading the overlay on every single iteration; a device can
/// own hundreds of communication objects, so that per-row shape is exactly
/// as wrong as `parameter_views`'s own doc comment already argues for
/// parameters.
///
/// Ref ids absent from the database are simply absent from the returned
/// map; this function never errors on a partial match, the caller decides
/// what a missing id means. An empty `com_object_ref_ids` returns an empty
/// map without touching the database at all — not one query with an empty
/// `IN ()`, which SQLite accepts but which would still load the overlay
/// for nothing (Global Constraint 1: `language: None` is the common case
/// this guards, but a caller that passes zero ids must not pay for a query
/// either, regardless of `language`).
///
/// `language`, the overlay and `ValueLayer` follow exactly what
/// `com_object_view`'s own doc comment already describes, applied once per
/// row instead of to one row.
///
/// **Chunking.** A program can declare more `ComObjectRef`s than a single
/// statement may bind (the corpus has a program with 543 `ParameterRef`s,
/// so the order of magnitude is real). `SQLITE_MAX_VARIABLE_NUMBER` is
/// 32766 in the bundled SQLite this crate links (`rusqlite` with
/// `features = ["bundled"]`, `libsqlite3-sys` 0.38.2, SQLite 3.53.2); it
/// was 999 before SQLite 3.32, and a system SQLite may still be built
/// that way.
/// `com_object_ref_ids` is chunked at 900 ids per statement — comfortably
/// under the limit alongside the `program_id` parameter — one prepared
/// statement per chunk, never one per id. The overlay is loaded once,
/// outside the chunk loop, for the whole call regardless of how many
/// chunks the slice needed.
pub fn com_object_views(
    conn: &Connection,
    program_id: &str,
    com_object_ref_ids: &[&str],
    language: Option<&str>,
) -> Result<HashMap<String, ComObjectView>, ProductDbError> {
    if com_object_ref_ids.is_empty() {
        return Ok(HashMap::new());
    }

    const CHUNK_SIZE: usize = 900;

    let mut raw_rows: Vec<RawRow> = Vec::with_capacity(com_object_ref_ids.len());
    for chunk in com_object_ref_ids.chunks(CHUNK_SIZE) {
        let placeholders = std::iter::repeat_n("?", chunk.len())
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "SELECT cor.id, co.id, co.number,
                    co.text, co.function_text, co.visible_description, co.object_size,
                    co.priority, co.dpt_list, co.read_flag, co.write_flag,
                    co.transmit_flag, co.update_flag, co.communication_flag,
                    co.read_on_init_flag,
                    cor.text, cor.function_text, cor.visible_description, cor.object_size,
                    cor.priority, cor.dpt_list, cor.read_flag, cor.write_flag,
                    cor.transmit_flag, cor.update_flag, cor.communication_flag,
                    cor.read_on_init_flag
             FROM com_object_ref cor
             JOIN com_object co
               ON co.program_id = cor.program_id AND co.id = cor.com_object_id
             WHERE cor.program_id = ? AND cor.id IN ({placeholders})"
        );
        let mut stmt = conn.prepare(&sql)?;
        let params = std::iter::once(&program_id as &dyn rusqlite::ToSql)
            .chain(chunk.iter().map(|id| id as &dyn rusqlite::ToSql));
        let chunk_rows = stmt
            .query_map(rusqlite::params_from_iter(params), |r| {
                Ok(RawRow {
                    cor_id: r.get(0)?,
                    co_id: r.get(1)?,
                    number: r.get(2)?,
                    co_text: r.get(3)?,
                    co_function_text: r.get(4)?,
                    co_visible_description: r.get(5)?,
                    co_object_size: r.get(6)?,
                    co_priority: r.get(7)?,
                    co_dpt_list: r.get(8)?,
                    co_read: r.get(9)?,
                    co_write: r.get(10)?,
                    co_transmit: r.get(11)?,
                    co_update: r.get(12)?,
                    co_communication: r.get(13)?,
                    co_read_on_init: r.get(14)?,
                    cor_text: r.get(15)?,
                    cor_function_text: r.get(16)?,
                    cor_visible_description: r.get(17)?,
                    cor_object_size: r.get(18)?,
                    cor_priority: r.get(19)?,
                    cor_dpt_list: r.get(20)?,
                    cor_read: r.get(21)?,
                    cor_write: r.get(22)?,
                    cor_transmit: r.get(23)?,
                    cor_update: r.get(24)?,
                    cor_communication: r.get(25)?,
                    cor_read_on_init: r.get(26)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        raw_rows.extend(chunk_rows);
    }

    // Loaded once for the whole call, across every chunk above, and only
    // when a language was actually requested — see this function's own
    // doc comment and `translation_overlay`'s.
    let overlay = language
        .map(|lang| translation_overlay(conn, program_id, lang))
        .transpose()?;

    let mut views = HashMap::with_capacity(raw_rows.len());
    for raw in raw_rows {
        // Overlaid **before** `pick()`, per layer, so a translated string
        // lands at the same structural layer its untranslated counterpart
        // would have, and `*_translated` reflects the layer that actually
        // won (see `overlaid_pick`'s own doc comment).
        let (text, text_layer, text_translated) = overlaid_pick(
            overlay.as_ref(),
            &raw.co_id,
            &raw.cor_id,
            "Text",
            raw.co_text,
            raw.cor_text,
        );
        let (function_text, function_text_layer, function_text_translated) = overlaid_pick(
            overlay.as_ref(),
            &raw.co_id,
            &raw.cor_id,
            "FunctionText",
            raw.co_function_text,
            raw.cor_function_text,
        );
        let (visible_description, description_layer, visible_description_translated) =
            overlaid_pick(
                overlay.as_ref(),
                &raw.co_id,
                &raw.cor_id,
                "VisibleDescription",
                raw.co_visible_description,
                raw.cor_visible_description,
            );
        let (object_size, object_size_layer) = pick(raw.co_object_size, raw.cor_object_size);
        let (dpt_list, dpt_layer) = pick(raw.co_dpt_list, raw.cor_dpt_list);
        let (read, read_layer) = pick(raw.co_read, raw.cor_read);
        let (write, write_layer) = pick(raw.co_write, raw.cor_write);
        let (transmit, transmit_layer) = pick(raw.co_transmit, raw.cor_transmit);
        let (update, update_layer) = pick(raw.co_update, raw.cor_update);
        let (communication, communication_layer) =
            pick(raw.co_communication, raw.cor_communication);
        let (read_on_init, read_on_init_layer) = pick(raw.co_read_on_init, raw.cor_read_on_init);
        let priority = raw.cor_priority.or(raw.co_priority);

        views.insert(
            raw.cor_id,
            ComObjectView {
                number: raw.number,
                text,
                text_layer,
                text_translated,
                function_text,
                function_text_layer,
                function_text_translated,
                visible_description,
                description_layer,
                visible_description_translated,
                object_size,
                object_size_layer,
                priority,
                dpt_list,
                dpt_layer,
                read,
                read_layer,
                write,
                write_layer,
                transmit,
                transmit_layer,
                update,
                update_layer,
                communication,
                communication_layer,
                read_on_init,
                read_on_init_layer,
            },
        );
    }
    Ok(views)
}

/// Joins one `ComObjectRef` row to the `ComObject` it refers to, within the
/// same program, and folds every attribute through `pick`. A one-element
/// call into `com_object_views` — resolving more than one ref through a
/// loop of these is the shape that function's own doc comment exists to
/// replace.
///
/// `language` is the requested display language, `None` meaning the
/// package's own untranslated text — today's behaviour, unchanged, issuing
/// no `translation` query at all (Global Constraint 3). `Some` loads one
/// overlay (`translation_overlay`, shared verbatim with `parameter_views`)
/// and applies it **before** `pick()`, exactly as `parameter_views` does:
/// a translated `ComObject/@Text` lands at the `Program` layer, a
/// translated `ComObjectRef/@Text` at the `ProgramRef` layer, so
/// `ValueLayer` keeps meaning only "which structural layer supplied the
/// value", never also "which language did". Only `text`, `function_text`
/// and `visible_description` are ever overlaid — `object_size`, `priority`,
/// `dpt_list`, `number` and the six flags are values, not display text,
/// and translating them would corrupt stored project data the moment
/// someone switched languages, exactly as `Value` stays untranslated in
/// `translation_overlay`'s own doc comment.
pub fn com_object_view(
    conn: &Connection,
    program_id: &str,
    com_object_ref_id: &str,
    language: Option<&str>,
) -> Result<Option<ComObjectView>, ProductDbError> {
    Ok(
        com_object_views(conn, program_id, &[com_object_ref_id], language)?
            .remove(com_object_ref_id),
    )
}

/// A single `Parameter`, resolved through its own program's `ParameterRef`
/// and `ParameterType` (design D22). Follows `ComObjectView`'s `pick()`/
/// `ValueLayer` idiom for the one field a `ParameterRef` can override —
/// there is no three-layer override chain here, `ComObjectInstanceRef`'s
/// wider one does not apply to parameters (ARCHITECTURE.md §5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParameterView {
    /// `parameter_ref.id`, the `ValueMap`/`ets_id` key (D21).
    pub id: String,
    /// `parameter_ref.display_order`, verbatim, `None` when the package
    /// declares no order at all. `ParameterRef/@DisplayOrder` is genuinely
    /// optional in shipped packages — measured, not assumed: every one of
    /// the 543 `parameter_ref` rows for `prod3`'s program
    /// `M-0083_A-0317-31-7DC6` omits it. `None` is therefore its own real
    /// value, distinct from `Some(0)`, not a magic-constant stand-in for it.
    pub display_order: Option<i64>,
    pub tag: Option<String>,
    /// `parameter.name`.
    pub name: Option<String>,
    /// `pick(parameter.text, parameter_ref.text)`.
    pub text: Option<String>,
    pub text_layer: ValueLayer,
    /// `parameter_type.kind`, verbatim: one of `Restriction`, `Number`,
    /// `Text`, `None`, `Float`, `IPAddress`, `Picture`, `Raw`, `Color`,
    /// `Time`, `Other`.
    pub kind: String,
    /// `parameter.access`, verbatim — display only, D24 does not gate on it.
    pub access: Option<String>,
    pub min_inclusive: Option<String>,
    pub max_inclusive: Option<String>,
    /// `parameter_type.size_in_bit`, verbatim. Populated by `Number` and
    /// `Restriction` (both pre-T18 slice 5) and, since T18 slice 5, by
    /// `Text` (`<TypeText SizeInBit="…"/>`'s own maximum content length in
    /// bits — corpus-observed: `<TypeText SizeInBit="240"/>` and
    /// `SizeInBit="640"` in the MDT `M-0083_A-0317-31-7DC6` program). `None`
    /// for every other kind, including `Text` packages old enough (or
    /// exotic enough) to omit the attribute.
    pub size_in_bit: Option<i64>,
    /// `(value, text)`, only non-empty when `kind == "Restriction"` — the
    /// other seven kinds never have rows in `parameter_type_enum`.
    pub enum_options: Vec<(String, Option<String>)>,
}

struct ParameterRawRow {
    id: String,
    display_order: Option<i64>,
    tag: Option<String>,
    /// `parameter.id` — not exposed on `ParameterView` (which carries the
    /// `ParameterRef` id, `id` above), but needed as the overlay lookup key
    /// for `p.text`/`p.name`, since the translation table is keyed by
    /// `ref_id` and a `Parameter`-layer translation's `ref_id` is the
    /// `Parameter`'s own id, not its `ParameterRef`'s.
    parameter_id: String,
    name: Option<String>,
    p_text: Option<String>,
    pr_text: Option<String>,
    kind: String,
    access: Option<String>,
    min_inclusive: Option<String>,
    max_inclusive: Option<String>,
    size_in_bit: Option<i64>,
    parameter_type_id: String,
}

/// One overlay lookup, or `None` when there is no overlay for this call at
/// all (the `language: None` path) or the overlay has no row for this exact
/// `(ref_id, attribute)`. There is deliberately no second lookup on a miss —
/// Global Constraint 4 (no fallback chain between languages): a miss falls
/// straight through to the package's own untranslated column, never to
/// another language.
fn overlay_text(
    overlay: Option<&HashMap<(String, String), String>>,
    ref_id: &str,
    attribute_name: &str,
) -> Option<String> {
    overlay?
        .get(&(ref_id.to_string(), attribute_name.to_string()))
        .cloned()
}

/// Every `ParameterView` a program declares, in `parameter_ref.
/// display_order`. One query, not one per field — a single `ModuleDef` can
/// own on the order of hundreds of these (RESEARCH.md §4.4 Q3), so N calls
/// is the wrong shape, exactly as `com_object_view`'s own doc comment
/// already reasons for communication objects.
///
/// `language` is the requested display language, `None` meaning the
/// package's own untranslated text — today's behaviour, unchanged, and
/// issuing no `translation` query at all. `Some` loads one overlay
/// (`translation_overlay`) for the whole call and applies it **before**
/// `pick()`, per element: a translated `Parameter/@Text` lands at the
/// `Program` layer and a translated `ParameterRef/@Text` at the
/// `ProgramRef` layer, exactly where their untranslated counterparts would.
/// `ValueLayer` therefore keeps its one existing meaning — which structural
/// layer supplied the value — instead of also having to mean "which
/// language did" (design D22's translation slice).
pub fn parameter_views(
    conn: &Connection,
    program_id: &str,
    language: Option<&str>,
) -> Result<Vec<ParameterView>, ProductDbError> {
    // `pr.display_order` is selected raw, not `COALESCE`d: real-world
    // packages exist where `ParameterRef/@DisplayOrder` is simply absent —
    // observed on the full corpus, not a hypothetical, every one of one MDT
    // program's 543 `ParameterRef`s omits it — and `None` is its own real
    // value there, distinct from a package that genuinely declares position
    // zero. `rusqlite` maps a NULL INTEGER straight to `None` for an
    // `Option<i64>` target, so no `COALESCE` is needed for that mapping.
    //
    // `ORDER BY pr.display_order, pr.rowid`: with `DisplayOrder` absent on
    // every row (the common real-world case above), every row ties on the
    // first key and the result would otherwise rest on SQLite's sorter,
    // whose tie stability is not documented. `parameter_ref` is a plain
    // rowid table (`PRIMARY KEY (program_id, id)`, not `WITHOUT ROWID`), and
    // the parser inserts `ParameterRef` rows in document order as it streams
    // the XML, so `rowid` recovers the package's own declaration order for
    // ties. This cannot change any order where `DisplayOrder` *is* declared
    // — it only breaks ties among rows sharing one value (including
    // NULL). SQLite's default NULLs-first-ascending placement is kept
    // as-is: how ETS orders a set that mixes declared and undeclared
    // `DisplayOrder` is unattested anywhere in the corpus, and inverting it
    // or adding `NULLS LAST` here would be an assumption dressed up as
    // behaviour.
    let mut stmt = conn.prepare(
        "SELECT pr.id, pr.display_order, pr.tag,
                p.id, p.name, p.text, pr.text,
                pt.kind, p.access, pt.min_inclusive, pt.max_inclusive, pt.size_in_bit, pt.id
         FROM parameter_ref pr
         JOIN parameter p ON p.program_id = pr.program_id AND p.id = pr.parameter_id
         JOIN parameter_type pt ON pt.program_id = p.program_id AND pt.id = p.parameter_type_id
         WHERE pr.program_id = ?1
         ORDER BY pr.display_order, pr.rowid",
    )?;
    let raw_rows: Vec<ParameterRawRow> = stmt
        .query_map([program_id], |r| {
            Ok(ParameterRawRow {
                id: r.get(0)?,
                display_order: r.get(1)?,
                tag: r.get(2)?,
                parameter_id: r.get(3)?,
                name: r.get(4)?,
                p_text: r.get(5)?,
                pr_text: r.get(6)?,
                kind: r.get(7)?,
                access: r.get(8)?,
                min_inclusive: r.get(9)?,
                max_inclusive: r.get(10)?,
                size_in_bit: r.get(11)?,
                parameter_type_id: r.get(12)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    // Loaded once for the whole call, never once per row (see this
    // function's own doc comment above and `translation_overlay`'s).
    // `None` issues no `translation` query at all: a caller that never asks
    // for a language must not pay for one.
    let overlay = language
        .map(|lang| translation_overlay(conn, program_id, lang))
        .transpose()?;

    let mut views = Vec::with_capacity(raw_rows.len());
    for raw in raw_rows {
        // Overlaid **before** `pick()`, per element, so a translated string
        // lands at the same structural layer its untranslated counterpart
        // would have (see the doc comment above `pick()` is unchanged).
        let p_text = overlay_text(overlay.as_ref(), &raw.parameter_id, "Text").or(raw.p_text);
        let pr_text = overlay_text(overlay.as_ref(), &raw.id, "Text").or(raw.pr_text);
        let (text, text_layer) = pick(p_text, pr_text);
        let name = overlay_text(overlay.as_ref(), &raw.parameter_id, "Name").or(raw.name);
        // Only `Restriction` kinds ever have rows in `parameter_type_enum`
        // (the other nine kinds have no enumeration concept at all) — the
        // kind check keeps this a second query for the fraction of rows
        // that need it, not a blind per-row lookup.
        let enum_options = if raw.kind == "Restriction" {
            parameter_type_enum_options(conn, program_id, &raw.parameter_type_id, overlay.as_ref())?
        } else {
            Vec::new()
        };
        views.push(ParameterView {
            id: raw.id,
            display_order: raw.display_order,
            tag: raw.tag,
            name,
            text,
            text_layer,
            kind: raw.kind,
            access: raw.access,
            min_inclusive: raw.min_inclusive,
            max_inclusive: raw.max_inclusive,
            size_in_bit: raw.size_in_bit,
            enum_options,
        });
    }
    Ok(views)
}

/// Single-row, single-attribute translation lookup, locale-prefix-matched
/// via `best_matching_language` the same as every batch overlay in this
/// file. For a caller with only a handful of independent scoped lookups
/// (`device_product`'s three, `datapoint_type`'s one) rather than many rows
/// sharing one `scope_id` — the shape `translation_overlay`/
/// `catalog_overlay` batch for — a few small point queries are simpler than
/// hand-written SQL that fakes prefix matching inside a `JOIN ... ON`
/// clause, which cannot express it: the winning language is resolved per
/// candidate group, not by a literal equality SQLite's planner can use.
/// Returns `None` on no match of any kind (no rows at all, or no stored
/// language resolves against `language`) — never an error, and the call
/// site decides what "no match" falls back to, same convention as
/// `overlay_text`.
fn overlay_one(
    conn: &Connection,
    scope: &str,
    scope_id: &str,
    ref_id: &str,
    attribute_name: &str,
    language: &str,
) -> Result<Option<String>, ProductDbError> {
    let mut stmt = conn.prepare(
        "SELECT language, text FROM translation
         WHERE scope = ?1 AND scope_id = ?2 AND ref_id = ?3 AND attribute_name = ?4
           AND text IS NOT NULL",
    )?;
    let candidates: Vec<(String, String)> = stmt
        .query_map(
            rusqlite::params![scope, scope_id, ref_id, attribute_name],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?
        .collect::<Result<Vec<_>, _>>()?;
    let languages: Vec<&str> = candidates.iter().map(|(l, _)| l.as_str()).collect();
    Ok(
        best_matching_language(language, &languages).and_then(|matched| {
            candidates
                .iter()
                .find(|(l, _)| l == matched)
                .map(|(_, text)| text.clone())
        }),
    )
}

/// `(ref_id, attribute_name) -> text` for one program and language, loaded
/// once per `parameter_views` call: a single `ModuleDef` can own hundreds of
/// parameters, so a per-row lookup would be the wrong shape — the same
/// reasoning `parameter_views`'s own doc comment already gives for its own
/// query. Restricted to the five attributes a display label may legally be
/// overlaid from; `Value` is excluded even though it appears in the same
/// table, because a parameter's value is a key written into the project
/// file (`ParameterFieldDto.value`), and translating it by display language
/// would corrupt stored project data the moment someone switched languages
/// (Global Constraint 3).
fn translation_overlay(
    conn: &Connection,
    program_id: &str,
    language: &str,
) -> Result<HashMap<(String, String), String>, ProductDbError> {
    // Every stored language is loaded, not just `language` itself (R2):
    // resolving `de` against `de-DE` requires knowing `de-DE` exists for
    // this `(ref_id, attribute_name)` pair in the first place, and that
    // set can differ per pair (one element may only have been translated
    // into `de-AT`, another only `de-DE`), so the resolution in
    // `best_matching_language` happens per group below, not once for the
    // whole call.
    let mut stmt = conn.prepare(
        "SELECT ref_id, attribute_name, language, text FROM translation
         WHERE scope = 'Program' AND scope_id = ?1
           AND attribute_name IN ('Text','FunctionText','SuffixText','VisibleDescription','Name')
           AND text IS NOT NULL",
    )?;
    let mut grouped: HashMap<(String, String), Vec<(String, String)>> = HashMap::new();
    let mut rows = stmt.query([program_id])?;
    while let Some(row) = rows.next()? {
        let ref_id: String = row.get(0)?;
        let attribute_name: String = row.get(1)?;
        let stored_language: String = row.get(2)?;
        let text: String = row.get(3)?;
        grouped
            .entry((ref_id, attribute_name))
            .or_default()
            .push((stored_language, text));
    }
    let mut resolved = HashMap::with_capacity(grouped.len());
    for (key, candidates) in grouped {
        let languages: Vec<&str> = candidates.iter().map(|(l, _)| l.as_str()).collect();
        if let Some(matched) = best_matching_language(language, &languages) {
            if let Some((_, text)) = candidates.iter().find(|(l, _)| l == matched) {
                resolved.insert(key, text.clone());
            }
        }
    }
    Ok(resolved)
}

/// `Channel`/`ChannelIndependentBlock` `@Id` -> translated `@Text` for one
/// program and language (ISSUE-08). Placeholders are left as stored; the
/// caller substitutes them per module expansion. An element without a
/// translation into `language` is simply absent, and the caller keeps its
/// stored text. One range scan over the program's `Text` translations,
/// filtered to channel elements here, never one query per channel.
pub fn channel_texts(
    conn: &Connection,
    program_id: &str,
    language: &str,
) -> Result<HashMap<String, String>, ProductDbError> {
    let mut stmt = conn.prepare(
        "SELECT DISTINCT element_id FROM dynamic_node
         WHERE program_id = ?1 AND kind IN ('Channel','ChannelIndependentBlock')
           AND element_id IS NOT NULL",
    )?;
    let channel_ids: HashSet<String> = stmt
        .query_map([program_id], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    if channel_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let mut stmt = conn.prepare(
        "SELECT ref_id, language, text FROM translation
         WHERE scope = 'Program' AND scope_id = ?1 AND attribute_name = 'Text'
           AND text IS NOT NULL",
    )?;
    let mut grouped: HashMap<String, Vec<(String, String)>> = HashMap::new();
    let mut rows = stmt.query([program_id])?;
    while let Some(row) = rows.next()? {
        let ref_id: String = row.get(0)?;
        if !channel_ids.contains(&ref_id) {
            continue;
        }
        grouped
            .entry(ref_id)
            .or_default()
            .push((row.get(1)?, row.get(2)?));
    }
    let mut resolved = HashMap::with_capacity(grouped.len());
    for (ref_id, candidates) in grouped {
        let languages: Vec<&str> = candidates.iter().map(|(l, _)| l.as_str()).collect();
        if let Some(matched) = best_matching_language(language, &languages) {
            if let Some((_, text)) = candidates.iter().find(|(l, _)| l == matched) {
                resolved.insert(ref_id, text.clone());
            }
        }
    }
    Ok(resolved)
}

/// One language identifier's row count, as returned by
/// `translation_languages`/`program_translation_languages`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranslationLanguage {
    pub language: String,
    pub rows: i64,
}

/// Every language identifier any program in this database declares, with
/// how many translation rows it has, most rows first then identifier
/// ascending. Database-wide: a settings screen offering "every language
/// this installation has ever seen" wants this; a single device's parameter
/// panel wants `program_translation_languages` instead — see that
/// function's doc comment for why the two must not be conflated.
pub fn translation_languages(
    conn: &Connection,
) -> Result<Vec<TranslationLanguage>, ProductDbError> {
    let mut stmt = conn.prepare(
        "SELECT language, COUNT(*) FROM translation
         GROUP BY language
         ORDER BY COUNT(*) DESC, language ASC",
    )?;
    let rows = stmt
        .query_map([], |r| {
            Ok(TranslationLanguage {
                language: r.get(0)?,
                rows: r.get(1)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// The same count as `translation_languages`, narrowed to one application
/// program. Languages are declared per `ApplicationProgram`'s own
/// `Languages` block, not per database — measured on the real corpus, the
/// programs in one database declare 10, 5, 5, 5, 4 and 2 languages
/// respectively, so a database-wide answer would be wrong for any single
/// device's parameter panel, which only ever renders one program's rows.
pub fn program_translation_languages(
    conn: &Connection,
    program_id: &str,
) -> Result<Vec<TranslationLanguage>, ProductDbError> {
    let mut stmt = conn.prepare(
        "SELECT language, COUNT(*) FROM translation
         WHERE scope = 'Program' AND scope_id = ?1
         GROUP BY language
         ORDER BY COUNT(*) DESC, language ASC",
    )?;
    let rows = stmt
        .query_map([program_id], |r| {
            Ok(TranslationLanguage {
                language: r.get(0)?,
                rows: r.get(1)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

fn parameter_type_enum_options(
    conn: &Connection,
    program_id: &str,
    parameter_type_id: &str,
    overlay: Option<&HashMap<(String, String), String>>,
) -> Result<Vec<(String, Option<String>)>, ProductDbError> {
    let mut stmt = conn.prepare(
        "SELECT id, value, text FROM parameter_type_enum
         WHERE program_id = ?1 AND parameter_type_id = ?2
         ORDER BY display_order",
    )?;
    let rows = stmt
        .query_map([program_id, parameter_type_id], |r| {
            let id: String = r.get(0)?;
            let value: String = r.get(1)?;
            let text: Option<String> = r.get(2)?;
            Ok((id, value, text))
        })?
        .collect::<Result<Vec<(String, String, Option<String>)>, _>>()?;
    // `value` is never overlaid, even when a `Value` translation row exists
    // for this same `id` (Global Constraint 3) — only `text` is a display
    // string, and only `text` is looked up, by this row's own `id`.
    Ok(rows
        .into_iter()
        .map(|(id, value, text)| {
            let text = overlay_text(overlay, &id, "Text").or(text);
            (value, text)
        })
        .collect())
}

/// The bare `parameter_ref.id` set for a program — D21's stale-value diff
/// (a stored value whose id is no longer a declared `ParameterRef`) needs
/// only this, not the full `ParameterView`, so it is kept as its own thin
/// query rather than mapped off `parameter_views`'s output.
pub fn parameter_ref_ids(
    conn: &Connection,
    program_id: &str,
) -> Result<HashSet<String>, ProductDbError> {
    let mut stmt = conn.prepare("SELECT id FROM parameter_ref WHERE program_id = ?1")?;
    let rows = stmt
        .query_map([program_id], |r| r.get(0))?
        .collect::<Result<HashSet<_>, _>>()?;
    Ok(rows)
}

/// `(id, name)` for every manufacturer, ordered by id — the listing
/// `knx products list` prints.
pub fn manufacturers(conn: &Connection) -> Result<Vec<(String, Option<String>)>, ProductDbError> {
    let mut stmt = conn.prepare("SELECT id, name FROM manufacturer ORDER BY id")?;
    let rows = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramRow {
    pub id: String,
    pub manufacturer_id: String,
    pub name: Option<String>,
    pub application_number: Option<String>,
    pub application_version: Option<String>,
    pub mask_version: Option<String>,
    /// Source lexemes, not validated runtime or commissioning capabilities.
    pub is_secure_enabled: Option<String>,
    pub max_security_group_key_table_entries: Option<String>,
    pub max_security_individual_address_entries: Option<String>,
    pub max_security_p2p_key_table_entries: Option<String>,
    pub max_tunneling_user_entries: Option<String>,
    pub max_user_entries: Option<String>,
    pub min_ets_version: Option<String>,
    pub replaces_versions: Option<String>,
}

/// Every application program, optionally narrowed to one manufacturer,
/// ordered by id — the listing `knx products list` prints.
pub fn programs(
    conn: &Connection,
    manufacturer: Option<&str>,
) -> Result<Vec<ProgramRow>, ProductDbError> {
    let sql =
        "SELECT id, manufacturer_id, name, application_number, application_version, mask_version,
                is_secure_enabled, max_security_group_key_table_entries,
                max_security_individual_address_entries, max_security_p2p_key_table_entries,
                max_tunneling_user_entries, max_user_entries, min_ets_version, replaces_versions
               FROM application_program
               WHERE ?1 IS NULL OR manufacturer_id = ?1
               ORDER BY id";
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt
        .query_map([manufacturer], |r| {
            Ok(ProgramRow {
                id: r.get(0)?,
                manufacturer_id: r.get(1)?,
                name: r.get(2)?,
                application_number: r.get(3)?,
                application_version: r.get(4)?,
                mask_version: r.get(5)?,
                is_secure_enabled: r.get(6)?,
                max_security_group_key_table_entries: r.get(7)?,
                max_security_individual_address_entries: r.get(8)?,
                max_security_p2p_key_table_entries: r.get(9)?,
                max_tunneling_user_entries: r.get(10)?,
                max_user_entries: r.get(11)?,
                min_ets_version: r.get(12)?,
                replaces_versions: r.get(13)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogItemRow {
    pub id: String,
    pub manufacturer_id: String,
    pub name: Option<String>,
    pub number: Option<String>,
    pub visible_description: Option<String>,
    pub product_ref_id: Option<String>,
    pub hardware2program_ref_id: Option<String>,
}

fn row_to_catalog_item(r: &rusqlite::Row) -> rusqlite::Result<CatalogItemRow> {
    Ok(CatalogItemRow {
        id: r.get(0)?,
        manufacturer_id: r.get(1)?,
        name: r.get(2)?,
        number: r.get(3)?,
        visible_description: r.get(4)?,
        product_ref_id: r.get(5)?,
        hardware2program_ref_id: r.get(6)?,
    })
}

const CATALOG_ITEM_COLUMNS: &str = "id, manufacturer_id, name, number, visible_description, product_ref_id, hardware2program_ref_id";

/// Every `catalog_item` row, optionally narrowed to one manufacturer and/or
/// a case-insensitive substring match on `name`/`number` — backs the catalog
/// browser (T2/T32). Device creation (`apps/knx-server`) goes straight to
/// `catalog_item` by id instead: nothing yet picks an id through this
/// listing.
///
/// `language` is `None` for today's untranslated behaviour, in which case
/// this issues the exact same statement it always has — no `translation`
/// query at all, nothing that could make SQLite pick a different plan or a
/// different tie-break for two rows sorting equal. `Some(lang)` batch-loads
/// `Name`/`VisibleDescription` rows scoped to `scope = 'Catalog' AND
/// scope_id = catalog_item.manufacturer_id` (Catalog-scope rows are keyed
/// by the *manufacturer's* RefId, not the item's own id — T32 Task 1/2)
/// and `ref_id = catalog_item.id` via `catalog_overlay`, then resolves and
/// applies the overlay in Rust before filtering/sorting — not a `JOIN ...
/// ON language = ?`, because R2's locale-prefix matching picks a different
/// winning stored language per `(manufacturer_id, item_id)` pair, which a
/// literal `ON` equality cannot express. Both the search filter and the
/// `ORDER BY` follow the overlaid name, so a translated-only match is
/// findable and the list still sorts the way it displays; `number` is
/// never translated and keeps matching/sorting on its own untranslated
/// value exactly as before.
pub fn catalog_items(
    conn: &Connection,
    manufacturer: Option<&str>,
    search: Option<&str>,
    language: Option<&str>,
) -> Result<Vec<CatalogItemRow>, ProductDbError> {
    let Some(lang) = language else {
        let sql = format!(
            "SELECT {CATALOG_ITEM_COLUMNS}
             FROM catalog_item
             WHERE (?1 IS NULL OR manufacturer_id = ?1)
               AND (?2 IS NULL
                    OR LOWER(name) LIKE '%' || LOWER(?2) || '%'
                    OR LOWER(number) LIKE '%' || LOWER(?2) || '%')
             ORDER BY manufacturer_id, name"
        );
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt
            .query_map([manufacturer, search], row_to_catalog_item)?
            .collect::<Result<Vec<_>, _>>()?;
        return Ok(rows);
    };

    // Every catalog item matching `manufacturer` is loaded untranslated
    // first — no search filter yet, since the search must run against the
    // *overlaid* name below, not the stored one. A plain `JOIN ... ON
    // language = ?` (this function's previous shape) cannot express R2's
    // prefix matching: which stored language wins can differ per
    // `catalog_item`, each keyed by its own manufacturer's `scope_id`, so
    // the resolution has to happen in Rust after batch-loading every
    // candidate, exactly as `translation_overlay` does for one program —
    // except a `catalog_items` call can span more than one manufacturer at
    // once (catalog browsing is rarely narrowed to one), so the overlay
    // below groups by `(manufacturer_id, item_id)`, not just `item_id`.
    let sql = format!(
        "SELECT {CATALOG_ITEM_COLUMNS}
         FROM catalog_item
         WHERE (?1 IS NULL OR manufacturer_id = ?1)"
    );
    let mut stmt = conn.prepare(&sql)?;
    let mut rows: Vec<CatalogItemRow> = stmt
        .query_map([manufacturer], row_to_catalog_item)?
        .collect::<Result<Vec<_>, _>>()?;

    let manufacturer_ids: Vec<String> = {
        let mut ids: Vec<String> = rows.iter().map(|r| r.manufacturer_id.clone()).collect();
        ids.sort();
        ids.dedup();
        ids
    };
    let overlay = catalog_overlay(conn, &manufacturer_ids, lang)?;
    for row in &mut rows {
        if let Some(name) = overlay.get(&(row.manufacturer_id.clone(), row.id.clone(), "Name")) {
            row.name = Some(name.clone());
        }
        if let Some(desc) = overlay.get(&(
            row.manufacturer_id.clone(),
            row.id.clone(),
            "VisibleDescription",
        )) {
            row.visible_description = Some(desc.clone());
        }
    }

    if let Some(needle) = search {
        let needle = needle.to_lowercase();
        rows.retain(|r| {
            r.name
                .as_deref()
                .is_some_and(|n| n.to_lowercase().contains(&needle))
                || r.number
                    .as_deref()
                    .is_some_and(|n| n.to_lowercase().contains(&needle))
        });
    }
    // Mirrors the untranslated branch's `ORDER BY manufacturer_id, name`,
    // on the overlaid name — SQL's NULLs-first ascending order is matched
    // by `Option`'s own `Ord` (`None < Some(_)`).
    rows.sort_by(|a, b| {
        (a.manufacturer_id.as_str(), a.name.as_deref())
            .cmp(&(b.manufacturer_id.as_str(), b.name.as_deref()))
    });
    Ok(rows)
}

/// Batch-loads every stored language variant of a `Catalog`-scope `Name`/
/// `VisibleDescription` row across `manufacturer_ids`, then resolves each
/// `(manufacturer_id, item_id, attribute_name)` group down to one text via
/// `best_matching_language` — the `catalog_items` analogue of
/// `translation_overlay`, widened to more than one `scope_id` per call
/// because a catalog listing is not scoped to one manufacturer the way a
/// program's parameter/com-object views are scoped to one program.
/// `(scope_id, ref_id, attribute)` key for `catalog_overlay`'s grouping and
/// result maps — named so clippy's `type_complexity` lint stops flagging
/// the nested tuple type at every one of its three use sites.
type CatalogOverlayKey = (String, String, &'static str);

fn catalog_overlay(
    conn: &Connection,
    manufacturer_ids: &[String],
    language: &str,
) -> Result<HashMap<CatalogOverlayKey, String>, ProductDbError> {
    if manufacturer_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let placeholders = std::iter::repeat_n("?", manufacturer_ids.len())
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "SELECT scope_id, ref_id, attribute_name, language, text FROM translation
         WHERE scope = 'Catalog' AND scope_id IN ({placeholders})
           AND attribute_name IN ('Name','VisibleDescription') AND text IS NOT NULL"
    );
    let mut stmt = conn.prepare(&sql)?;
    let params = rusqlite::params_from_iter(manufacturer_ids.iter());
    let mut grouped: HashMap<CatalogOverlayKey, Vec<(String, String)>> = HashMap::new();
    let mut rows = stmt.query(params)?;
    while let Some(row) = rows.next()? {
        let scope_id: String = row.get(0)?;
        let ref_id: String = row.get(1)?;
        let attribute_name: String = row.get(2)?;
        let stored_language: String = row.get(3)?;
        let text: String = row.get(4)?;
        let attribute: &'static str = match attribute_name.as_str() {
            "Name" => "Name",
            "VisibleDescription" => "VisibleDescription",
            // The `IN` filter above admits only these two; anything else
            // would be a logic error in this function, not real data.
            other => unreachable!("unexpected attribute_name from filtered query: {other}"),
        };
        grouped
            .entry((scope_id, ref_id, attribute))
            .or_default()
            .push((stored_language, text));
    }
    let mut resolved = HashMap::with_capacity(grouped.len());
    for (key, candidates) in grouped {
        let languages: Vec<&str> = candidates.iter().map(|(l, _)| l.as_str()).collect();
        if let Some(matched) = best_matching_language(language, &languages) {
            if let Some((_, text)) = candidates.iter().find(|(l, _)| l == matched) {
                resolved.insert(key, text.clone());
            }
        }
    }
    Ok(resolved)
}

/// The single-row lookup `apps/knx-server`'s device creation uses.
pub fn catalog_item(conn: &Connection, id: &str) -> Result<Option<CatalogItemRow>, ProductDbError> {
    let sql = format!("SELECT {CATALOG_ITEM_COLUMNS} FROM catalog_item WHERE id = ?1");
    conn.query_row(&sql, [id], row_to_catalog_item)
        .optional()
        .map_err(Into::into)
}

/// The evidence a catalog item provides for device creation.  A catalog row
/// alone is insufficient: its product, hardware, hardware-to-program and
/// application-program references must form one consistent chain before the
/// application is allowed to create a configured device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatalogItemProgram {
    Program {
        product_ref_id: String,
        hardware2program_ref_id: String,
        program_id: String,
    },
    /// The product's hardware explicitly says it has no application program.
    /// This is the only database-evidenced empty-program case that remains
    /// creatable; an absent or broken relation is not silently treated alike.
    Programless { product_ref_id: String },
}

#[derive(Debug)]
pub enum CatalogItemRelationError {
    ProductMissing {
        product_ref_id: String,
    },
    HardwareMissing {
        product_ref_id: String,
        hardware_id: String,
    },
    Hardware2ProgramMissing {
        hardware2program_ref_id: String,
    },
    Hardware2ProgramMismatch {
        product_ref_id: String,
        hardware2program_ref_id: String,
    },
    ProgramMissing {
        hardware2program_ref_id: String,
        program_ref_id: Option<String>,
    },
    Database(ProductDbError),
}

impl From<rusqlite::Error> for CatalogItemRelationError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Database(ProductDbError::from(error))
    }
}

impl std::fmt::Display for CatalogItemRelationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProductMissing { product_ref_id } => {
                write!(f, "catalog product relation is missing: {product_ref_id}")
            }
            Self::HardwareMissing { hardware_id, .. } => {
                write!(
                    f,
                    "catalog product hardware relation is missing: {hardware_id}"
                )
            }
            Self::Hardware2ProgramMissing {
                hardware2program_ref_id,
            } => write!(
                f,
                "catalog hardware-to-program relation is missing: {hardware2program_ref_id}"
            ),
            Self::Hardware2ProgramMismatch {
                product_ref_id,
                hardware2program_ref_id,
            } => write!(
                f,
                "catalog product {product_ref_id} and hardware-to-program {hardware2program_ref_id} do not share hardware"
            ),
            Self::ProgramMissing {
                hardware2program_ref_id,
                program_ref_id,
            } => write!(
                f,
                "catalog hardware-to-program {hardware2program_ref_id} has no installed application program{}",
                program_ref_id
                    .as_deref()
                    .map(|id| format!(": {id}"))
                    .unwrap_or_default()
            ),
            Self::Database(error) => write!(f, "product database query failed: {error}"),
        }
    }
}

/// Resolves a catalog item only when all persisted relations support creating
/// a device.  Kept in the product database because these are normalized
/// manufacturer-data facts, not server/UI policy.
pub fn resolve_catalog_item_program(
    conn: &Connection,
    item: &CatalogItemRow,
) -> Result<CatalogItemProgram, CatalogItemRelationError> {
    let product_ref_id = item
        .product_ref_id
        .as_deref()
        .filter(|id| !id.is_empty())
        .ok_or_else(|| CatalogItemRelationError::ProductMissing {
            product_ref_id: "(absent)".into(),
        })?;
    let product: Option<(String, Option<i64>)> = conn
        .query_row(
            "SELECT p.hardware_id, h.has_application_program
             FROM product p LEFT JOIN hardware h ON h.id = p.hardware_id
             WHERE p.id = ?1",
            [product_ref_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let Some((hardware_id, has_application_program)) = product else {
        return Err(CatalogItemRelationError::ProductMissing {
            product_ref_id: product_ref_id.into(),
        });
    };

    let hardware_exists: Option<i64> = conn
        .query_row(
            "SELECT 1 FROM hardware WHERE id = ?1",
            [&hardware_id],
            |row| row.get(0),
        )
        .optional()?;
    if hardware_exists.is_none() {
        return Err(CatalogItemRelationError::HardwareMissing {
            product_ref_id: product_ref_id.into(),
            hardware_id,
        });
    }

    let Some(hardware2program_ref_id) = item
        .hardware2program_ref_id
        .as_deref()
        .filter(|id| !id.is_empty())
    else {
        return if has_application_program == Some(0) {
            Ok(CatalogItemProgram::Programless {
                product_ref_id: product_ref_id.into(),
            })
        } else {
            Err(CatalogItemRelationError::Hardware2ProgramMissing {
                hardware2program_ref_id: "(absent)".into(),
            })
        };
    };
    let h2p: Option<(String, Option<String>)> = conn
        .query_row(
            "SELECT hardware_id, application_program_ref FROM hardware2program WHERE id = ?1",
            [hardware2program_ref_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let Some((h2p_hardware_id, program_ref_id)) = h2p else {
        return Err(CatalogItemRelationError::Hardware2ProgramMissing {
            hardware2program_ref_id: hardware2program_ref_id.into(),
        });
    };
    if h2p_hardware_id != hardware_id {
        return Err(CatalogItemRelationError::Hardware2ProgramMismatch {
            product_ref_id: product_ref_id.into(),
            hardware2program_ref_id: hardware2program_ref_id.into(),
        });
    }
    let Some(program_id) = program_ref_id.as_deref() else {
        return Err(CatalogItemRelationError::ProgramMissing {
            hardware2program_ref_id: hardware2program_ref_id.into(),
            program_ref_id,
        });
    };
    let exists: Option<i64> = conn
        .query_row(
            "SELECT 1 FROM application_program WHERE id = ?1",
            [program_id],
            |row| row.get(0),
        )
        .optional()?;
    if exists.is_none() {
        return Err(CatalogItemRelationError::ProgramMissing {
            hardware2program_ref_id: hardware2program_ref_id.into(),
            program_ref_id: Some(program_id.into()),
        });
    }
    Ok(CatalogItemProgram::Program {
        product_ref_id: product_ref_id.into(),
        hardware2program_ref_id: hardware2program_ref_id.into(),
        program_id: program_id.into(),
    })
}

/// The evidence `apps/knx-server` overlays onto `DeviceProductNode::catalog`
/// (T16) — everything a device's stated `product_ref`/`hardware2program_ref`
/// pair resolves to, in one row rather than one query per table, joined the
/// same permissive way `resolve_catalog_item_program`'s own chain is
/// modelled: `product.id` → `product.hardware_id` → `hardware`,
/// `hardware2program.id` → `hardware2program.application_program_ref` →
/// `application_program`, but only when the link names the product's own
/// hardware, plus `catalog_item` (optional — a product need
/// not be listed in any catalog section) and `manufacturer` (for the
/// display name).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceProductRow {
    pub manufacturer_id: String,
    pub manufacturer_name: Option<String>,
    /// `product.text`.
    pub product_text: Option<String>,
    /// `product.order_number`.
    pub order_number: Option<String>,
    /// `hardware.name`.
    pub hardware_name: Option<String>,
    /// `hardware.version_number`.
    pub hardware_version: Option<String>,
    pub hardware_serial_number: Option<String>,
    /// `catalog_item.name`.
    pub catalog_item_name: Option<String>,
    pub catalog_item_number: Option<String>,
    pub application_program_id: Option<String>,
    /// `application_program.name`.
    pub application_name: Option<String>,
    pub application_number: Option<String>,
    pub application_version: Option<String>,
    pub mask_version: Option<String>,
    /// Whether the requested program link belongs to this product's
    /// hardware. Mismatched links never contribute program metadata.
    pub program_relation: DeviceProgramRelation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceProgramRelation {
    Matched,
    Missing,
    HardwareMismatch,
}

/// The untranslated row, plus the two extra ids `device_product`'s
/// `Some(lang)` branch needs to resolve a translation's `scope_id` but that
/// `DeviceProductRow` itself has no reason to expose: `catalog_item.id`
/// (the overlay `ref_id` for `catalog_item.name` — `device_product`'s own
/// `product_ref_id`/`hardware2program_ref_id` arguments name the *product*
/// and its *program link*, not the catalog entry) and
/// `catalog_item.manufacturer_id` (the overlay `scope_id` — Catalog-scope
/// rows are keyed by the manufacturer's RefId, same as `catalog_items`' own
/// join, and a catalog item's manufacturer is not assumed equal to the
/// product's merely because they usually are).
struct DeviceProductRawRow {
    row: DeviceProductRow,
    catalog_item_id: Option<String>,
    catalog_item_manufacturer_id: Option<String>,
}

fn row_to_device_product_raw(r: &rusqlite::Row) -> rusqlite::Result<DeviceProductRawRow> {
    let product_hardware_id: String = r.get(16)?;
    let program_hardware_id: Option<String> = r.get(17)?;
    let program_relation = match program_hardware_id.as_deref() {
        None => DeviceProgramRelation::Missing,
        Some(id) if id == product_hardware_id => DeviceProgramRelation::Matched,
        Some(_) => DeviceProgramRelation::HardwareMismatch,
    };
    Ok(DeviceProductRawRow {
        row: DeviceProductRow {
            manufacturer_id: r.get(0)?,
            manufacturer_name: r.get(1)?,
            product_text: r.get(2)?,
            order_number: r.get(3)?,
            hardware_name: r.get(4)?,
            hardware_version: r.get(5)?,
            hardware_serial_number: r.get(6)?,
            catalog_item_name: r.get(7)?,
            catalog_item_number: r.get(8)?,
            application_program_id: r.get(9)?,
            application_name: r.get(10)?,
            application_number: r.get(11)?,
            application_version: r.get(12)?,
            mask_version: r.get(13)?,
            program_relation,
        },
        catalog_item_id: r.get(14)?,
        catalog_item_manufacturer_id: r.get(15)?,
    })
}

/// Resolves a device's `product_ref`/`hardware2program_ref` pair against
/// the product database. `Ok(None)` only when `product_ref_id` itself does
/// not name a `product` row — a broken or unresolvable `hardware2program`
/// reference does not empty the result, it just leaves the `application_*`
/// fields `None` (a *partial* result, not an absent one — losing the
/// product's own name because its program link happens to be broken would
/// be exactly the silent discard CLAUDE.md forbids). Callers pass an empty
/// string for either id to mean "not stated"; an empty string never names a
/// row in this schema, so it behaves exactly like an id that doesn't
/// resolve.
///
/// `language: None` issues the plain, untranslated statement and returns —
/// one query, no overlay lookups at all (Global Constraint 3). `Some(lang)`
/// runs the same base statement (it always does, now — the two used to be
/// separate near-duplicate `SELECT`s, one of them `COALESCE`d against three
/// `LEFT JOIN translation ... AND language = ?3`s; that shape could not
/// express R2's locale-prefix matching, since a `JOIN ... ON` equality
/// cannot pick a different winning stored language per row, so each of the
/// three attributes is now resolved separately, in Rust, through the one
/// shared `overlay_one` point-query helper instead) and then overlays three
/// attributes through the shared `translation` table, each scoped exactly
/// the way its owning file's `Languages` block keys it
/// (`parse/translation.rs`): `product.text` (`scope = 'Hardware'`,
/// `scope_id = product.manufacturer_id`, `ref_id = product.id`,
/// `attribute_name = 'Text'`), `catalog_item.name` (`scope = 'Catalog'`,
/// `scope_id = catalog_item.manufacturer_id`, `ref_id = catalog_item.id`,
/// `attribute_name = 'Name'` — identical to `catalog_items`' own join, and
/// only attempted when a catalog item actually resolved), and
/// `application_program.name` (`scope = 'Program'`,
/// `scope_id = application_program.id`, `ref_id = application_program.id`,
/// `attribute_name = 'Name'`, only attempted when a program actually
/// resolved).
///
/// `hardware.name` is deliberately never overlaid, on an observation rather
/// than a rule: across every `Hardware.xml` this project has ingested — nine
/// files from seven manufacturers, drawn from the five packages under
/// `OriginalData/ProductDatabases/` plus the manufacturer packages extracted
/// from the two reference ETS exports (which are the *same* installation
/// exported from ETS 4 and ETS 6, not two independent ones) — every
/// `TranslationElement/@RefId` inside a `Languages` block is a `Product/@Id`,
/// and none is the owning `Hardware/@Id`. Zero counter-examples, but the KNX
/// App XML schema does not itself forbid one, so this is a statement about
/// the corpus and not about the format. Adding a join for an attribute no
/// observed package populates would silently match nothing; that is stated
/// here rather than guessed at in SQL. If a future package turns out to carry
/// one, this is the doc comment to correct.
///
/// Every join below is a `LEFT JOIN` on purpose: a product whose hardware,
/// hardware-to-program link or catalogue entry was never ingested still
/// returns its own row with the missing half `None`, rather than collapsing
/// to `Ok(None)` and losing the product name too. A program link belonging
/// to different hardware is likewise retained as
/// `DeviceProgramRelation::HardwareMismatch`, but cannot populate any
/// application or catalog columns. The absent `hardware2program` half is covered by
/// `device_product_with_no_matching_hardware2program_is_a_partial_row_not_none`;
/// the `hardware` half is not, because a product ingested without its own
/// `Hardware` element has not been observed. Flipping that one join to an
/// `INNER JOIN` would therefore pass the suite — worth knowing before anyone
/// touches this query.
pub fn device_product(
    conn: &Connection,
    product_ref_id: &str,
    hardware2program_ref_id: &str,
    language: Option<&str>,
) -> Result<Option<DeviceProductRow>, ProductDbError> {
    const SQL: &str = "SELECT p.manufacturer_id, m.name, p.text, p.order_number,
                h.name, h.version_number, h.serial_number,
                ci.name, ci.number,
                apg.id, apg.name, apg.application_number,
                apg.application_version, apg.mask_version,
                ci.id, ci.manufacturer_id,
                p.hardware_id, h2p.hardware_id
         FROM product p
         LEFT JOIN hardware h ON h.id = p.hardware_id
         LEFT JOIN manufacturer m ON m.id = p.manufacturer_id
         LEFT JOIN hardware2program h2p ON h2p.id = ?2
         LEFT JOIN application_program apg ON apg.id = h2p.application_program_ref
              AND h2p.hardware_id = p.hardware_id
         LEFT JOIN catalog_item ci ON ci.product_ref_id = ?1
              AND ci.hardware2program_ref_id = ?2
              AND h2p.hardware_id = p.hardware_id
         WHERE p.id = ?1";
    let raw = conn
        .query_row(
            SQL,
            rusqlite::params![product_ref_id, hardware2program_ref_id],
            row_to_device_product_raw,
        )
        .optional()?;
    let Some(raw) = raw else {
        return Ok(None);
    };
    let Some(lang) = language else {
        return Ok(Some(raw.row));
    };

    let mut row = raw.row;
    if let Some(text) = overlay_one(
        conn,
        "Hardware",
        &row.manufacturer_id,
        product_ref_id,
        "Text",
        lang,
    )? {
        row.product_text = Some(text);
    }
    if let (Some(item_id), Some(item_manufacturer_id)) =
        (&raw.catalog_item_id, &raw.catalog_item_manufacturer_id)
    {
        if let Some(name) =
            overlay_one(conn, "Catalog", item_manufacturer_id, item_id, "Name", lang)?
        {
            row.catalog_item_name = Some(name);
        }
    }
    if let Some(program_id) = &row.application_program_id {
        if let Some(name) = overlay_one(conn, "Program", program_id, program_id, "Name", lang)? {
            row.application_name = Some(name);
        }
    }
    Ok(Some(row))
}

/// Every `com_object_ref.id` for `program_id`, in document/ingest order.
/// `ORDER BY rowid` rather than `ORDER BY id`: `com_object_ref` is not
/// declared `WITHOUT ROWID`, so `rowid` preserves insertion order, and the
/// ids themselves (`A-1_O-1_R-1`, `A-1_O-1_R-10`, `A-1_O-1_R-2`, …) do not
/// sort into that order lexically.
pub fn com_object_ref_ids(
    conn: &Connection,
    program_id: &str,
) -> Result<Vec<String>, ProductDbError> {
    let mut stmt =
        conn.prepare("SELECT id FROM com_object_ref WHERE program_id = ?1 ORDER BY rowid")?;
    let rows = stmt
        .query_map([program_id], |r| r.get(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// One `datapoint_type` row (R1, design D10 slice 1): `parse/master.rs`'s
/// `ingest_master_data` already fills this table from `knx_master.xml`'s
/// `DatapointTypes`/`DatapointSubtypes`, `INSERT OR IGNORE`d, untranslated.
/// Nothing read it before this — `grep`ping for `datapoint_type` outside
/// `migration.rs` and this module turned up only two unrelated test names
/// in `enrich.rs`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatapointTypeRow {
    /// `DPT-<main>` or `DPST-<main>-<sub>`, verbatim as `master.rs` stores
    /// it.
    pub id: String,
    pub main: i64,
    pub sub: Option<i64>,
    /// `DatapointType`/`DatapointSubtype`'s own `@Name`. Never overlaid: no
    /// package in this project's corpus carries a `Master`-scope
    /// translation for it (this function's own doc comment states the
    /// evidence), only for `text`.
    pub name: Option<String>,
    /// `DatapointType`/`DatapointSubtype`'s own `@Text`, overlaid from a
    /// `Master`-scope translation in `language` when one resolves — the
    /// stored, untranslated value otherwise. A missing translation is never
    /// an error and never turns this into `Some("")`.
    pub text: Option<String>,
}

fn row_to_datapoint_type(r: &rusqlite::Row) -> rusqlite::Result<DatapointTypeRow> {
    Ok(DatapointTypeRow {
        id: r.get(0)?,
        main: r.get(1)?,
        sub: r.get(2)?,
        name: r.get(3)?,
        text: r.get(4)?,
    })
}

const DATAPOINT_TYPE_COLUMNS: &str = "id, main, sub, name, text";

/// `(ref_id -> text)` for every `Master`-scope, `Text`-attribute
/// translation in `language`, resolved through `best_matching_language`
/// exactly as `translation_overlay` resolves `Program`-scope rows — except
/// there is only ever one `scope_id` to consider here (`''`, the sentinel
/// `migrate_v3_to_v4` and `parse/master.rs` both use for "no scope"), so
/// this groups by `ref_id` alone rather than `(scope_id, ref_id)`.
/// Restricted to `attribute_name = 'Text'` on measured evidence, not
/// convenience: every `TranslationElement` under every sampled package's
/// `knx_master.xml` `<Languages>` block carries `AttributeName="Text"`
/// ([V], n=5 packages under `OriginalData/ProductDatabases/` —
/// `646704-04_ETS4_2012_47_DE_EN`, both `Weinzierl_730_KNX_IP_Interface_ETS4`
/// variants, `MDT_KP_AMI_AMS_03_Switch_Actuator_V31a`,
/// `Dummy_Applikation_Secure`; no other `AttributeName` value was seen).
///
/// This function's `ref_id -> text` map is entity-agnostic — it has one row
/// per translated `RefId` regardless of which table (if any) that `RefId`
/// names a row in — so it already serves `function_types`/`space_usages`
/// below the same way it serves `datapoint_types` above; whether a given
/// `RefId` family resolves to anything depends only on whether the caller
/// joins it against a table that has that id, not on this function.
fn master_text_overlay(
    conn: &Connection,
    language: &str,
) -> Result<HashMap<String, String>, ProductDbError> {
    let mut stmt = conn.prepare(
        "SELECT ref_id, language, text FROM translation
         WHERE scope = 'Master' AND scope_id = '' AND attribute_name = 'Text'
           AND text IS NOT NULL",
    )?;
    let mut grouped: HashMap<String, Vec<(String, String)>> = HashMap::new();
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let ref_id: String = row.get(0)?;
        let stored_language: String = row.get(1)?;
        let text: String = row.get(2)?;
        grouped
            .entry(ref_id)
            .or_default()
            .push((stored_language, text));
    }
    let mut resolved = HashMap::with_capacity(grouped.len());
    for (ref_id, candidates) in grouped {
        let languages: Vec<&str> = candidates.iter().map(|(l, _)| l.as_str()).collect();
        if let Some(matched) = best_matching_language(language, &languages) {
            if let Some((_, text)) = candidates.iter().find(|(l, _)| l == matched) {
                resolved.insert(ref_id, text.clone());
            }
        }
    }
    Ok(resolved)
}

/// Every `datapoint_type` row, `main` then `sub` ascending (`sub: None`,
/// the main type's own row, sorts before its subtypes), with `text`
/// overlaid from a `Master`-scope translation in `language` when one
/// resolves. `language: None` is the untranslated behaviour and issues no
/// `translation` query at all, same convention as every other overlay in
/// this file (Global Constraint 3).
pub fn datapoint_types(
    conn: &Connection,
    language: Option<&str>,
) -> Result<Vec<DatapointTypeRow>, ProductDbError> {
    let sql = format!("SELECT {DATAPOINT_TYPE_COLUMNS} FROM datapoint_type ORDER BY main, sub");
    let mut stmt = conn.prepare(&sql)?;
    let rows: Vec<DatapointTypeRow> = stmt
        .query_map([], row_to_datapoint_type)?
        .collect::<Result<Vec<_>, _>>()?;
    let Some(lang) = language else {
        return Ok(rows);
    };
    let overlay = master_text_overlay(conn, lang)?;
    Ok(rows
        .into_iter()
        .map(|mut row| {
            if let Some(text) = overlay.get(&row.id) {
                row.text = Some(text.clone());
            }
            row
        })
        .collect())
}

/// The single-row lookup — `datapoint_types` narrowed to one `id`, for a
/// caller that already has one (a communication object's `DatapointType`)
/// rather than wanting the whole catalogue.
pub fn datapoint_type(
    conn: &Connection,
    id: &str,
    language: Option<&str>,
) -> Result<Option<DatapointTypeRow>, ProductDbError> {
    let sql = format!("SELECT {DATAPOINT_TYPE_COLUMNS} FROM datapoint_type WHERE id = ?1");
    let row: Option<DatapointTypeRow> = conn
        .query_row(&sql, [id], row_to_datapoint_type)
        .optional()?;
    let Some(mut row) = row else {
        return Ok(None);
    };
    let Some(lang) = language else {
        return Ok(Some(row));
    };
    if let Some(text) = overlay_one(conn, "Master", "", &row.id, "Text", lang)? {
        row.text = Some(text);
    }
    Ok(Some(row))
}

/// One `function_type` row (design D10, closing `docs/KNOWN_LIMITATIONS.md`
/// §64's last residue): `parse/master.rs`'s `ingest_master_data` fills this
/// table from `knx_master.xml`'s `FunctionTypes`, `INSERT OR IGNORE`d,
/// untranslated, the same shape `datapoint_type` above already used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionTypeRow {
    /// `FT-<n>`, verbatim as `master.rs` stores it.
    pub id: String,
    pub number: Option<i64>,
    /// `FunctionType`'s own `@Text`, overlaid from a `Master`-scope
    /// translation in `language` when one resolves.
    pub text: Option<String>,
    pub status: Option<String>,
}

fn row_to_function_type(r: &rusqlite::Row) -> rusqlite::Result<FunctionTypeRow> {
    Ok(FunctionTypeRow {
        id: r.get(0)?,
        number: r.get(1)?,
        text: r.get(2)?,
        status: r.get(3)?,
    })
}

const FUNCTION_TYPE_COLUMNS: &str = "id, number, text, status";

/// Every `function_type` row, `number` ascending, `text` overlaid the same
/// way `datapoint_types` overlays its own — `language: None` skips the
/// `translation` query entirely.
pub fn function_types(
    conn: &Connection,
    language: Option<&str>,
) -> Result<Vec<FunctionTypeRow>, ProductDbError> {
    let sql = format!("SELECT {FUNCTION_TYPE_COLUMNS} FROM function_type ORDER BY number");
    let mut stmt = conn.prepare(&sql)?;
    let rows: Vec<FunctionTypeRow> = stmt
        .query_map([], row_to_function_type)?
        .collect::<Result<Vec<_>, _>>()?;
    let Some(lang) = language else {
        return Ok(rows);
    };
    let overlay = master_text_overlay(conn, lang)?;
    Ok(rows
        .into_iter()
        .map(|mut row| {
            if let Some(text) = overlay.get(&row.id) {
                row.text = Some(text.clone());
            }
            row
        })
        .collect())
}

/// The single-row lookup — `function_types` narrowed to one `id`.
pub fn function_type(
    conn: &Connection,
    id: &str,
    language: Option<&str>,
) -> Result<Option<FunctionTypeRow>, ProductDbError> {
    let sql = format!("SELECT {FUNCTION_TYPE_COLUMNS} FROM function_type WHERE id = ?1");
    let row: Option<FunctionTypeRow> = conn
        .query_row(&sql, [id], row_to_function_type)
        .optional()?;
    let Some(mut row) = row else {
        return Ok(None);
    };
    let Some(lang) = language else {
        return Ok(Some(row));
    };
    if let Some(text) = overlay_one(conn, "Master", "", &row.id, "Text", lang)? {
        row.text = Some(text);
    }
    Ok(Some(row))
}

/// One `function_point` row, nested under its owning `FunctionType`. Its own
/// `RefId` family (`FP-*_DR-*` per §64's earlier survey, and this branch's
/// fixtures spell it the same way) can carry `Master`-scope translations
/// too, overlaid the same way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionPointRow {
    pub id: String,
    pub function_type_id: String,
    pub datapoint_type: Option<String>,
    pub role: Option<String>,
    pub characteristics: Option<String>,
    pub text: Option<String>,
}

fn row_to_function_point(r: &rusqlite::Row) -> rusqlite::Result<FunctionPointRow> {
    Ok(FunctionPointRow {
        id: r.get(0)?,
        function_type_id: r.get(1)?,
        datapoint_type: r.get(2)?,
        role: r.get(3)?,
        characteristics: r.get(4)?,
        text: r.get(5)?,
    })
}

const FUNCTION_POINT_COLUMNS: &str =
    "id, function_type_id, datapoint_type, role, characteristics, text";

/// Every `function_point` row belonging to one `function_type_id`, `id`
/// ascending, `text` overlaid the same way `function_types` overlays its
/// own.
pub fn function_points(
    conn: &Connection,
    function_type_id: &str,
    language: Option<&str>,
) -> Result<Vec<FunctionPointRow>, ProductDbError> {
    let sql =
        format!("SELECT {FUNCTION_POINT_COLUMNS} FROM function_point WHERE function_type_id = ?1 ORDER BY id");
    let mut stmt = conn.prepare(&sql)?;
    let rows: Vec<FunctionPointRow> = stmt
        .query_map([function_type_id], row_to_function_point)?
        .collect::<Result<Vec<_>, _>>()?;
    let Some(lang) = language else {
        return Ok(rows);
    };
    let overlay = master_text_overlay(conn, lang)?;
    Ok(rows
        .into_iter()
        .map(|mut row| {
            if let Some(text) = overlay.get(&row.id) {
                row.text = Some(text.clone());
            }
            row
        })
        .collect())
}

/// One `space_usage` row (design D10, closing §64's last residue alongside
/// `FunctionTypeRow` above): `parse/master.rs`'s `ingest_master_data` fills
/// this table from `knx_master.xml`'s `SpaceUsages`, `INSERT OR IGNORE`d,
/// untranslated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpaceUsageRow {
    /// `SU-<n>`, verbatim as `master.rs` stores it.
    pub id: String,
    pub number: Option<i64>,
    /// `SpaceUsage`'s own `@Text`, overlaid from a `Master`-scope
    /// translation in `language` when one resolves.
    pub text: Option<String>,
}

fn row_to_space_usage(r: &rusqlite::Row) -> rusqlite::Result<SpaceUsageRow> {
    Ok(SpaceUsageRow {
        id: r.get(0)?,
        number: r.get(1)?,
        text: r.get(2)?,
    })
}

const SPACE_USAGE_COLUMNS: &str = "id, number, text";

/// Every `space_usage` row, `number` ascending, `text` overlaid the same way
/// `datapoint_types` overlays its own.
pub fn space_usages(
    conn: &Connection,
    language: Option<&str>,
) -> Result<Vec<SpaceUsageRow>, ProductDbError> {
    let sql = format!("SELECT {SPACE_USAGE_COLUMNS} FROM space_usage ORDER BY number");
    let mut stmt = conn.prepare(&sql)?;
    let rows: Vec<SpaceUsageRow> = stmt
        .query_map([], row_to_space_usage)?
        .collect::<Result<Vec<_>, _>>()?;
    let Some(lang) = language else {
        return Ok(rows);
    };
    let overlay = master_text_overlay(conn, lang)?;
    Ok(rows
        .into_iter()
        .map(|mut row| {
            if let Some(text) = overlay.get(&row.id) {
                row.text = Some(text.clone());
            }
            row
        })
        .collect())
}

/// The single-row lookup — `space_usages` narrowed to one `id`.
pub fn space_usage(
    conn: &Connection,
    id: &str,
    language: Option<&str>,
) -> Result<Option<SpaceUsageRow>, ProductDbError> {
    let sql = format!("SELECT {SPACE_USAGE_COLUMNS} FROM space_usage WHERE id = ?1");
    let row: Option<SpaceUsageRow> = conn.query_row(&sql, [id], row_to_space_usage).optional()?;
    let Some(mut row) = row else {
        return Ok(None);
    };
    let Some(lang) = language else {
        return Ok(Some(row));
    };
    if let Some(text) = overlay_one(conn, "Master", "", &row.id, "Text", lang)? {
        row.text = Some(text);
    }
    Ok(Some(row))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_and_migrate;
    use crate::parse::{
        catalog::ingest_catalog,
        hardware::ingest_hardware,
        program::ingest_program,
        translation::{ingest_translations, insert_translations, TranslationScope},
    };

    const HARDWARE: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-006A">
<Hardware><Hardware Id="H-1" Name="X" SerialNumber="S" VersionNumber="1">
<Products><Product Id="M-006A_H-1_P-1" /></Products>
<Hardware2Programs><Hardware2Program Id="H-1_HP-1" MediumTypes="MT-0">
<ApplicationProgramRef RefId="A-1" /></Hardware2Program></Hardware2Programs>
</Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#;

    const PROGRAM: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-006A">
<ApplicationPrograms><ApplicationProgram Id="A-1" Name="P" ApplicationNumber="1"
  ApplicationVersion="22" MaskVersion="MV-0701"><Static>
<ComObjectTable>
  <ComObject Id="A-1_O-1" Number="1" Text="Schalten" ObjectSize="1 Bit"
             DatapointType="DPST-1-1" WriteFlag="Enabled" ReadFlag="Disabled" />
</ComObjectTable>
<ComObjectRefs>
  <ComObjectRef Id="A-1_O-1_R-1" RefId="A-1_O-1" />
  <ComObjectRef Id="A-1_O-1_R-2" RefId="A-1_O-1" Text="Dimmen" DatapointType="DPST-3-7" />
</ComObjectRefs>
</Static></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

    fn db() -> (tempfile::TempDir, Connection) {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
        ingest_hardware(&conn, "sha-h", "M-006A/Hardware.xml", HARDWARE.as_bytes()).unwrap();
        ingest_program(&conn, "sha-p", "M-006A/A.xml", PROGRAM.as_bytes()).unwrap();
        (dir, conn)
    }

    const CATALOG: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11">
  <ManufacturerData>
    <Manufacturer RefId="M-006A">
      <Catalog>
        <CatalogSection Id="M-006A_CG-1" Name="Actuators" Number="1" DefaultLanguage="de-DE">
          <CatalogItem Id="M-006A_CI-1" Name="Schaltaktor" Number="EM12102"
                       DefaultLanguage="de-DE"
                       ProductRefId="M-006A_H-1_P-1"
                       Hardware2ProgramRefId="H-1_HP-1" />
        </CatalogSection>
      </Catalog>
    </Manufacturer>
  </ManufacturerData>
</KNX>"#;

    #[test]
    fn catalog_items_lists_and_filters_by_manufacturer_and_search() {
        let (_dir, conn) = db();
        ingest_catalog(&conn, "sha-c", "M-006A/Catalog.xml", CATALOG.as_bytes()).unwrap();

        let all = catalog_items(&conn, None, None, None).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, "M-006A_CI-1");

        assert_eq!(
            catalog_items(&conn, Some("M-006A"), None, None)
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            catalog_items(&conn, Some("M-999X"), None, None)
                .unwrap()
                .len(),
            0
        );
        assert_eq!(
            catalog_items(&conn, None, Some("schalt"), None)
                .unwrap()
                .len(),
            1,
            "search is case-insensitive"
        );
        assert_eq!(
            catalog_items(&conn, None, Some("EM12102"), None)
                .unwrap()
                .len(),
            1,
            "search also matches on number"
        );
        assert_eq!(
            catalog_items(&conn, None, Some("nope"), None)
                .unwrap()
                .len(),
            0
        );
    }

    /// A `de-DE` translation of the catalog item's `Name`, mirroring
    /// `parse::translation`'s own `CATALOG` fixture: the `Languages` block
    /// is a sibling of `Catalog`, scoped by `Manufacturer/@RefId`, not the
    /// item's own id (T32 Task 1/2's `(scope, scope_id)` key).
    const CATALOG_WITH_TRANSLATION: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11">
  <ManufacturerData>
    <Manufacturer RefId="M-006A">
      <Catalog>
        <CatalogSection Id="M-006A_CG-1" Name="Actuators" Number="1" DefaultLanguage="de-DE">
          <CatalogItem Id="M-006A_CI-1" Name="Schaltaktor" Number="EM12102"
                       DefaultLanguage="de-DE"
                       ProductRefId="M-006A_H-1_P-1"
                       Hardware2ProgramRefId="H-1_HP-1" />
        </CatalogSection>
      </Catalog>
      <Languages>
        <Language Identifier="de-DE">
          <TranslationUnit RefId="M-006A_CI-1">
            <TranslationElement RefId="M-006A_CI-1">
              <Translation AttributeName="Name" Text="Umschaltaktor" />
            </TranslationElement>
          </TranslationUnit>
        </Language>
      </Languages>
    </Manufacturer>
  </ManufacturerData>
</KNX>"#;

    /// `ingest_catalog` alone never reads `Catalog.xml`'s own `Languages`
    /// block (see `ingest.rs`'s own `ingest_file_in_transaction` doc
    /// comment) — the second `ingest_translations` pass below is what
    /// actually does, mirroring how `ingest_file` runs both in one
    /// transaction outside of tests.
    fn ingest_catalog_with_translation(conn: &Connection) {
        ingest_catalog(
            conn,
            "sha-c",
            "M-006A/Catalog.xml",
            CATALOG_WITH_TRANSLATION.as_bytes(),
        )
        .unwrap();
        ingest_translations(
            conn,
            TranslationScope::Catalog,
            "M-006A/Catalog.xml",
            CATALOG_WITH_TRANSLATION.as_bytes(),
        )
        .unwrap();
    }

    #[test]
    fn catalog_items_without_a_language_returns_the_stored_name_unchanged() {
        let (_dir, conn) = db();
        ingest_catalog_with_translation(&conn);

        let items = catalog_items(&conn, None, None, None).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name.as_deref(), Some("Schaltaktor"));
    }

    #[test]
    fn catalog_items_with_a_language_overlays_the_translated_name() {
        let (_dir, conn) = db();
        ingest_catalog_with_translation(&conn);

        let items = catalog_items(&conn, None, None, Some("de-DE")).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name.as_deref(), Some("Umschaltaktor"));
    }

    #[test]
    fn catalog_items_search_finds_a_translated_only_match_only_when_the_language_is_set() {
        let (_dir, conn) = db();
        ingest_catalog_with_translation(&conn);

        assert_eq!(
            catalog_items(&conn, None, Some("umschalt"), Some("de-DE"))
                .unwrap()
                .len(),
            1,
            "the translated name is searchable once a language is set"
        );
        assert_eq!(
            catalog_items(&conn, None, Some("umschalt"), None)
                .unwrap()
                .len(),
            0,
            "without a language the search only sees the stored (untranslated) name"
        );
    }

    #[test]
    fn catalog_items_with_a_language_that_has_no_rows_returns_stored_names_unchanged() {
        let (_dir, conn) = db();
        ingest_catalog_with_translation(&conn);

        let items = catalog_items(&conn, None, None, Some("fr-FR")).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name.as_deref(), Some("Schaltaktor"));
    }

    #[test]
    fn catalog_items_matches_a_short_locale_request_by_prefix() {
        // R2, applied to `catalog_overlay`: the fixture only stores
        // `de-DE`, and a bare `de` request must still find it.
        let (_dir, conn) = db();
        ingest_catalog_with_translation(&conn);

        let items = catalog_items(&conn, None, None, Some("de")).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name.as_deref(), Some("Umschaltaktor"));
    }

    #[test]
    fn catalog_item_looks_up_a_single_row_by_id() {
        let (_dir, conn) = db();
        ingest_catalog(&conn, "sha-c", "M-006A/Catalog.xml", CATALOG.as_bytes()).unwrap();

        let item = catalog_item(&conn, "M-006A_CI-1").unwrap().unwrap();
        assert_eq!(item.manufacturer_id, "M-006A");
        assert_eq!(item.hardware2program_ref_id.as_deref(), Some("H-1_HP-1"));
        assert!(catalog_item(&conn, "nope").unwrap().is_none());
    }

    #[test]
    fn catalog_item_program_resolves_a_complete_relation_chain() {
        let (_dir, conn) = db();
        ingest_catalog(&conn, "sha-c", "M-006A/Catalog.xml", CATALOG.as_bytes()).unwrap();
        let item = catalog_item(&conn, "M-006A_CI-1").unwrap().unwrap();

        assert!(matches!(
            resolve_catalog_item_program(&conn, &item).unwrap(),
            CatalogItemProgram::Program {
                product_ref_id,
                hardware2program_ref_id,
                program_id,
            } if product_ref_id == "M-006A_H-1_P-1"
                && hardware2program_ref_id == "H-1_HP-1"
                && program_id == "A-1"
        ));
    }

    #[test]
    fn catalog_item_program_reports_a_missing_relation() {
        let (_dir, conn) = db();
        ingest_catalog(&conn, "sha-c", "M-006A/Catalog.xml", CATALOG.as_bytes()).unwrap();
        conn.execute("DELETE FROM hardware2program WHERE id = 'H-1_HP-1'", [])
            .unwrap();
        let item = catalog_item(&conn, "M-006A_CI-1").unwrap().unwrap();

        assert!(matches!(
            resolve_catalog_item_program(&conn, &item),
            Err(CatalogItemRelationError::Hardware2ProgramMissing { .. })
        ));
    }

    #[test]
    fn catalog_item_program_preserves_database_errors() {
        let (_dir, conn) = db();
        ingest_catalog(&conn, "sha-c", "M-006A/Catalog.xml", CATALOG.as_bytes()).unwrap();
        let item = catalog_item(&conn, "M-006A_CI-1").unwrap().unwrap();
        conn.execute_batch("DROP TABLE product").unwrap();

        assert!(matches!(
            resolve_catalog_item_program(&conn, &item),
            Err(CatalogItemRelationError::Database(ProductDbError::Sqlite(
                _
            )))
        ));
    }

    #[test]
    fn device_product_resolves_the_full_chain() {
        let (_dir, conn) = db();
        ingest_catalog(&conn, "sha-c", "M-006A/Catalog.xml", CATALOG.as_bytes()).unwrap();

        let row = device_product(&conn, "M-006A_H-1_P-1", "H-1_HP-1", None)
            .unwrap()
            .unwrap();
        assert_eq!(row.manufacturer_id, "M-006A");
        assert_eq!(row.manufacturer_name, None, "no master data ingested");
        assert_eq!(row.hardware_name.as_deref(), Some("X"));
        assert_eq!(row.hardware_version.as_deref(), Some("1"));
        assert_eq!(row.hardware_serial_number.as_deref(), Some("S"));
        assert_eq!(row.catalog_item_name.as_deref(), Some("Schaltaktor"));
        assert_eq!(row.catalog_item_number.as_deref(), Some("EM12102"));
        assert_eq!(row.application_program_id.as_deref(), Some("A-1"));
        assert_eq!(row.application_name.as_deref(), Some("P"));
        assert_eq!(row.application_number.as_deref(), Some("1"));
        assert_eq!(row.application_version.as_deref(), Some("22"));
        assert_eq!(row.mask_version.as_deref(), Some("MV-0701"));
    }

    #[test]
    fn device_product_with_no_matching_hardware2program_is_a_partial_row_not_none() {
        // An empty `hardware2program_ref_id` (the "not stated" convention
        // `DeviceProductNode` uses) must not swallow the product/hardware
        // half of the row it can still resolve.
        let (_dir, conn) = db();
        let row = device_product(&conn, "M-006A_H-1_P-1", "", None)
            .unwrap()
            .unwrap();
        assert_eq!(row.manufacturer_id, "M-006A");
        assert_eq!(row.hardware_name.as_deref(), Some("X"));
        assert_eq!(row.application_program_id, None);
        assert_eq!(row.application_name, None);
        assert_eq!(row.catalog_item_name, None);
    }

    #[test]
    fn device_product_does_not_attach_a_program_linked_to_other_hardware() {
        let (_dir, conn) = db();
        conn.execute_batch(
            "INSERT INTO hardware (id, manufacturer_id, name, source_sha256)
               VALUES ('H-2', 'M-006A', 'Other hardware', 'x');
             INSERT INTO product (id, manufacturer_id, hardware_id, text, source_sha256)
               VALUES ('M-006A_H-2_P-1', 'M-006A', 'H-2', 'Other product', 'x');",
        )
        .unwrap();

        let row = device_product(&conn, "M-006A_H-2_P-1", "H-1_HP-1", None)
            .unwrap()
            .unwrap();

        assert_eq!(row.product_text.as_deref(), Some("Other product"));
        assert_eq!(row.hardware_name.as_deref(), Some("Other hardware"));
        assert_eq!(row.application_program_id, None);
        assert_eq!(row.application_name, None);
        assert_eq!(
            row.program_relation,
            DeviceProgramRelation::HardwareMismatch
        );
    }

    #[test]
    fn device_product_with_an_unknown_product_id_is_none() {
        let (_dir, conn) = db();
        assert_eq!(
            device_product(&conn, "nope", "H-1_HP-1", None).unwrap(),
            None
        );
    }

    #[test]
    fn device_product_overlays_translated_text_only_when_a_language_is_given() {
        let (_dir, conn) = db();
        ingest_catalog(&conn, "sha-c", "M-006A/Catalog.xml", CATALOG.as_bytes()).unwrap();
        insert_translations(
            &conn,
            TranslationScope::Hardware,
            "M-006A",
            "de-DE",
            "M-006A_H-1_P-1",
            "Text",
            "Schaltaktor 12-fach",
        )
        .unwrap();
        insert_translations(
            &conn,
            TranslationScope::Catalog,
            "M-006A",
            "de-DE",
            "M-006A_CI-1",
            "Name",
            "Umschaltaktor",
        )
        .unwrap();
        insert_translations(
            &conn,
            TranslationScope::Program,
            "A-1",
            "de-DE",
            "A-1",
            "Name",
            "Programm P",
        )
        .unwrap();

        let untranslated = device_product(&conn, "M-006A_H-1_P-1", "H-1_HP-1", None)
            .unwrap()
            .unwrap();
        assert_eq!(untranslated.product_text, None, "Product carries no @Text");
        assert_eq!(
            untranslated.catalog_item_name.as_deref(),
            Some("Schaltaktor")
        );
        assert_eq!(untranslated.application_name.as_deref(), Some("P"));

        let translated = device_product(&conn, "M-006A_H-1_P-1", "H-1_HP-1", Some("de-DE"))
            .unwrap()
            .unwrap();
        assert_eq!(
            translated.product_text.as_deref(),
            Some("Schaltaktor 12-fach")
        );
        assert_eq!(
            translated.catalog_item_name.as_deref(),
            Some("Umschaltaktor")
        );
        assert_eq!(translated.application_name.as_deref(), Some("Programm P"));
        // `hardware.name` is never a translation target in this schema
        // (see `device_product`'s own doc comment) — untouched either way.
        assert_eq!(translated.hardware_name.as_deref(), Some("X"));
    }

    #[test]
    fn device_product_matches_a_short_locale_request_by_prefix() {
        // R2, applied to `overlay_one`'s three call sites inside
        // `device_product`: every fixture row below only stores `de-DE`.
        let (_dir, conn) = db();
        ingest_catalog(&conn, "sha-c", "M-006A/Catalog.xml", CATALOG.as_bytes()).unwrap();
        insert_translations(
            &conn,
            TranslationScope::Hardware,
            "M-006A",
            "de-DE",
            "M-006A_H-1_P-1",
            "Text",
            "Schaltaktor 12-fach",
        )
        .unwrap();
        insert_translations(
            &conn,
            TranslationScope::Catalog,
            "M-006A",
            "de-DE",
            "M-006A_CI-1",
            "Name",
            "Umschaltaktor",
        )
        .unwrap();
        insert_translations(
            &conn,
            TranslationScope::Program,
            "A-1",
            "de-DE",
            "A-1",
            "Name",
            "Programm P",
        )
        .unwrap();

        let translated = device_product(&conn, "M-006A_H-1_P-1", "H-1_HP-1", Some("de"))
            .unwrap()
            .unwrap();
        assert_eq!(
            translated.product_text.as_deref(),
            Some("Schaltaktor 12-fach")
        );
        assert_eq!(
            translated.catalog_item_name.as_deref(),
            Some("Umschaltaktor")
        );
        assert_eq!(translated.application_name.as_deref(), Some("Programm P"));
    }

    #[test]
    fn com_object_ref_ids_returns_every_ref_in_document_order() {
        let (_dir, conn) = db();
        let ids = com_object_ref_ids(&conn, "A-1").unwrap();
        assert_eq!(
            ids,
            vec!["A-1_O-1_R-1".to_string(), "A-1_O-1_R-2".to_string()]
        );
    }

    #[test]
    fn a_hardware2program_id_resolves_to_its_application_program() {
        let (_dir, conn) = db();
        assert_eq!(
            resolve_program(&conn, "H-1_HP-1").unwrap(),
            Some("A-1".into())
        );
        assert_eq!(resolve_program(&conn, "H-9_HP-9").unwrap(), None);
    }

    #[test]
    fn a_ref_without_overrides_shows_the_program_layer_values() {
        let (_dir, conn) = db();
        let v = com_object_view(&conn, "A-1", "A-1_O-1_R-1", None)
            .unwrap()
            .unwrap();
        assert_eq!(v.text.as_deref(), Some("Schalten"));
        assert_eq!(v.text_layer, ValueLayer::Program);
        assert_eq!(v.dpt_list.as_deref(), Some("DPST-1-1"));
        assert_eq!(v.dpt_layer, ValueLayer::Program);
        assert_eq!(v.number, Some(1));
        assert_eq!(v.write.as_deref(), Some("Enabled"));
    }

    #[test]
    fn a_ref_with_overrides_shows_the_program_ref_layer_for_those_values_only() {
        let (_dir, conn) = db();
        let v = com_object_view(&conn, "A-1", "A-1_O-1_R-2", None)
            .unwrap()
            .unwrap();
        assert_eq!(v.text.as_deref(), Some("Dimmen"));
        assert_eq!(v.text_layer, ValueLayer::ProgramRef);
        assert_eq!(v.dpt_list.as_deref(), Some("DPST-3-7"));
        assert_eq!(v.dpt_layer, ValueLayer::ProgramRef);
        // Not overridden: still the program's own value and layer.
        assert_eq!(v.object_size.as_deref(), Some("1 Bit"));
        assert_eq!(v.write.as_deref(), Some("Enabled"));
    }

    #[test]
    fn an_unknown_ref_id_is_none_not_an_error() {
        let (_dir, conn) = db();
        assert!(com_object_view(&conn, "A-1", "A-1_O-9_R-9", None)
            .unwrap()
            .is_none());
    }

    // -----------------------------------------------------------------
    // com_object_view's translation overlay (T33 Task 1).
    // -----------------------------------------------------------------

    /// One `ComObject` (`A-7_O-1`) carrying `Text`, `FunctionText`,
    /// `VisibleDescription` and `ObjectSize`, and two `ComObjectRef`s:
    /// `R-1` takes every value from the `ComObject` (`Program` layer),
    /// `R-2` overrides only `Text` (`ProgramRef` layer), and `R-3`
    /// overrides `Text` as well but is itself never translated. `de-DE`
    /// translates all three display attributes on the `ComObject` *and* an
    /// (illegitimate) `ObjectSize`, plus `Text` on `R-2` itself — `R-3`
    /// deliberately gets no `TranslationElement` at all.
    const COM_OBJECT_PROGRAM_TRANSLATED: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-006A">
<ApplicationPrograms><ApplicationProgram Id="A-7" Name="P" ApplicationNumber="7"
  ApplicationVersion="22" MaskVersion="MV-0701"><Static>
<ComObjectTable>
  <ComObject Id="A-7_O-1" Number="1" Text="Schalten" FunctionText="Schaltfunktion"
             VisibleDescription="Schaltbeschreibung" ObjectSize="1 Bit"
             DatapointType="DPST-1-1" />
</ComObjectTable>
<ComObjectRefs>
  <ComObjectRef Id="A-7_O-1_R-1" RefId="A-7_O-1" />
  <ComObjectRef Id="A-7_O-1_R-2" RefId="A-7_O-1" Text="Dimmen" />
  <ComObjectRef Id="A-7_O-1_R-3" RefId="A-7_O-1" Text="Sperren" />
</ComObjectRefs>
</Static>
<Languages>
  <Language Identifier="de-DE">
    <TranslationUnit RefId="A-7">
      <TranslationElement RefId="A-7_O-1">
        <Translation AttributeName="Text" Text="Schalten DE" />
        <Translation AttributeName="FunctionText" Text="Schaltfunktion DE" />
        <Translation AttributeName="VisibleDescription" Text="Schaltbeschreibung DE" />
        <Translation AttributeName="ObjectSize" Text="1 Bit DE" />
      </TranslationElement>
      <TranslationElement RefId="A-7_O-1_R-2">
        <Translation AttributeName="Text" Text="Dimmen DE" />
      </TranslationElement>
    </TranslationUnit>
  </Language>
</Languages>
</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

    fn translated_com_object_db() -> (tempfile::TempDir, Connection) {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
        ingest_program(
            &conn,
            "sha-p7",
            "M-006A/A7.xml",
            COM_OBJECT_PROGRAM_TRANSLATED.as_bytes(),
        )
        .unwrap();
        (dir, conn)
    }

    #[test]
    fn com_object_view_without_a_language_is_unchanged() {
        let (_dir, conn) = translated_com_object_db();
        let v = com_object_view(&conn, "A-7", "A-7_O-1_R-1", None)
            .unwrap()
            .unwrap();
        assert_eq!(v.text.as_deref(), Some("Schalten"));
        assert_eq!(v.function_text.as_deref(), Some("Schaltfunktion"));
        assert_eq!(v.visible_description.as_deref(), Some("Schaltbeschreibung"));
        assert_eq!(v.object_size.as_deref(), Some("1 Bit"));
    }

    #[test]
    fn a_com_object_scope_text_translation_replaces_the_program_layer_text() {
        let (_dir, conn) = translated_com_object_db();
        let v = com_object_view(&conn, "A-7", "A-7_O-1_R-1", Some("de-DE"))
            .unwrap()
            .unwrap();
        assert_eq!(v.text.as_deref(), Some("Schalten DE"));
        assert_eq!(
            v.text_layer,
            ValueLayer::Program,
            "the translation overlaid ComObject/@Text, so it still reports the Program layer"
        );
    }

    #[test]
    fn a_com_object_ref_scope_text_translation_replaces_the_program_ref_layer_text() {
        let (_dir, conn) = translated_com_object_db();
        let v = com_object_view(&conn, "A-7", "A-7_O-1_R-2", Some("de-DE"))
            .unwrap()
            .unwrap();
        assert_eq!(v.text.as_deref(), Some("Dimmen DE"));
        assert_eq!(
            v.text_layer,
            ValueLayer::ProgramRef,
            "the translation overlaid ComObjectRef/@Text, so it still reports \
             the ProgramRef layer — translation is a language dimension, not \
             a layer dimension"
        );
    }

    #[test]
    fn function_text_and_visible_description_translations_land_in_their_own_fields() {
        let (_dir, conn) = translated_com_object_db();
        let v = com_object_view(&conn, "A-7", "A-7_O-1_R-1", Some("de-DE"))
            .unwrap()
            .unwrap();
        assert_eq!(v.function_text.as_deref(), Some("Schaltfunktion DE"));
        assert_eq!(
            v.visible_description.as_deref(),
            Some("Schaltbeschreibung DE")
        );
    }

    #[test]
    fn a_language_with_no_rows_for_this_program_leaves_every_field_untranslated() {
        // `db()`'s program `A-1` declares no `Languages` block at all, so
        // `de-DE` has zero translation rows for it — no fallback to
        // another language's rows either (Global Constraint 1).
        let (_dir, conn) = db();
        let v = com_object_view(&conn, "A-1", "A-1_O-1_R-1", Some("de-DE"))
            .unwrap()
            .unwrap();
        assert_eq!(v.text.as_deref(), Some("Schalten"));
        assert_eq!(v.text_layer, ValueLayer::Program);
    }

    #[test]
    fn a_non_display_attribute_translation_never_reaches_a_com_object_view_field() {
        let (_dir, conn) = translated_com_object_db();
        let v = com_object_view(&conn, "A-7", "A-7_O-1_R-1", Some("de-DE"))
            .unwrap()
            .unwrap();
        assert_eq!(
            v.object_size.as_deref(),
            Some("1 Bit"),
            "A-7_O-1 carries a de-DE ObjectSize translation row; object_size \
             is a value, not display text, and must stay the package's own"
        );
    }

    // -----------------------------------------------------------------
    // *_translated flags and com_object_views (T34 Task 1).
    // -----------------------------------------------------------------

    #[test]
    fn an_overlay_hit_sets_the_translated_flag() {
        let (_dir, conn) = translated_com_object_db();
        let v = com_object_view(&conn, "A-7", "A-7_O-1_R-1", Some("de-DE"))
            .unwrap()
            .unwrap();
        assert!(v.text_translated);
        assert!(v.function_text_translated);
        assert!(v.visible_description_translated);
    }

    #[test]
    fn an_overlay_miss_leaves_the_translated_flag_false_and_the_value_untranslated() {
        // `A-1` (plain `db()`) declares no `<Languages>` block at all, so
        // `translation_overlay` returns an empty (not absent) map: `Some`
        // overlay, zero matching rows — the miss case, distinct from
        // `language: None` below.
        let (_dir, conn) = db();
        let v = com_object_view(&conn, "A-1", "A-1_O-1_R-1", Some("de-DE"))
            .unwrap()
            .unwrap();
        assert_eq!(v.text.as_deref(), Some("Schalten"));
        assert!(!v.text_translated);
        assert!(!v.function_text_translated);
        assert!(!v.visible_description_translated);
    }

    #[test]
    fn without_a_language_every_translated_flag_is_false() {
        let (_dir, conn) = translated_com_object_db();
        let v = com_object_view(&conn, "A-7", "A-7_O-1_R-1", None)
            .unwrap()
            .unwrap();
        assert_eq!(v.text.as_deref(), Some("Schalten"));
        assert!(!v.text_translated);
        assert!(!v.function_text_translated);
        assert!(!v.visible_description_translated);
    }

    #[test]
    fn a_com_object_ref_scope_translation_winning_over_a_com_object_scope_one_is_still_reported_translated(
    ) {
        // `A-7_O-1` itself carries a de-DE `Text` translation ("Schalten
        // DE"); `R-2` carries its own de-DE `Text` translation ("Dimmen
        // DE") and always wins `pick()` because a `ComObjectRef`-scope
        // value beats a `ComObject`-scope one whenever it is `Some`. Both
        // layers being translated at once is the point: `translated` must
        // follow the *winning* (`ProgramRef`) layer's own overlay hit, not
        // merely "some layer had one" — see `overlaid_pick`'s doc comment.
        let (_dir, conn) = translated_com_object_db();
        let v = com_object_view(&conn, "A-7", "A-7_O-1_R-2", Some("de-DE"))
            .unwrap()
            .unwrap();
        assert_eq!(v.text.as_deref(), Some("Dimmen DE"));
        assert_eq!(v.text_layer, ValueLayer::ProgramRef);
        assert!(v.text_translated);
    }

    #[test]
    fn an_untranslated_com_object_ref_override_winning_over_a_translated_com_object_is_not_translated(
    ) {
        // The reverse polarity of the test above, and the one a naive
        // `co_translated || cor_translated` would get wrong: `A-7_O-1`
        // *is* translated to de-DE, but `R-3` overrides `Text` with its
        // own untranslated "Sperren" and carries no translation of its
        // own, so the winning value is the package's raw string and
        // `text_translated` must say so. `function_text`, which `R-3`
        // does not override, rides the translated `Program` layer in the
        // same call — the flags are per field, not per view.
        let (_dir, conn) = translated_com_object_db();
        let v = com_object_view(&conn, "A-7", "A-7_O-1_R-3", Some("de-DE"))
            .unwrap()
            .unwrap();
        assert_eq!(v.text.as_deref(), Some("Sperren"));
        assert_eq!(v.text_layer, ValueLayer::ProgramRef);
        assert!(!v.text_translated);
        assert_eq!(v.function_text.as_deref(), Some("Schaltfunktion DE"));
        assert!(v.function_text_translated);
    }

    #[test]
    fn com_object_views_resolves_every_ref_matching_the_one_element_calls() {
        let (_dir, conn) = translated_com_object_db();
        let batch =
            com_object_views(&conn, "A-7", &["A-7_O-1_R-1", "A-7_O-1_R-2"], Some("de-DE")).unwrap();
        assert_eq!(batch.len(), 2);

        let single_r1 = com_object_view(&conn, "A-7", "A-7_O-1_R-1", Some("de-DE"))
            .unwrap()
            .unwrap();
        let single_r2 = com_object_view(&conn, "A-7", "A-7_O-1_R-2", Some("de-DE"))
            .unwrap()
            .unwrap();
        assert_eq!(batch["A-7_O-1_R-1"], single_r1);
        assert_eq!(batch["A-7_O-1_R-2"], single_r2);
    }

    #[test]
    fn com_object_views_omits_unknown_ref_ids_without_erroring() {
        let (_dir, conn) = db();
        let views = com_object_views(&conn, "A-1", &["A-1_O-1_R-1", "A-1_O-9_R-9"], None).unwrap();
        assert_eq!(
            views.len(),
            1,
            "the unknown id is simply absent, not an error"
        );
        assert!(views.contains_key("A-1_O-1_R-1"));
        assert!(!views.contains_key("A-1_O-9_R-9"));
    }

    #[test]
    fn com_object_views_with_an_empty_slice_touches_the_database_not_at_all() {
        let (_dir, conn) = translated_com_object_db();
        // Both tables a real query (or an overlay load) would need are
        // gone; if `com_object_views` ran either one it would return
        // `Err`, not `Ok(empty map)`.
        conn.execute_batch("DROP TABLE com_object_ref; DROP TABLE translation;")
            .unwrap();

        let views = com_object_views(&conn, "A-7", &[], Some("de-DE")).unwrap();
        assert!(views.is_empty());
    }

    #[test]
    fn com_object_views_without_a_language_never_queries_the_translation_table() {
        let (_dir, conn) = translated_com_object_db();
        // The other half of the no-language constraint: an empty slice
        // loads no overlay because it runs nothing at all, which says
        // nothing about a slice that does run. Here the rows exist and
        // the query happens; only `translation` is gone. Resolving
        // anyway is the proof that `language: None` issues zero
        // translation queries — a lazier implementation that loaded the
        // overlay first and consulted it later would return `Err`.
        conn.execute_batch("DROP TABLE translation").unwrap();

        let view = com_object_views(&conn, "A-7", &["A-7_O-1_R-1"], None)
            .unwrap()
            .remove("A-7_O-1_R-1")
            .expect("the ref resolves without the translation table");
        assert!(!view.text_translated);
        assert!(!view.function_text_translated);
        assert!(!view.visible_description_translated);
    }

    /// `count` `ComObject`/`ComObjectRef` pairs, each ref taking its
    /// `ComObject`'s own value (no overrides) — enough to synthesize a
    /// program larger than the 900-id chunk size without the corpus.
    fn com_object_table_and_refs(count: usize) -> (String, String) {
        let mut table = String::new();
        let mut refs = String::new();
        for i in 0..count {
            table.push_str(&format!(
                r#"<ComObject Id="A-9_O-{i}" Number="{i}" Text="T{i}" ObjectSize="1 Bit" />"#
            ));
            refs.push_str(&format!(
                r#"<ComObjectRef Id="A-9_O-{i}_R-1" RefId="A-9_O-{i}" />"#
            ));
        }
        (table, refs)
    }

    #[test]
    fn com_object_views_resolves_a_slice_larger_than_one_chunk() {
        // 900 ids per statement (see `com_object_views`'s own doc comment);
        // 950 forces a second, smaller chunk.
        const COUNT: usize = 950;
        let (table, refs) = com_object_table_and_refs(COUNT);
        let xml = format!(
            r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-006A">
<ApplicationPrograms><ApplicationProgram Id="A-9" Name="P" ApplicationNumber="9"
  ApplicationVersion="1" MaskVersion="MV-0701"><Static>
<ComObjectTable>{table}</ComObjectTable>
<ComObjectRefs>{refs}</ComObjectRefs>
</Static></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#
        );
        let dir = tempfile::tempdir().unwrap();
        let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
        ingest_program(&conn, "sha-a9", "M-006A/A9.xml", xml.as_bytes()).unwrap();

        let ids = com_object_ref_ids(&conn, "A-9").unwrap();
        assert_eq!(ids.len(), COUNT);
        let id_refs: Vec<&str> = ids.iter().map(String::as_str).collect();

        let views = com_object_views(&conn, "A-9", &id_refs, None).unwrap();
        assert_eq!(
            views.len(),
            COUNT,
            "every ref across both chunks must resolve, not just the first 900"
        );
        // Spot-check across the chunk boundary (indices 899/900) and both
        // ends of the slice.
        for i in [0usize, 899, 900, COUNT - 1] {
            let id = format!("A-9_O-{i}_R-1");
            let expected_text = format!("T{i}");
            let v = views.get(&id).unwrap_or_else(|| panic!("missing {id}"));
            assert_eq!(v.text.as_deref(), Some(expected_text.as_str()));
        }
    }

    #[test]
    fn listings_return_what_the_cli_prints() {
        let (_dir, conn) = db();
        assert_eq!(
            manufacturers(&conn).unwrap(),
            vec![("M-006A".to_string(), None)]
        );
        let programs = programs(&conn, Some("M-006A")).unwrap();
        assert_eq!(programs.len(), 1);
        assert_eq!(programs[0].id, "A-1");
        assert_eq!(programs[0].application_version.as_deref(), Some("22"));
    }

    // -----------------------------------------------------------------
    // parameter_views / parameter_ref_ids (T18, design D22).
    // -----------------------------------------------------------------

    /// Three `ParameterRef`s of kind `Number`, `Restriction` and `Text`
    /// respectively. `DisplayOrder` deliberately does not match id order
    /// (`PR-2` first, then `PR-3`, then `PR-1`), so a test asserting
    /// `parameter_views`'s output order actually proves it sorts by
    /// `display_order` and not by id or insertion order.
    const PARAMETER_PROGRAM: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-006A">
<ApplicationPrograms><ApplicationProgram Id="A-2" Name="P" ApplicationNumber="2"
  ApplicationVersion="22" MaskVersion="MV-0701"><Static>
<ParameterTypes>
  <ParameterType Id="PT-Num" Name="num"><TypeNumber maxInclusive="255" minInclusive="0" SizeInBit="8" Type="unsignedInt" /></ParameterType>
  <ParameterType Id="PT-Enum" Name="enum"><TypeRestriction Base="Value" SizeInBit="8">
    <Enumeration Id="PT-Enum_EN-0" Text="Off" Value="0" DisplayOrder="0" />
    <Enumeration Id="PT-Enum_EN-1" Text="On" Value="1" DisplayOrder="1" />
  </TypeRestriction></ParameterType>
  <ParameterType Id="PT-Text" Name="text"><TypeText /></ParameterType>
</ParameterTypes>
<Parameters>
  <Parameter Id="P-1" Name="Delay" Text="Delay" ParameterType="PT-Num" Access="ReadWrite" Value="5" />
  <Parameter Id="P-2" Name="Mode" Text="Mode" ParameterType="PT-Enum" Access="ReadWrite" Value="0" />
  <Parameter Id="P-3" Name="Label" Text="Label" ParameterType="PT-Text" Access="ReadWrite" Value="hi" />
</Parameters>
<ParameterRefs>
  <ParameterRef Id="PR-1" RefId="P-1" DisplayOrder="30" Tag="1" />
  <ParameterRef Id="PR-2" RefId="P-2" DisplayOrder="10" Tag="2" Text="On (override)" />
  <ParameterRef Id="PR-3" RefId="P-3" DisplayOrder="20" Tag="3" />
</ParameterRefs>
</Static></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

    fn parameter_db() -> (tempfile::TempDir, Connection) {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
        ingest_program(
            &conn,
            "sha-p2",
            "M-006A/A2.xml",
            PARAMETER_PROGRAM.as_bytes(),
        )
        .unwrap();
        (dir, conn)
    }

    #[test]
    fn parameter_views_returns_three_views_in_display_order_with_enum_options_only_on_the_restriction(
    ) {
        let (_dir, conn) = parameter_db();
        let views = parameter_views(&conn, "A-2", None).unwrap();
        assert_eq!(
            views.iter().map(|v| v.id.as_str()).collect::<Vec<_>>(),
            vec!["PR-2", "PR-3", "PR-1"],
            "sorted by display_order (10, 20, 30), not by id"
        );

        let pr2 = &views[0];
        assert_eq!(pr2.kind, "Restriction");
        assert_eq!(pr2.display_order, Some(10));
        assert_eq!(
            pr2.enum_options,
            vec![
                ("0".to_string(), Some("Off".to_string())),
                ("1".to_string(), Some("On".to_string())),
            ]
        );

        let pr3 = &views[1];
        assert_eq!(pr3.kind, "Text");
        assert_eq!(pr3.display_order, Some(20));
        assert!(pr3.enum_options.is_empty());
        // `<TypeText />` here carries no `SizeInBit` at all — `None`, not a
        // fabricated `0` (T18 slice 5's own "package genuinely omits it"
        // case, parallel to `DisplayOrder`'s reasoning two lines up).
        assert_eq!(pr3.size_in_bit, None);

        let pr1 = &views[2];
        assert_eq!(pr1.kind, "Number");
        assert_eq!(pr1.display_order, Some(30));
        assert!(pr1.enum_options.is_empty());
        assert_eq!(pr1.min_inclusive.as_deref(), Some("0"));
        assert_eq!(pr1.max_inclusive.as_deref(), Some("255"));
    }

    /// T18 slice 5: `Float`'s bounds and `Text`'s length cap reach
    /// `ParameterView` through the same generic columns `Number` already
    /// used — a second, standalone program so this doesn't have to graft
    /// onto `PARAMETER_PROGRAM`'s fixed three-row shape the tests above
    /// depend on.
    const PARAMETER_PROGRAM_FLOAT_AND_SIZED_TEXT: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-006A">
<ApplicationPrograms><ApplicationProgram Id="A-3" Name="P" ApplicationNumber="3"
  ApplicationVersion="22" MaskVersion="MV-0701"><Static>
<ParameterTypes>
  <ParameterType Id="PT-Float" Name="temp"><TypeFloat Encoding="DPT 9" minInclusive="-100" maxInclusive="200" /></ParameterType>
  <ParameterType Id="PT-SizedText" Name="label"><TypeText SizeInBit="240" /></ParameterType>
</ParameterTypes>
<Parameters>
  <Parameter Id="P-1" Name="Temp" Text="Temp" ParameterType="PT-Float" Access="ReadWrite" Value="0" />
  <Parameter Id="P-2" Name="Label" Text="Label" ParameterType="PT-SizedText" Access="ReadWrite" Value="hi" />
</Parameters>
<ParameterRefs>
  <ParameterRef Id="PR-1" RefId="P-1" DisplayOrder="1" Tag="1" />
  <ParameterRef Id="PR-2" RefId="P-2" DisplayOrder="2" Tag="2" />
</ParameterRefs>
</Static></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

    #[test]
    fn parameter_views_surfaces_float_bounds_and_text_size() {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
        ingest_program(
            &conn,
            "sha-p3",
            "M-006A/A3.xml",
            PARAMETER_PROGRAM_FLOAT_AND_SIZED_TEXT.as_bytes(),
        )
        .unwrap();
        let views = parameter_views(&conn, "A-3", None).unwrap();

        let temp = &views[0];
        assert_eq!(temp.kind, "Float");
        assert_eq!(temp.min_inclusive.as_deref(), Some("-100"));
        assert_eq!(temp.max_inclusive.as_deref(), Some("200"));

        let label = &views[1];
        assert_eq!(label.kind, "Text");
        assert_eq!(label.size_in_bit, Some(240));
    }

    #[test]
    fn parameter_ref_ids_returns_the_declared_id_set() {
        let (_dir, conn) = parameter_db();
        let ids = parameter_ref_ids(&conn, "A-2").unwrap();
        assert_eq!(
            ids,
            HashSet::from(["PR-1".to_string(), "PR-2".to_string(), "PR-3".to_string(),])
        );
    }

    /// A `ParameterRef` with no `DisplayOrder` attribute at all — the case
    /// measured on the real corpus (543/543 rows for `prod3`'s program
    /// `M-0083_A-0317-31-7DC6`) — must report `None`, not a fabricated `0`.
    /// `0` would be indistinguishable from a package that genuinely declared
    /// position zero, which is exactly the information loss `CLAUDE.md`
    /// rules out.
    const PARAMETER_PROGRAM_NO_DISPLAY_ORDER: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-006A">
<ApplicationPrograms><ApplicationProgram Id="A-3" Name="P" ApplicationNumber="3"
  ApplicationVersion="22" MaskVersion="MV-0701"><Static>
<ParameterTypes>
  <ParameterType Id="PT-Num" Name="num"><TypeNumber maxInclusive="255" minInclusive="0" SizeInBit="8" Type="unsignedInt" /></ParameterType>
</ParameterTypes>
<Parameters>
  <Parameter Id="P-1" Name="Delay" Text="Delay" ParameterType="PT-Num" Access="ReadWrite" Value="5" />
</Parameters>
<ParameterRefs>
  <ParameterRef Id="PR-1" RefId="P-1" Tag="1" />
</ParameterRefs>
</Static></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

    #[test]
    fn parameter_views_reports_none_when_display_order_is_not_declared() {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
        ingest_program(
            &conn,
            "sha-p3",
            "M-006A/A3.xml",
            PARAMETER_PROGRAM_NO_DISPLAY_ORDER.as_bytes(),
        )
        .unwrap();
        let views = parameter_views(&conn, "A-3", None).unwrap();
        assert_eq!(views.len(), 1);
        assert_eq!(
            views[0].display_order, None,
            "an undeclared DisplayOrder must stay None, not collapse into Some(0)"
        );
    }

    /// Two `ParameterRef`s that both lack `DisplayOrder` tie on the primary
    /// sort key; the `pr.rowid` tiebreak must then return them in
    /// declaration order, not some other order SQLite's sorter happens to
    /// pick for a tie. Declared as `PR-Z` then `PR-A` deliberately — id
    /// order would put `PR-A` first, so this only passes if the tiebreak is
    /// really `rowid`, not a hidden secondary sort on `id`.
    const PARAMETER_PROGRAM_TIEBREAK: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-006A">
<ApplicationPrograms><ApplicationProgram Id="A-4" Name="P" ApplicationNumber="4"
  ApplicationVersion="22" MaskVersion="MV-0701"><Static>
<ParameterTypes>
  <ParameterType Id="PT-Num" Name="num"><TypeNumber maxInclusive="255" minInclusive="0" SizeInBit="8" Type="unsignedInt" /></ParameterType>
</ParameterTypes>
<Parameters>
  <Parameter Id="P-1" Name="First" Text="First" ParameterType="PT-Num" Access="ReadWrite" Value="1" />
  <Parameter Id="P-2" Name="Second" Text="Second" ParameterType="PT-Num" Access="ReadWrite" Value="2" />
</Parameters>
<ParameterRefs>
  <ParameterRef Id="PR-Z" RefId="P-1" Tag="1" />
  <ParameterRef Id="PR-A" RefId="P-2" Tag="2" />
</ParameterRefs>
</Static></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

    #[test]
    fn parameter_views_ties_break_by_declaration_order_when_display_order_is_absent_for_all() {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
        ingest_program(
            &conn,
            "sha-p4",
            "M-006A/A4.xml",
            PARAMETER_PROGRAM_TIEBREAK.as_bytes(),
        )
        .unwrap();
        let views = parameter_views(&conn, "A-4", None).unwrap();
        assert_eq!(
            views.iter().map(|v| v.id.as_str()).collect::<Vec<_>>(),
            vec!["PR-Z", "PR-A"],
            "rowid tiebreak preserves declaration order, not ascending id order"
        );
        assert!(
            views.iter().all(|v| v.display_order.is_none()),
            "both rows in this fixture omit DisplayOrder"
        );
    }

    /// Mirrors `com_object_view`'s own `pick()` tests above: a `ParameterRef`
    /// with no `Text` of its own falls back to its `Parameter`'s, reporting
    /// `ValueLayer::Program`; one with an override reports `ProgramRef`.
    #[test]
    fn parameter_views_reports_pick_layer_the_same_way_com_object_view_does() {
        let (_dir, conn) = parameter_db();
        let views = parameter_views(&conn, "A-2", None).unwrap();
        let pr1 = views.iter().find(|v| v.id == "PR-1").unwrap();
        assert_eq!(pr1.text.as_deref(), Some("Delay"));
        assert_eq!(pr1.text_layer, ValueLayer::Program);

        let pr2 = views.iter().find(|v| v.id == "PR-2").unwrap();
        assert_eq!(pr2.text.as_deref(), Some("On (override)"));
        assert_eq!(pr2.text_layer, ValueLayer::ProgramRef);
    }

    // -----------------------------------------------------------------
    // The translation overlay and the language queries (T26 Task 1).
    // -----------------------------------------------------------------

    /// Four `ParameterRef`s exercising every layer/translation combination
    /// this task's tests need, plus a `Restriction` type with two
    /// enumeration options:
    ///
    /// - `PR-1` (`RefId="P-1"`) has no `Text` of its own; `P-1/@Text` has a
    ///   `de-DE` translation.
    /// - `PR-2` (`RefId="P-2"`) is the `Restriction` parameter; its type's
    ///   enumeration `PT-Enum_EN-0` has a `de-DE` `Text` translation,
    ///   `PT-Enum_EN-1` has both a legitimate `de-DE` `Text` translation and
    ///   an illegitimate `de-DE` `Value` translation that must be ignored.
    /// - `PR-3` (`RefId="P-3"`) has no translation row at all in `de-DE`.
    /// - `PR-4` (`RefId="P-4"`) has no `Text` of its own; `P-4/@Text` is
    ///   never translated, but `PR-4` itself (the `ParameterRef` layer) has
    ///   a `de-DE` `Text` translation.
    const PARAMETER_PROGRAM_TRANSLATED: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-006A">
<ApplicationPrograms><ApplicationProgram Id="A-5" Name="P" ApplicationNumber="5"
  ApplicationVersion="22" MaskVersion="MV-0701"><Static>
<ParameterTypes>
  <ParameterType Id="PT-Num" Name="num"><TypeNumber maxInclusive="255" minInclusive="0" SizeInBit="8" Type="unsignedInt" /></ParameterType>
  <ParameterType Id="PT-Enum" Name="enum"><TypeRestriction Base="Value" SizeInBit="8">
    <Enumeration Id="PT-Enum_EN-0" Text="Off" Value="0" DisplayOrder="0" />
    <Enumeration Id="PT-Enum_EN-1" Text="On" Value="1" DisplayOrder="1" />
  </TypeRestriction></ParameterType>
</ParameterTypes>
<Parameters>
  <Parameter Id="P-1" Name="Delay" Text="Delay" ParameterType="PT-Num" Access="ReadWrite" Value="5" />
  <Parameter Id="P-2" Name="Mode" Text="Mode" ParameterType="PT-Enum" Access="ReadWrite" Value="0" />
  <Parameter Id="P-3" Name="Untranslated" Text="Untranslated" ParameterType="PT-Num" Access="ReadWrite" Value="1" />
  <Parameter Id="P-4" Name="RefWins" Text="RefWins English" ParameterType="PT-Num" Access="ReadWrite" Value="2" />
</Parameters>
<ParameterRefs>
  <ParameterRef Id="PR-1" RefId="P-1" DisplayOrder="10" Tag="1" />
  <ParameterRef Id="PR-2" RefId="P-2" DisplayOrder="20" Tag="2" />
  <ParameterRef Id="PR-3" RefId="P-3" DisplayOrder="30" Tag="3" />
  <ParameterRef Id="PR-4" RefId="P-4" DisplayOrder="40" Tag="4" />
</ParameterRefs>
</Static>
<Languages>
  <Language Identifier="de-DE">
    <TranslationUnit RefId="A-5">
      <TranslationElement RefId="P-1">
        <Translation AttributeName="Text" Text="Verzoegerung" />
      </TranslationElement>
      <TranslationElement RefId="PR-4">
        <Translation AttributeName="Text" Text="RefWins Deutsch" />
      </TranslationElement>
      <TranslationElement RefId="PT-Enum_EN-0">
        <Translation AttributeName="Text" Text="Aus" />
      </TranslationElement>
      <TranslationElement RefId="PT-Enum_EN-1">
        <Translation AttributeName="Text" Text="An" />
        <Translation AttributeName="Value" Text="99" />
      </TranslationElement>
    </TranslationUnit>
  </Language>
</Languages>
</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

    fn translated_parameter_db() -> (tempfile::TempDir, Connection) {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
        ingest_program(
            &conn,
            "sha-p5",
            "M-006A/A5.xml",
            PARAMETER_PROGRAM_TRANSLATED.as_bytes(),
        )
        .unwrap();
        (dir, conn)
    }

    #[test]
    fn parameter_views_without_a_language_returns_the_untranslated_text() {
        let (_dir, conn) = translated_parameter_db();
        let views = parameter_views(&conn, "A-5", None).unwrap();

        let pr1 = views.iter().find(|v| v.id == "PR-1").unwrap();
        assert_eq!(pr1.text.as_deref(), Some("Delay"));
        let pr2 = views.iter().find(|v| v.id == "PR-2").unwrap();
        assert_eq!(
            pr2.enum_options,
            vec![
                ("0".to_string(), Some("Off".to_string())),
                ("1".to_string(), Some("On".to_string())),
            ],
            "no language requested: enum labels stay the package's own"
        );
    }

    #[test]
    fn parameter_views_with_a_language_returns_the_translated_text() {
        let (_dir, conn) = translated_parameter_db();
        let views = parameter_views(&conn, "A-5", Some("de-DE")).unwrap();

        let pr1 = views.iter().find(|v| v.id == "PR-1").unwrap();
        assert_eq!(pr1.text.as_deref(), Some("Verzoegerung"));
        assert_eq!(
            pr1.text_layer,
            ValueLayer::Program,
            "the translation overlaid Parameter/@Text, so it still reports the Program layer"
        );
    }

    #[test]
    fn parameter_views_matches_a_short_locale_request_by_prefix() {
        // R2, applied to `translation_overlay` (shared by `parameter_views`
        // and `com_object_views`): the fixture only stores `de-DE`, and a
        // caller requesting the bare `de` must still get it.
        let (_dir, conn) = translated_parameter_db();
        let views = parameter_views(&conn, "A-5", Some("de")).unwrap();
        let pr1 = views.iter().find(|v| v.id == "PR-1").unwrap();
        assert_eq!(pr1.text.as_deref(), Some("Verzoegerung"));
    }

    #[test]
    fn a_parameter_without_a_row_in_that_language_keeps_its_own_text() {
        let (_dir, conn) = translated_parameter_db();
        let views = parameter_views(&conn, "A-5", Some("de-DE")).unwrap();

        let pr3 = views.iter().find(|v| v.id == "PR-3").unwrap();
        assert_eq!(
            pr3.text.as_deref(),
            Some("Untranslated"),
            "PR-3/P-3 has no de-DE translation row; it must keep its own text \
             in the very call that translates PR-1's neighbour"
        );
    }

    #[test]
    fn a_translated_parameter_ref_text_still_beats_an_untranslated_parameter_text() {
        let (_dir, conn) = translated_parameter_db();
        let views = parameter_views(&conn, "A-5", Some("de-DE")).unwrap();

        let pr4 = views.iter().find(|v| v.id == "PR-4").unwrap();
        assert_eq!(
            pr4.text.as_deref(),
            Some("RefWins Deutsch"),
            "P-4/@Text ('RefWins English') is never translated; PR-4's own \
             translated text must still win"
        );
        assert_eq!(
            pr4.text_layer,
            ValueLayer::ProgramRef,
            "the translation overlaid ParameterRef/@Text, so it still reports \
             the ProgramRef layer — translation is a language dimension, not \
             a layer dimension"
        );
    }

    #[test]
    fn enum_option_labels_are_translated() {
        let (_dir, conn) = translated_parameter_db();
        let views = parameter_views(&conn, "A-5", Some("de-DE")).unwrap();

        let pr2 = views.iter().find(|v| v.id == "PR-2").unwrap();
        assert_eq!(
            pr2.enum_options,
            vec![
                ("0".to_string(), Some("Aus".to_string())),
                ("1".to_string(), Some("An".to_string())),
            ]
        );
    }

    #[test]
    fn a_value_translation_never_changes_a_stored_value() {
        let (_dir, conn) = translated_parameter_db();
        let views = parameter_views(&conn, "A-5", Some("de-DE")).unwrap();

        let pr2 = views.iter().find(|v| v.id == "PR-2").unwrap();
        let on_option = pr2
            .enum_options
            .iter()
            .find(|(value, _)| value == "1")
            .unwrap();
        assert_eq!(
            on_option,
            &("1".to_string(), Some("An".to_string())),
            "PT-Enum_EN-1 carries a de-DE Value=99 translation row alongside \
             its legitimate Text=An row; the value must stay the package's \
             own '1', never '99', while the sibling Text translation still \
             applies"
        );
    }

    /// Three languages on one program with counts 3, 2 and 2 — the tie
    /// between `de-DE` and `fr-FR` only passes if the ordering really is
    /// `COUNT(*) DESC, language ASC` and not, say, insertion order.
    const TRANSLATION_LANGUAGES_PROGRAM: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-006A">
<ApplicationPrograms><ApplicationProgram Id="A-6" Name="P" ApplicationNumber="6"
  ApplicationVersion="1" MaskVersion="MV-0701"><Static>
<ComObjectTable>
  <ComObject Id="A-6_O-0" Number="0" Text="X" ObjectSize="1 Bit" />
</ComObjectTable>
</Static>
<Languages>
  <Language Identifier="en-US"><TranslationUnit RefId="A-6"><TranslationElement RefId="A-6_O-0">
    <Translation AttributeName="Text" Text="Output" />
    <Translation AttributeName="FunctionText" Text="Switch" />
    <Translation AttributeName="VisibleDescription" Text="Desc" />
  </TranslationElement></TranslationUnit></Language>
  <Language Identifier="de-DE"><TranslationUnit RefId="A-6"><TranslationElement RefId="A-6_O-0">
    <Translation AttributeName="Text" Text="Ausgang" />
    <Translation AttributeName="FunctionText" Text="Schalten" />
  </TranslationElement></TranslationUnit></Language>
  <Language Identifier="fr-FR"><TranslationUnit RefId="A-6"><TranslationElement RefId="A-6_O-0">
    <Translation AttributeName="Text" Text="Sortie" />
    <Translation AttributeName="FunctionText" Text="Commuter" />
  </TranslationElement></TranslationUnit></Language>
</Languages>
</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

    /// A second program declaring only `it-IT` — used to prove
    /// `program_translation_languages` narrows to one program's set rather
    /// than answering for the whole database (the real corpus has programs
    /// declaring 2 and 10 languages; this fixture mirrors that shape with
    /// 3 and 1).
    const TRANSLATION_LANGUAGES_PROGRAM_IT: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-006A">
<ApplicationPrograms><ApplicationProgram Id="A-8" Name="P" ApplicationNumber="8"
  ApplicationVersion="1" MaskVersion="MV-0701"><Static>
<ComObjectTable>
  <ComObject Id="A-8_O-0" Number="0" Text="Y" ObjectSize="1 Bit" />
</ComObjectTable>
</Static>
<Languages>
  <Language Identifier="it-IT"><TranslationUnit RefId="A-8"><TranslationElement RefId="A-8_O-0">
    <Translation AttributeName="Text" Text="Uscita" />
    <Translation AttributeName="FunctionText" Text="Commutare" />
  </TranslationElement></TranslationUnit></Language>
</Languages>
</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

    #[test]
    fn translation_languages_orders_by_row_count_then_identifier() {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
        ingest_program(
            &conn,
            "sha-a6",
            "M-006A/A6.xml",
            TRANSLATION_LANGUAGES_PROGRAM.as_bytes(),
        )
        .unwrap();

        let langs = translation_languages(&conn).unwrap();
        assert_eq!(
            langs,
            vec![
                TranslationLanguage {
                    language: "en-US".to_string(),
                    rows: 3
                },
                TranslationLanguage {
                    language: "de-DE".to_string(),
                    rows: 2
                },
                TranslationLanguage {
                    language: "fr-FR".to_string(),
                    rows: 2
                },
            ],
            "en-US has more rows and sorts first; de-DE and fr-FR tie on \
             count and must then sort by identifier"
        );
    }

    #[test]
    fn program_translation_languages_returns_only_that_programs_set() {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
        ingest_program(
            &conn,
            "sha-a6",
            "M-006A/A6.xml",
            TRANSLATION_LANGUAGES_PROGRAM.as_bytes(),
        )
        .unwrap();
        ingest_program(
            &conn,
            "sha-a8",
            "M-006A/A8.xml",
            TRANSLATION_LANGUAGES_PROGRAM_IT.as_bytes(),
        )
        .unwrap();

        let a6 = program_translation_languages(&conn, "A-6").unwrap();
        assert_eq!(
            a6,
            vec![
                TranslationLanguage {
                    language: "en-US".to_string(),
                    rows: 3
                },
                TranslationLanguage {
                    language: "de-DE".to_string(),
                    rows: 2
                },
                TranslationLanguage {
                    language: "fr-FR".to_string(),
                    rows: 2
                },
            ],
            "A-6's own set, unaffected by A-8 sharing the same database"
        );

        let a8 = program_translation_languages(&conn, "A-8").unwrap();
        assert_eq!(
            a8,
            vec![TranslationLanguage {
                language: "it-IT".to_string(),
                rows: 2
            }],
            "A-8 declares only it-IT; a database-wide answer would wrongly \
             include A-6's languages too"
        );
    }

    // --- R2: best_matching_language, the one place locale-prefix matching
    // lives, with its own dedicated unit tests per the brief's requirement.

    #[test]
    fn best_matching_language_prefers_an_exact_hit_over_a_prefix_hit() {
        assert_eq!(
            best_matching_language("de-DE", &["de-DE", "de-AT", "en-US"]),
            Some("de-DE"),
            "an exact match must win even though de-AT also prefix-matches \
             a hypothetically shorter request"
        );
    }

    #[test]
    fn best_matching_language_matches_a_short_request_against_a_longer_stored_one() {
        assert_eq!(
            best_matching_language("de", &["de-DE", "en-US"]),
            Some("de-DE")
        );
    }

    #[test]
    fn best_matching_language_breaks_a_two_variant_tie_by_the_lowest_identifier() {
        // `de-AT` < `de-DE` in plain byte order — the documented,
        // deterministic (if arbitrary) tiebreak.
        assert_eq!(
            best_matching_language("de", &["de-DE", "de-AT"]),
            Some("de-AT")
        );
        assert_eq!(
            best_matching_language("de", &["de-AT", "de-DE"]),
            Some("de-AT"),
            "the tiebreak must not depend on the candidates' input order"
        );
    }

    #[test]
    fn best_matching_language_returns_none_on_no_hit() {
        assert_eq!(best_matching_language("fr", &["de-DE", "en-US"]), None);
        assert_eq!(best_matching_language("de", &[]), None);
    }

    #[test]
    fn best_matching_language_returns_none_on_an_empty_or_garbage_request() {
        assert_eq!(best_matching_language("", &["de-DE", "en-US"]), None);
        assert_eq!(
            best_matching_language("!!!not-a-language!!!", &["de-DE", "en-US"]),
            None
        );
    }

    #[test]
    fn best_matching_language_does_not_match_a_request_with_no_separator() {
        // `de` must not match a hypothetical `deX` — R2 requires the
        // separating `-`, not a bare string-prefix test.
        assert_eq!(
            best_matching_language("de", &["deX", "de-DE"]),
            Some("de-DE")
        );
        assert_eq!(best_matching_language("de", &["deX"]), None);
    }

    // --- R1: datapoint_types/datapoint_type, the new Master-scope reader.

    const MASTER_WITH_DATAPOINT_TYPES: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11">
<MasterData>
<DatapointTypes>
  <DatapointType Id="DPT-1" Number="1" Name="1-bit">
    <DatapointSubtypes>
      <DatapointSubtype Id="DPST-1-1" Number="1" Name="switch" Text="Switch" />
    </DatapointSubtypes>
  </DatapointType>
</DatapointTypes>
<FunctionTypes>
  <FunctionType Id="FT-1" Number="1" Text="Switch" Status="Certified">
    <FunctionPoint Id="FP-1_DR-1" Text="Switch" DatapointType="DPST-1-1" Role="Control" Characteristics="W" />
  </FunctionType>
</FunctionTypes>
<SpaceUsages>
  <SpaceUsage Id="SU-1" Number="1" Text="Office" />
</SpaceUsages>
</MasterData>
<Languages>
  <Language Identifier="de-DE">
    <TranslationUnit RefId="DPST-1-1">
      <TranslationElement RefId="DPST-1-1">
        <Translation AttributeName="Text" Text="Schalten" />
      </TranslationElement>
    </TranslationUnit>
    <TranslationUnit RefId="FT-1">
      <TranslationElement RefId="FT-1">
        <Translation AttributeName="Text" Text="Schalten" />
      </TranslationElement>
    </TranslationUnit>
    <TranslationUnit RefId="SU-1">
      <TranslationElement RefId="SU-1">
        <Translation AttributeName="Text" Text="Büro" />
      </TranslationElement>
    </TranslationUnit>
  </Language>
</Languages>
</KNX>"#;

    fn master_db() -> (tempfile::TempDir, Connection) {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
        crate::ingest_master_data(&conn, MASTER_WITH_DATAPOINT_TYPES.as_bytes()).unwrap();
        (dir, conn)
    }

    #[test]
    fn datapoint_types_without_a_language_returns_the_stored_text_unchanged() {
        let (_dir, conn) = master_db();
        let rows = datapoint_types(&conn, None).unwrap();
        let dpst = rows.iter().find(|r| r.id == "DPST-1-1").unwrap();
        assert_eq!(dpst.main, 1);
        assert_eq!(dpst.sub, Some(1));
        assert_eq!(dpst.name.as_deref(), Some("switch"));
        assert_eq!(dpst.text.as_deref(), Some("Switch"));
    }

    #[test]
    fn datapoint_types_with_a_language_overlays_the_translated_text() {
        let (_dir, conn) = master_db();
        let rows = datapoint_types(&conn, Some("de-DE")).unwrap();
        let dpst = rows.iter().find(|r| r.id == "DPST-1-1").unwrap();
        assert_eq!(dpst.text.as_deref(), Some("Schalten"));
        // The untranslated `name` is never touched by the overlay.
        assert_eq!(dpst.name.as_deref(), Some("switch"));
    }

    #[test]
    fn datapoint_types_matches_a_short_locale_request_by_prefix() {
        let (_dir, conn) = master_db();
        let rows = datapoint_types(&conn, Some("de")).unwrap();
        let dpst = rows.iter().find(|r| r.id == "DPST-1-1").unwrap();
        assert_eq!(dpst.text.as_deref(), Some("Schalten"));
    }

    #[test]
    fn datapoint_types_with_an_unmatched_language_keeps_the_stored_text() {
        let (_dir, conn) = master_db();
        let rows = datapoint_types(&conn, Some("fr-FR")).unwrap();
        let dpst = rows.iter().find(|r| r.id == "DPST-1-1").unwrap();
        assert_eq!(
            dpst.text.as_deref(),
            Some("Switch"),
            "a missing translation is never an error and never an empty string"
        );
    }

    #[test]
    fn datapoint_type_looks_up_a_single_row_by_id() {
        let (_dir, conn) = master_db();
        assert_eq!(
            datapoint_type(&conn, "DPST-1-1", Some("de-DE"))
                .unwrap()
                .unwrap()
                .text
                .as_deref(),
            Some("Schalten")
        );
        assert!(datapoint_type(&conn, "nope", None).unwrap().is_none());
    }

    #[test]
    fn function_types_overlay_their_translated_text() {
        let (_dir, conn) = master_db();
        let rows = function_types(&conn, Some("de-DE")).unwrap();
        let ft = rows.iter().find(|r| r.id == "FT-1").unwrap();
        assert_eq!(ft.number, Some(1));
        assert_eq!(ft.status.as_deref(), Some("Certified"));
        assert_eq!(ft.text.as_deref(), Some("Schalten"));
    }

    #[test]
    fn function_types_without_a_language_returns_the_stored_text_unchanged() {
        let (_dir, conn) = master_db();
        let rows = function_types(&conn, None).unwrap();
        let ft = rows.iter().find(|r| r.id == "FT-1").unwrap();
        assert_eq!(ft.text.as_deref(), Some("Switch"));
    }

    #[test]
    fn function_type_looks_up_a_single_row_by_id() {
        let (_dir, conn) = master_db();
        assert_eq!(
            function_type(&conn, "FT-1", Some("de-DE"))
                .unwrap()
                .unwrap()
                .text
                .as_deref(),
            Some("Schalten")
        );
        assert!(function_type(&conn, "nope", None).unwrap().is_none());
    }

    #[test]
    fn function_points_are_scoped_to_their_owning_function_type() {
        let (_dir, conn) = master_db();
        let rows = function_points(&conn, "FT-1", Some("de-DE")).unwrap();
        assert_eq!(rows.len(), 1);
        let point = &rows[0];
        assert_eq!(point.id, "FP-1_DR-1");
        assert_eq!(point.datapoint_type.as_deref(), Some("DPST-1-1"));
        assert_eq!(point.role.as_deref(), Some("Control"));
        // No `Master`-scope translation was planted for `FP-1_DR-1` itself
        // in this fixture, so the stored text survives untouched.
        assert_eq!(point.text.as_deref(), Some("Switch"));

        assert!(function_points(&conn, "FT-does-not-exist", None)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn space_usages_overlay_their_translated_text() {
        let (_dir, conn) = master_db();
        let rows = space_usages(&conn, Some("de-DE")).unwrap();
        let su = rows.iter().find(|r| r.id == "SU-1").unwrap();
        assert_eq!(su.number, Some(1));
        assert_eq!(su.text.as_deref(), Some("Büro"));
    }

    #[test]
    fn space_usages_with_an_unmatched_language_keeps_the_stored_text() {
        let (_dir, conn) = master_db();
        let rows = space_usages(&conn, Some("fr-FR")).unwrap();
        let su = rows.iter().find(|r| r.id == "SU-1").unwrap();
        assert_eq!(su.text.as_deref(), Some("Office"));
    }

    #[test]
    fn space_usage_looks_up_a_single_row_by_id() {
        let (_dir, conn) = master_db();
        assert_eq!(
            space_usage(&conn, "SU-1", Some("de-DE"))
                .unwrap()
                .unwrap()
                .text
                .as_deref(),
            Some("Büro")
        );
        assert!(space_usage(&conn, "nope", None).unwrap().is_none());
    }
}
