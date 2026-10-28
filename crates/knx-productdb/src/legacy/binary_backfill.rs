//! v29 -> v30 (WP-03): fills legacy long-enum binary values from the retained EX-IM payload.
//!
//! A v29 legacy import stored no `parameter_type_enum.binary_value`. Per
//! legacy payload, in its own savepoint and one payload at a time, the
//! retained bytes are authenticated (key, SHA-256, namespace, program
//! inventory, owners, type shape, enumeration keys with selector, text and
//! order) and mapped by the very mapper a fresh import runs. Only then are
//! the NULL columns filled. A payload that cannot be authenticated, or
//! whose relevant source-mapped enum keys are missing, extra, ambiguous or
//! conflicting, keeps every row and gets a `binary-values-backfill-error`
//! finding. Unrelated stored types are preserved. Old diagnostics are
//! never edited; v30 findings are appended after them.

use std::collections::{BTreeMap, BTreeSet};

use rusqlite::{params, Connection, OptionalExtension};

use super::exim::parse_exim;
use super::mapping::{map_legacy_database, MappedProgram};
use crate::{load_source_file, sha256_hex, ProductDbError};

const SAVEPOINT: &str = "v30_binary_values";
const ERROR_KIND: &str = "binary-values-backfill-error";
const SUMMARY_KIND: &str = "binary-values-backfilled";
const CURRENT_FINDINGS: [&str; 2] = ["binary-values-invalid", "atomic4-binary-fields-unmodeled"];
const FOUND_BY_BACKFILL: &str =
    "found by the v30 backfill of the retained payload, not by the original ingest";

/// What one payload's backfill wrote.
struct Done {
    filled: usize,
    programs: usize,
    findings: Vec<(String, String)>,
}

type Stored = (Option<String>, Option<String>, Option<i64>, Option<String>);

