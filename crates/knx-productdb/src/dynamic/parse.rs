//! Reads every `Dynamic` tree in an `ApplicationProgram` file — the
//! program's own (`module_def_id = ''`) and each `ModuleDef`'s
//! (`module_def_id` = that `ModuleDef`'s `@Id`) — and stores one
//! `dynamic_node` row per element, in document order, per design D2.
//!
//! This does not interpret the tree. `@test` is stored verbatim and
//! untouched; `Module`/`ModuleDef` references are stored, not followed.
//! `kind` is always the element's own local name (design D3) — there is no
//! recognized/unrecognized split at this layer, because storage does not
//! need to know what an element means, only what it is. That split belongs
//! to the evaluator this module does not yet have.

use quick_xml::events::Event;
use quick_xml::Reader;
use rusqlite::{params, Connection, OptionalExtension};

use crate::report::{UnknownCollector, UnknownConstruct};
use crate::xml::{attrs, local_name, Attrs};
use crate::ProductDbError;

/// What `parse_dynamic_trees` did not recognize. Merged by the caller into
/// whatever the `Static` pass over the same bytes already collected, then
/// written once via `report::insert_unknown` (the same pattern every other
/// parser in this crate follows).
pub struct DynamicIngest {
    pub unknown: Vec<UnknownConstruct>,
}

/// The attribute names this build recognizes for one element `kind`
/// (design D4's table, verbatim), and which `dynamic_node` column, if any,
/// each maps to. An attribute recognized here but with no target column
/// (`ParameterBlock/@Name`, `Assign`'s three attributes, …) still lands in
/// `extra` — nothing this build reads is ever dropped, "modelled" only
/// controls whether it is also flagged in `ingest_unknown`.
struct ElementSpec {
    known: &'static [&'static str],
    id_attr: Option<&'static str>,
    ref_attrs: &'static [&'static str],
    test_attr: Option<&'static str>,
    default_attr: Option<&'static str>,
    text_attr: Option<&'static str>,
    value_attr: Option<&'static str>,
}

const UNMODELLED: ElementSpec = ElementSpec {
    known: &[],
    id_attr: None,
    ref_attrs: &[],
    test_attr: None,
    default_attr: None,
    text_attr: None,
    value_attr: None,
};

/// Design D4's table. `choose`'s `@ParamRefId`, `Channel`/`ParameterBlock`/
/// `ParameterRefRef`/`ComObjectRefRef`/`Module`'s `@RefId` — the two
/// spellings `spec_for` actually maps — resolve into the one `ref_id`
/// column below. (D2's schema comment for this column also lists
/// `@ParameterRefId`; nothing in the researched corpus or this table uses
/// that spelling, so no kind below maps it.)
fn spec_for(kind: &str) -> ElementSpec {
    match kind {
        "choose" => ElementSpec {
            known: &["ParamRefId"],
            ref_attrs: &["ParamRefId"],
            ..UNMODELLED
        },
        "when" => ElementSpec {
            known: &["test", "default", "Id"],
            id_attr: Some("Id"),
            test_attr: Some("test"),
            default_attr: Some("default"),
            ..UNMODELLED
        },
        "Channel" | "ParameterBlock" => ElementSpec {
            known: &["Id", "RefId", "Text", "Name"],
            id_attr: Some("Id"),
            ref_attrs: &["RefId"],
            text_attr: Some("Text"),
            ..UNMODELLED
        },
        "ParameterRefRef" | "ComObjectRefRef" => ElementSpec {
            known: &["RefId"],
            ref_attrs: &["RefId"],
            ..UNMODELLED
        },
        "ParameterSeparator" => ElementSpec {
            known: &["Id", "Text", "UIHint"],
            id_attr: Some("Id"),
            text_attr: Some("Text"),
            ..UNMODELLED
        },
        "Module" => ElementSpec {
            known: &["Id", "RefId"],
            id_attr: Some("Id"),
            ref_attrs: &["RefId"],
            ..UNMODELLED
        },
        // Design D47 (goal-completion task 12): a `Module`'s argument
        // bindings. `@RefId` names the `ModuleDef/Arguments/Argument` being
        // bound, `@Value` is what it is bound to, and `@Id` — present on
        // `TextArg` in the researched corpus, absent on `NumericArg` — is
        // the binding element's own identity. Both spellings were reaching
        // `UNMODELLED` before v10, which left `@Value` in `extra`, a store
        // documented as not re-parseable. `Assign`'s own `@Value` keeps
        // landing in `extra`: `Assign` is still inert in the evaluator, so
        // promoting its value to a column would claim an interpretation
        // that does not exist.
        "NumericArg" | "TextArg" => ElementSpec {
            known: &["Id", "RefId", "Value"],
            id_attr: Some("Id"),
            ref_attrs: &["RefId"],
            value_attr: Some("Value"),
            ..UNMODELLED
        },
        "Assign" => ElementSpec {
            known: &["TargetParamRefRef", "Value", "SourceParamRefRef"],
            ..UNMODELLED
        },
        _ => UNMODELLED,
    }
}

