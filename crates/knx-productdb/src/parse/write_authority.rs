//! Records the write authority a program declares (ADR-0080).
//!
//! ADR-0080: the write authority a program declares, recorded beside the
//! rows the static pass already wrote.
//!
//! Two facts, both stored verbatim and neither interpreted:
//! - `ParameterRef/@Access` (`parameter_ref.access`). The static pass never
//!   read it; `Parameter/@Access` it did.
//! - which `ParameterRef`s each `ParameterCalculation` names on its
//!   `LParameters`/`RParameters` side (`parameter_calculation_ref`). The
//!   calculation element itself stays a reported unknown construct; its
//!   scripts are never run.
//!
//! Only programs this blob owns (`application_program.source_sha256`) are
//! touched, exactly like the rows the static pass inserts for them. A program
//! whose pass completed is marked `write_authority_recorded = 1`; the server
//! treats an unmarked program as read-only (fail closed). The same function
//! serves ingest and the v19 -> v20 backfill, so the two cannot drift.

use std::collections::HashSet;

use quick_xml::events::Event;
use quick_xml::Reader;
use rusqlite::{params, Connection};

use crate::xml::{attrs, local_name};
use crate::ProductDbError;

/// Which side of a `ParameterCalculation` a `ParameterRefRef` stands on, as
/// stored in `parameter_calculation_ref.side`.
fn side_of(container: &str) -> Option<&'static str> {
    match container {
        "LParameters" => Some("L"),
        "RParameters" => Some("R"),
        _ => None,
    }
}

pub(crate) fn record_write_authority(
    conn: &Connection,
    source_sha256: &str,
    source_path: &str,
    bytes: &[u8],
) -> Result<(), ProductDbError> {
    let owned: HashSet<String> = conn
        .prepare("SELECT id FROM application_program WHERE source_sha256 = ?1")?
        .query_map([source_sha256], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    if owned.is_empty() {
        return Ok(());
    }

    let mut reader = Reader::from_reader(bytes);
    let mut buf = Vec::new();
    let mut open: Vec<String> = Vec::new();
    let mut program: Option<String> = None;
    let mut calculation: Option<String> = None;
    let mut calculations_seen: u64 = 0;
    loop {
        buf.clear();
        let event = reader
            .read_event_into(&mut buf)
            .map_err(|e| ProductDbError::Xml {
                source_path: source_path.to_string(),
                cause: e.to_string(),
            })?;
        let (start, is_empty) = match &event {
            Event::Start(e) => (Some(e), false),
            Event::Empty(e) => (Some(e), true),
            Event::End(_) => {
                match open.pop().as_deref() {
                    Some("ApplicationProgram") => program = None,
                    Some("ParameterCalculation") => calculation = None,
                    _ => {}
                }
                continue;
            }
            Event::Eof => break,
            _ => continue,
        };
        let Some(e) = start else { continue };
        let name = local_name(e);
        let a = attrs(e, source_path)?;
        match name.as_str() {
            "ApplicationProgram" => {
                program = a
                    .get("Id")
                    .filter(|id| owned.contains(*id))
                    .map(str::to_string);
            }
            "ParameterRef" if !open.iter().any(|n| n == "Dynamic") => {
                if let (Some(program), Some(id), Some(access)) =
                    (&program, a.get("Id"), a.get("Access"))
                {
                    conn.execute(
                        "UPDATE parameter_ref SET access = ?1 WHERE program_id = ?2 AND id = ?3",
                        params![access, program, id],
                    )?;
                }
            }
            "ParameterCalculation" => {
                // A calculation without `@Id` still names its members; a
                // missing key must not let them stay writable.
                calculations_seen += 1;
                calculation = Some(
                    a.get("Id")
                        .map(str::to_string)
                        .unwrap_or_else(|| format!("#{calculations_seen}")),
                );
            }
            "ParameterRefRef" => {
                let side = open.last().and_then(|parent| side_of(parent));
                let in_calculation =
                    open.len() >= 2 && open[open.len() - 2] == "ParameterCalculation";
                if let (Some(program), Some(calculation), Some(side), Some(ref_id), true) =
                    (&program, &calculation, side, a.get("RefId"), in_calculation)
                {
                    conn.execute(
                        "INSERT OR IGNORE INTO parameter_calculation_ref
                         (program_id, calculation_id, side, parameter_ref_id)
                         VALUES (?1, ?2, ?3, ?4)",
                        params![program, calculation, side, ref_id],
                    )?;
                }
            }
            _ => {}
        }
        if !is_empty {
            open.push(name);
        }
    }
    for program in &owned {
        conn.execute(
            "UPDATE application_program SET write_authority_recorded = 1 WHERE id = ?1",
            [program],
        )?;
    }
    Ok(())
}