pub(crate) fn migrate_v29_to_v30(conn: &Connection) -> Result<(), ProductDbError> {
    let payloads = conn
        .prepare("SELECT payload_sha256 FROM legacy_source ORDER BY payload_sha256")?
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    for sha in payloads {
        conn.execute_batch(&format!("SAVEPOINT {SAVEPOINT};"))?;
        match backfill_payload(conn, &sha) {
            Ok(Ok(done)) => {
                for (kind, detail) in &done.findings {
                    append(conn, &sha, kind, &format!("{detail} ({FOUND_BY_BACKFILL})"))?;
                }
                if done.filled > 0 {
                    append(
                        conn,
                        &sha,
                        SUMMARY_KIND,
                        &format!(
                            "v30 backfill: {} binary values filled in {} programs from the retained payload",
                            done.filled, done.programs
                        ),
                    )?;
                }
                conn.execute_batch(&format!("RELEASE SAVEPOINT {SAVEPOINT};"))?;
            }
            Ok(Err(reason)) => {
                conn.execute_batch(&format!(
                    "ROLLBACK TO SAVEPOINT {SAVEPOINT}; RELEASE SAVEPOINT {SAVEPOINT};"
                ))?;
                let short: String = sha.chars().take(12).collect();
                append(
                    conn,
                    &sha,
                    ERROR_KIND,
                    &format!("payload {short}: {reason}"),
                )?;
            }
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

/// Appends one finding after the payload's last ordinal, unless the exact
/// kind and detail are already there.
fn append(conn: &Connection, sha: &str, kind: &str, detail: &str) -> Result<(), ProductDbError> {
    let present: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM legacy_diagnostic
                       WHERE payload_sha256 = ?1 AND kind = ?2 AND detail = ?3)",
        params![sha, kind, detail],
        |r| r.get(0),
    )?;
    if !present {
        conn.execute(
            "INSERT INTO legacy_diagnostic (payload_sha256, ordinal, kind, detail)
             VALUES (?1, (SELECT coalesce(max(ordinal), -1) + 1 FROM legacy_diagnostic
                          WHERE payload_sha256 = ?1), ?2, ?3)",
            params![sha, kind, detail],
        )?;
    }
    Ok(())
}

/// `Ok(Err(reason))` names a payload that was refused and must be rolled back.
fn backfill_payload(
    conn: &Connection,
    sha: &str,
) -> Result<Result<Done, &'static str>, ProductDbError> {
    let Some(bytes) = load_source_file(conn, sha)? else {
        return Ok(Err("retained source is missing"));
    };
    if sha256_hex(&bytes) != sha {
        return Ok(Err("retained source does not match its SHA-256"));
    }
    let mapping = match parse_exim(&bytes)
        .ok()
        .and_then(|document| map_legacy_database(&document, sha).ok())
    {
        Some(mapping) => mapping,
        None => return Ok(Err("retained source cannot be parsed or mapped")),
    };
    drop(bytes);
    let namespace: Option<String> = conn
        .query_row(
            "SELECT namespace FROM legacy_source WHERE payload_sha256 = ?1",
            [sha],
            |r| r.get(0),
        )
        .optional()?;
    if namespace.as_deref() != Some(mapping.namespace.as_str()) {
        return Ok(Err("namespace does not match the payload"));
    }

    // One mapped key must name one enumeration.
    let mut seen = BTreeSet::new();
    for program in &mapping.programs {
        for e in &program.enumerations {
            if !seen.insert((program.id.as_str(), e.id.as_str())) {
                return Ok(Err("retained source maps one enumeration key twice"));
            }
        }
    }

    let mut mapped: Vec<(&str, &str)> = mapping
        .programs
        .iter()
        .map(|p| (p.id.as_str(), p.exim_program_id.as_str()))
        .collect();
    mapped.sort();
    let stored: Vec<(String, String)> = conn
        .prepare("SELECT program_id, exim_program_id FROM legacy_program WHERE payload_sha256 = ?1 ORDER BY program_id")?
        .query_map([sha], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?;
    if !stored
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .eq(mapped.iter().copied())
    {
        return Ok(Err("stored programs do not match the retained source"));
    }
    for program in &mapping.programs {
        let owner: Option<String> = conn
            .query_row(
                "SELECT source_sha256 FROM application_program WHERE id = ?1",
                [&program.id],
                |r| r.get(0),
            )
            .optional()?;
        if owner.as_deref() != Some(sha) {
            return Ok(Err("program owner does not match the payload"));
        }
    }

    let mut fills: Vec<(&str, &str, &str)> = Vec::new();
    let mut conflict = false;
    for program in &mapping.programs {
        match plan_program(conn, program, &mut fills, &mut conflict)? {
            Ok(()) => {}
            Err(reason) => return Ok(Err(reason)),
        }
    }
    if conflict {
        return Ok(Err(
            "stored binary value conflicts with the retained source",
        ));
    }
    let mut programs = BTreeSet::new();
    for (program, id, value) in &fills {
        conn.execute(
            "UPDATE parameter_type_enum SET binary_value = ?3
             WHERE program_id = ?1 AND id = ?2 AND binary_value IS NULL",
            params![program, id, value],
        )?;
        programs.insert(*program);
    }
    let findings = mapping
        .diagnostics
        .iter()
        .filter(|d| CURRENT_FINDINGS.contains(&d.kind()))
        .map(|d| (d.kind().to_string(), d.to_string()))
        .collect();
    Ok(Ok(Done {
        filled: fills.len(),
        programs: programs.len(),
        findings,
    }))
}

/// Checks one program's binary-value types and enumerations against the
/// stored rows and lists the NULL columns to fill.
fn plan_program<'a>(
    conn: &Connection,
    program: &'a MappedProgram,
    fills: &mut Vec<(&'a str, &'a str, &'a str)>,
    conflict: &mut bool,
) -> Result<Result<(), &'static str>, ProductDbError> {
    for t in program
        .parameter_types
        .iter()
        .filter(|t| t.base.as_deref() == Some("BinaryValue"))
    {
        let shape: Option<(String, Option<i64>, Option<String>)> = conn
            .query_row(
                "SELECT kind, size_in_bit, base FROM parameter_type WHERE program_id = ?1 AND id = ?2",
                [&program.id, &t.id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;
        let expected = (t.kind.clone(), t.size_in_bit, t.base.clone());
        if shape.as_ref() != Some(&expected) {
            return Ok(Err("parameter type does not match the retained source"));
        }
        let rows = conn
            .prepare(
                "SELECT id, value, text, display_order, binary_value FROM parameter_type_enum
                 WHERE program_id = ?1 AND parameter_type_id = ?2",
            )?
            .query_map([&program.id, &t.id], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    (r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?),
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let stored: BTreeMap<String, Stored> = rows.into_iter().collect();
        let wanted: Vec<_> = program
            .enumerations
            .iter()
            .filter(|e| e.parameter_type_id == t.id)
            .collect();
        if stored.len() != wanted.len() {
            return Ok(Err("enumerations do not match the retained source"));
        }
        for e in wanted {
            let Some((value, text, order, binary)) = stored.get(&e.id) else {
                return Ok(Err("enumerations do not match the retained source"));
            };
            if value.as_deref() != Some(e.value.as_str())
                || *text != e.text
                || *order != Some(e.display_order)
            {
                return Ok(Err("enumerations do not match the retained source"));
            }
            match (binary, &e.binary_value) {
                (None, Some(fresh)) => fills.push((&program.id, &e.id, fresh)),
                (Some(old), fresh) if fresh.as_ref() != Some(old) => *conflict = true,
                _ => {}
            }
        }
    }
    Ok(Ok(()))
}