/// One open element's node id and the position its next child will get.
type Frame = (i64, i64);

/// Reads every `Dynamic` tree out of one `ApplicationProgram` file's bytes
/// and stores it. Designed to run standalone over a single blob — the
/// caller supplies only the connection, the bytes and their content hash —
/// so it can be driven both from ingest (a second pass over the bytes the
/// `Static` parser just read, in the same transaction) and, unmodified,
/// from a migration replaying stored `source_file` blobs (design D5).
///
/// A program already holding `dynamic_node` rows is left alone: this is
/// what makes the function idempotent under both a forced re-parse of
/// already-ingested bytes (`package::install_package`'s `parse_existing`
/// path) and a migration backfill that runs after this table already has
/// some programs' trees in it. A program that lost an id conflict against
/// an earlier, different file (the same rule `parse::program::ingest_program`
/// applies to the `Static` tables) is skipped for the same reason: the
/// first file's data wins everywhere, not just in the `Static` tables.
pub fn parse_dynamic_trees(
    conn: &Connection,
    source_sha256: &str,
    source_path: &str,
    bytes: &[u8],
) -> Result<DynamicIngest, ProductDbError> {
    let mut reader = Reader::from_reader(bytes);
    let mut buf = Vec::new();
    let mut unknown = UnknownCollector::default();

    let mut program_id = String::new();
    let mut module_def_id = String::new();
    let mut skip_program = false;

    // Non-empty exactly while inside an open `<Dynamic>` subtree, root
    // included: one frame per currently-open ancestor.
    let mut stack: Vec<Frame> = Vec::new();
    let mut next_node_id: i64 = 0;
    // Document position of the next `ModuleDef/Arguments/Argument`, reset
    // at every `ModuleDef` the same way `next_node_id` resets at every
    // `Dynamic` root.
    let mut next_argument_position: i64 = 0;

    loop {
        buf.clear();
        let event = reader
            .read_event_into(&mut buf)
            .map_err(|e| ProductDbError::Xml {
                source_path: source_path.to_string(),
                cause: e.to_string(),
            })?;
        match event {
            Event::Eof => break,
            Event::Start(e) => {
                let name = local_name(&e);
                let a = attrs(&e, source_path)?;
                handle_start_or_empty(
                    conn,
                    &name,
                    &a,
                    true,
                    source_sha256,
                    &mut program_id,
                    &mut module_def_id,
                    &mut skip_program,
                    &mut stack,
                    &mut next_node_id,
                    &mut next_argument_position,
                    &mut unknown,
                )?;
            }
            Event::Empty(e) => {
                let name = local_name(&e);
                let a = attrs(&e, source_path)?;
                handle_start_or_empty(
                    conn,
                    &name,
                    &a,
                    false,
                    source_sha256,
                    &mut program_id,
                    &mut module_def_id,
                    &mut skip_program,
                    &mut stack,
                    &mut next_node_id,
                    &mut next_argument_position,
                    &mut unknown,
                )?;
            }
            Event::End(e) => {
                let name = e.local_name();
                let name: &str = name.as_ref();
                if name == "ModuleDef" {
                    module_def_id.clear();
                }
                stack.pop();
            }
            _ => {}
        }
    }

    Ok(DynamicIngest {
        unknown: unknown.into_vec(),
    })
}

/// Dispatches one `Event::Start` (`is_start = true`) or `Event::Empty`
/// (`is_start = false`) element, shared by both event arms above so the two
/// never drift apart (fix round 1, finding 3 — mirrors `parse/program.rs`'s
/// own `handle_start_or_empty`).
#[allow(clippy::too_many_arguments)]
fn handle_start_or_empty(
    conn: &Connection,
    name: &str,
    a: &Attrs,
    is_start: bool,
    source_sha256: &str,
    program_id: &mut String,
    module_def_id: &mut String,
    skip_program: &mut bool,
    stack: &mut Vec<Frame>,
    next_node_id: &mut i64,
    next_argument_position: &mut i64,
    unknown: &mut UnknownCollector,
) -> Result<(), ProductDbError> {
    match name {
        "ApplicationProgram" => {
            *program_id = a.get("Id").unwrap_or_default().to_string();
            module_def_id.clear();
            *skip_program = program_should_be_skipped(conn, program_id, source_sha256)?;
        }
        "ModuleDef" => {
            *module_def_id = a.get("Id").unwrap_or_default().to_string();
            *next_argument_position = 0;
            if !is_start {
                // `quick-xml` never emits an `Event::End` for a self-closing
                // element, so a `<ModuleDef .../>` would otherwise leak its
                // id forward onto every element until the next
                // `ModuleDef`/`ApplicationProgram` (fix round 1, finding 2).
                // It has no children to attribute anyway — self-closing
                // means no content — so the scope reverts synchronously
                // here instead of waiting for an `End` that will never
                // come.
                module_def_id.clear();
            }
        }
        // Design D47: `ModuleDef/Arguments/Argument` is the declaration a
        // `Module`'s `NumericArg`/`TextArg` binding points at, and the only
        // place an argument's `@Name` is written down. It sits outside
        // `Dynamic`, so it gets no `dynamic_node` row and no node id — it
        // is not part of any tree — but the evaluator cannot resolve a
        // `{{Name}}` placeholder without it, so it gets a table of its own.
        // The `stack.is_empty()` guard keeps this to the real declaration
        // site: an element that happens to be called `Argument` inside a
        // `Dynamic` tree (none in the researched corpus) stays a plain
        // `dynamic_node` row.
        "Argument" if !module_def_id.is_empty() && stack.is_empty() => {
            if !*skip_program {
                insert_module_def_argument(
                    conn,
                    program_id,
                    module_def_id,
                    *next_argument_position,
                    a,
                )?;
            }
            *next_argument_position += 1;
        }
        _ => {
            handle_dynamic_element(
                stack,
                next_node_id,
                conn,
                program_id,
                module_def_id,
                *skip_program,
                name,
                a,
                unknown,
                is_start,
            )?;
        }
    }
    Ok(())
}

/// True when this program's `dynamic_node` rows must not be (re-)written:
/// either an earlier, different file already won the id conflict for this
/// `program_id` (mirroring `parse::program::ingest_program`'s own check),
/// or this program already has rows — a repeat parse of bytes this
/// function, or an earlier migration pass, already processed.
fn program_should_be_skipped(
    conn: &Connection,
    program_id: &str,
    source_sha256: &str,
) -> Result<bool, ProductDbError> {
    if program_id.is_empty() {
        return Ok(true);
    }
    let existing_sha: Option<String> = conn
        .query_row(
            "SELECT source_sha256 FROM application_program WHERE id = ?1",
            [program_id],
            |r| r.get(0),
        )
        .optional()?;
    if matches!(&existing_sha, Some(kept) if kept != source_sha256) {
        return Ok(true);
    }
    let already_has_rows: Option<i64> = conn
        .query_row(
            "SELECT 1 FROM dynamic_node WHERE program_id = ?1 LIMIT 1",
            [program_id],
            |r| r.get(0),
        )
        .optional()?;
    Ok(already_has_rows.is_some())
}

/// Handles one element encountered while walking the file: either it opens
/// (or is) the root `<Dynamic>` of the current scope, it is some other
/// descendant of an already-open `Dynamic` tree, or it is neither (still in
/// `Static`, or between a `Dynamic` tree and the next one) and is ignored.
/// `is_start` decides whether a frame is pushed for a non-empty element to
/// receive children; `Event::End` always pops one frame per element this
/// pushed, matching quick-xml's well-formed nesting.
#[allow(clippy::too_many_arguments)]
fn handle_dynamic_element(
    stack: &mut Vec<Frame>,
    next_node_id: &mut i64,
    conn: &Connection,
    program_id: &str,
    module_def_id: &str,
    skip_program: bool,
    name: &str,
    a: &Attrs,
    unknown: &mut UnknownCollector,
    is_start: bool,
) -> Result<(), ProductDbError> {
    let (parent_id, position) = if name == "Dynamic" && stack.is_empty() {
        // `node_id` is monotonic per (program_id, module_def_id), design
        // D2 — not per file. Each fresh `Dynamic` root starts a new scope
        // (the program's own, or one `ModuleDef`'s), so the counter resets
        // here rather than continuing across scopes.
        *next_node_id = 0;
        (None, 0)
    } else if let Some((parent, next_position)) = stack.last() {
        (Some(*parent), *next_position)
    } else {
        // Outside any Dynamic tree (Static, or a same-named coincidence
        // elsewhere) — nothing to record.
        return Ok(());
    };

    let node_id = *next_node_id;
    *next_node_id += 1;
    if let Some(last) = stack.last_mut() {
        last.1 += 1;
    }
    if is_start {
        stack.push((node_id, 0));
    }

    if skip_program {
        return Ok(());
    }
    insert_node(
        conn,
        program_id,
        module_def_id,
        node_id,
        parent_id,
        position,
        name,
        a,
        unknown,
    )
}

#[allow(clippy::too_many_arguments)]
fn insert_node(
    conn: &Connection,
    program_id: &str,
    module_def_id: &str,
    node_id: i64,
    parent_id: Option<i64>,
    position: i64,
    kind: &str,
    a: &Attrs,
    unknown: &mut UnknownCollector,
) -> Result<(), ProductDbError> {
    let spec = spec_for(kind);
    let xpath = if module_def_id.is_empty() {
        format!("/KNX/ManufacturerData/Manufacturer/ApplicationPrograms/ApplicationProgram/Dynamic//{kind}")
    } else {
        format!(
            "/KNX/ManufacturerData/Manufacturer/ApplicationPrograms/ApplicationProgram/ModuleDefs/ModuleDef/Dynamic//{kind}"
        )
    };
    // `@default`'s only recognized spelling is the literal string `"true"`
    // (design D4). Any other value — `"false"`, `"1"`, a typo — is not the
    // modelled construct the `is_default` column represents, so (fix round
    // 1, finding 1) it must not be captured out of `extra` either: it is
    // treated exactly like an attribute this build does not understand,
    // reported through `report_unknown_attrs` below and kept verbatim in
    // `extra`, rather than silently vanishing because its spelling didn't
    // match.
    let default_value = spec.default_attr.and_then(|n| a.get(n));
    let default_is_true = default_value == Some("true");
    let is_default = default_is_true.then_some(1i64);
    let known: Vec<&str> = if default_is_true {
        spec.known.to_vec()
    } else {
        spec.known
            .iter()
            .copied()
            .filter(|k| Some(*k) != spec.default_attr)
            .collect()
    };
    crate::parse::report_unknown_attrs(unknown, &xpath, a, &known);

    let element_id = spec.id_attr.and_then(|n| a.get(n));
    let ref_id = spec.ref_attrs.iter().find_map(|n| a.get(n));
    let test = spec.test_attr.and_then(|n| a.get(n));
    let text = spec.text_attr.and_then(|n| a.get(n));
    let value = spec.value_attr.and_then(|n| a.get(n));

    let mut captured: Vec<&str> = Vec::new();
    captured.extend(spec.id_attr);
    captured.extend(spec.ref_attrs.iter().copied());
    captured.extend(spec.test_attr);
    if default_is_true {
        captured.extend(spec.default_attr);
    }
    captured.extend(spec.text_attr);
    captured.extend(spec.value_attr);
    // `extra` is a human-readable audit trail, not a re-parseable encoding:
    // "name=value" pairs, sorted, newline-joined (design D2), which cannot
    // be split unambiguously back apart when a value itself contains `=` or
    // a newline — both occur in real manufacturer `@Text` values, just not,
    // in the researched corpus, on an attribute that lands here unmodelled.
    // Anything reading `extra` back must not assume a clean split.
    let extra: Vec<String> = a
        .names()
        .filter(|n| !captured.contains(n))
        .map(|n| format!("{n}={}", a.get(n).unwrap_or_default()))
        .collect();
    let extra = if extra.is_empty() {
        None
    } else {
        Some(extra.join("\n"))
    };

    conn.execute(
        "INSERT INTO dynamic_node
         (program_id, module_def_id, node_id, parent_id, position, kind,
          element_id, ref_id, test, is_default, text, value, extra)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
        params![
            program_id,
            module_def_id,
            node_id,
            parent_id,
            position,
            kind,
            element_id,
            ref_id,
            test,
            is_default,
            text,
            value,
            extra,
        ],
    )?;
    Ok(())
}

/// Stores one `ModuleDef/Arguments/Argument` declaration (design D47).
///
/// `@Type` is kept verbatim rather than normalized: the published schema's
/// `ModuleDefArgType_t` (`Project Schema23 v01.00.00` §1.1.2.38, **[D]**)
/// has three facets — `Numeric`, `Text`, `AllocatorRef` — and the evaluator
/// interprets exactly two of them. Storing the spelling that was actually
/// written is what lets the evaluator report the third as unsupported
/// instead of guessing at it. An absent `@Type` is stored as `NULL` and
/// read as numeric, which is how all 24 numeric declarations in the
/// installed corpus are written (**[V]**, goal-completion task 12); no
/// declaration anywhere in `OriginalData/` spells `Numeric` out.
///
/// Nothing is reported to `unknown` from here. The `Static` pass over the
/// same bytes already reports `Arguments`/`Argument` and every attribute on
/// them, and still should: this function models the name-to-value binding,
/// not the memory-allocation facet (`@Allocates`, `Memory/@BaseOffset`,
/// `ComObject/@BaseNumber`), which stays unmodelled and therefore stays
/// reported.
fn insert_module_def_argument(
    conn: &Connection,
    program_id: &str,
    module_def_id: &str,
    position: i64,
    a: &Attrs,
) -> Result<(), ProductDbError> {
    const CAPTURED: &[&str] = &["Id", "Name", "Type", "Allocates"];
    let extra: Vec<String> = a
        .names()
        .filter(|n| !CAPTURED.contains(n))
        .map(|n| format!("{n}={}", a.get(n).unwrap_or_default()))
        .collect();
    let extra = if extra.is_empty() {
        None
    } else {
        Some(extra.join("\n"))
    };
    conn.execute(
        "INSERT OR IGNORE INTO module_def_argument
         (program_id, module_def_id, id, name, arg_type, allocates, position, extra)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
        params![
            program_id,
            module_def_id,
            a.get("Id").unwrap_or_default(),
            a.get("Name"),
            a.get("Type"),
            a.get("Allocates").and_then(|v| v.parse::<i64>().ok()),
            position,
            extra,
        ],
    )?;
    Ok(())
}
