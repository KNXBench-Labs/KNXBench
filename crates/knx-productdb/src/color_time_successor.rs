//! Repair only source-proven v15 Color/Time loser promotions without rewriting history.

use quick_xml::events::Event;
use rusqlite::{params, types::Value, Connection, OptionalExtension};

use crate::{FileKind, ProductDbError};

fn refusal(path: &str, cause: impl std::fmt::Display) -> ProductDbError {
    ProductDbError::Xml {
        source_path: path.into(),
        cause: format!("v32 Color/Time repair: {cause}"),
    }
}

/// Authenticate lengths before loading, and bytes before classification or parsing.
fn source(conn: &Connection, sha: &str, path: &str) -> Result<Vec<u8>, ProductDbError> {
    let lengths: Option<(i64, i64)> = conn
        .query_row(
            "SELECT length(bytes),len FROM source_file WHERE sha256=?1",
            [sha],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    let Some((actual, declared)) = lengths else {
        return Err(refusal(path, "missing retained program source"));
    };
    if actual < 0 || actual as u64 > knx_core::limits::MAX_RETAINED_BYTES {
        return Err(refusal(path, "retained source size limit exceeded"));
    }
    if actual != declared {
        return Err(refusal(path, "retained source length mismatch"));
    }
    let bytes: Vec<u8> = conn.query_row(
        "SELECT bytes FROM source_file WHERE sha256=?1",
        [sha],
        |r| r.get(0),
    )?;
    if crate::sha256_hex(&bytes) != sha {
        return Err(refusal(path, "retained source SHA-256 mismatch"));
    }
    if crate::ingest::classify(&bytes) != FileKind::ApplicationProgram {
        return Err(refusal(path, "source is not an ApplicationProgram"));
    }
    Ok(bytes)
}

/// Borrowed lexical work admission before any parser inventory/path copies. This
/// does not recognize semantic types; all decisions belong to ingest_program.
fn admit(path: &str, bytes: &[u8]) -> Result<(), ProductDbError> {
    use crate::parse::scheme_evidence::{
        MAX_EVIDENCE_BYTES, MAX_EVIDENCE_DEPTH, MAX_EVIDENCE_ITEMS,
    };
    let mut reader = quick_xml::Reader::from_reader(bytes);
    let mut lengths = Vec::new();
    let mut path_bytes = 0_usize;
    let mut items = 0_usize;
    let mut work = 0_usize;
    loop {
        let event = reader.read_event().map_err(|e| refusal(path, e))?;
        match &event {
            Event::Start(e) | Event::Empty(e) => {
                if lengths.len() >= MAX_EVIDENCE_DEPTH {
                    return Err(refusal(path, "XML nesting exceeds evidence limit"));
                }
                let current = path_bytes
                    .saturating_add(1)
                    .saturating_add(e.name().as_ref().len());
                items = items.saturating_add(1);
                work = work
                    .saturating_add(current)
                    .saturating_add(e.name().as_ref().len());
                for attribute in e.attributes() {
                    let attribute = attribute.map_err(|e| refusal(path, e))?;
                    items = items.saturating_add(1);
                    work = work
                        .saturating_add(current)
                        .saturating_add(attribute.key.as_ref().len())
                        .saturating_add(attribute.value.len());
                }
                if items > MAX_EVIDENCE_ITEMS || work > MAX_EVIDENCE_BYTES {
                    return Err(refusal(path, "XML evidence work limit exceeded"));
                }
                if matches!(event, Event::Start(_)) {
                    lengths.push(path_bytes);
                    path_bytes = current;
                }
            }
            Event::End(_) => path_bytes = lengths.pop().unwrap_or(0),
            Event::Eof => break,
            _ => {}
        }
    }
    crate::xml::validate_complete_document(path, bytes)
}

fn derive(
    conn: &Connection,
    sha: &str,
    path: &str,
    bytes: &[u8],
) -> Result<Connection, ProductDbError> {
    let scratch = crate::migration::promotion_scratch()?;
    // Foreign program winners must precede this source in both proof states.
    // Stream owners rather than retaining a catalogue-wide key inventory.
    let mut owners = conn.prepare(
        "SELECT id,manufacturer_id,source_sha256 FROM application_program WHERE source_sha256<>?1",
    )?;
    for owner in owners.query_map([sha], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
        ))
    })? {
        let (id, manufacturer, source) = owner?;
        scratch.execute(
            "INSERT INTO application_program(id,manufacturer_id,source_sha256) VALUES(?1,?2,?3)",
            params![id, manufacturer, source],
        )?;
    }
    match crate::parse::program::ingest_program(&scratch, sha, path, bytes) {
        Ok(_) => Ok(scratch),
        // Duplicate type keys have no reconstructible winner: refuse, never
        // replace the INSERT or widen the parser's admission to make them fit.
        Err(ProductDbError::Sqlite(e))
            if e.sqlite_error_code() == Some(rusqlite::ErrorCode::ConstraintViolation) =>
        {
            Err(refusal(path, "ambiguous parser keys"))
        }
        Err(e) => Err(e),
    }
}

fn owner_keys(conn: &Connection, sha: &str) -> Result<Vec<(String, String)>, ProductDbError> {
    Ok(conn
        .prepare(
            "SELECT id,manufacturer_id FROM application_program WHERE source_sha256=?1 ORDER BY id",
        )?
        .query_map([sha], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?)
}

fn correct(conn: &Connection, sha: &str, path: &str) -> Result<(), ProductDbError> {
    let bytes = source(conn, sha, path)?;
    admit(path, &bytes)?;
    let expected = derive(conn, sha, path, &bytes)?;
    if owner_keys(conn, sha)? != owner_keys(&expected, sha)? {
        return Err(refusal(path, "authoritative program ownership mismatch"));
    }
    let legacy = derive(conn, sha, path, &bytes)?;
    // The historical parser recognized every other current kind. These three
    // used its Other arm: all six kind fields were NULL except kind itself.
    // This is a proof model, not a retained historical producer execution.
    legacy.execute_batch("UPDATE parameter_type SET kind='Other',size_in_bit=NULL,base=NULL,min_inclusive=NULL,max_inclusive=NULL,number_type=NULL WHERE kind IN ('Color','Time','Date')")?;
    crate::parse::program::backfill_type_child_kinds(
        &legacy,
        sha,
        path,
        &bytes,
        false,
        &["TypeColor", "TypeTime"],
    )?;

    const SELECT: &str = "SELECT t.* FROM parameter_type t JOIN application_program a ON a.id=t.program_id WHERE a.source_sha256=?1 ORDER BY t.program_id,t.id";
    let mut actual_stmt = conn.prepare(SELECT)?;
    let mut expected_stmt = expected.prepare(SELECT)?;
    let mut legacy_stmt = legacy.prepare(SELECT)?;
    let n = actual_stmt.column_count();
    let mut actual_rows = actual_stmt.query([sha])?;
    let mut expected_rows = expected_stmt.query([sha])?;
    let mut legacy_rows = legacy_stmt.query([sha])?;
    loop {
        let a = actual_rows.next()?;
        let e = expected_rows.next()?;
        let l = legacy_rows.next()?;
        let (Some(a), Some(e), Some(l)) = (a, e, l) else {
            if a.is_none() && e.is_none() && l.is_none() {
                break;
            }
            return Err(refusal(path, "authoritative type key inventory mismatch"));
        };
        let values = |r: &rusqlite::Row<'_>| {
            (0..n)
                .map(|i| r.get::<_, Value>(i))
                .collect::<Result<Vec<_>, _>>()
        };
        let actual = values(a)?;
        let expected = values(e)?;
        let legacy = values(l)?;
        // Keys, name, base and number_type are not repairable. Require exact
        // agreement even on rows whose named kind columns already agree.
        if [0, 1, 2, 5, 8].iter().any(|i| actual[*i] != expected[*i]) {
            return Err(refusal(path, "unrelated type metadata or key mismatch"));
        }
        if actual == expected {
            continue;
        }
        if !matches!(&actual[3], Value::Text(kind) if matches!(kind.as_str(), "Color" | "Time"))
            || !matches!(&expected[3], Value::Text(kind) if matches!(kind.as_str(), "Other" | "Date"))
            || actual != legacy
        {
            return Err(refusal(
                path,
                "stored tuple does not equal released v15 proof",
            ));
        }
        conn.execute("UPDATE parameter_type SET kind=?3,size_in_bit=?4,min_inclusive=?5,max_inclusive=?6 WHERE program_id=?1 AND id=?2", params![actual[0],actual[1],expected[3],expected[4],expected[6],expected[7]])?;
    }
    Ok(())
}

pub(crate) fn repair(conn: &Connection) -> Result<(), ProductDbError> {
    // One source at a time: no accumulated reparsed catalogue or source blobs.
    // Settle the keyset cursor before any source rollback: SQLite can abort
    // an outstanding SELECT when a later savepoint rolls back writes.
    let mut last: Option<String> = None;
    loop {
        let sha: Option<String> = conn.query_row(
            "SELECT source_sha256 FROM (SELECT source_sha256 FROM application_program UNION SELECT source_sha256 FROM package_member WHERE role='ApplicationProgram') WHERE ?1 IS NULL OR source_sha256>?1 ORDER BY 1 LIMIT 1",
            [&last], |r| r.get(0)).optional()?;
        let Some(sha) = sha else {
            break;
        };
        last = Some(sha.clone());
        // Do not diagnose or rederive unrelated migrations' damaged sources.
        // Orphan Color/Time rows still require a package-owned source proof;
        // parsing below must identify the missing program instead of adopting it.
        let candidate: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM parameter_type t LEFT JOIN application_program a ON a.id=t.program_id WHERE t.kind IN ('Color','Time') AND (a.source_sha256=?1 OR a.id IS NULL))",
            [&sha], |r| r.get(0))?;
        if !candidate {
            continue;
        }
        let legacy: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM legacy_source WHERE payload_sha256=?1)",
            [&sha],
            |r| r.get(0),
        )?;
        if legacy {
            continue;
        }
        let path: String = conn
            .query_row(
                "SELECT source_path FROM source_file WHERE sha256=?1",
                [&sha],
                |r| r.get(0),
            )
            .optional()?
            .unwrap_or_else(|| "missing program source".into());
        conn.execute_batch("SAVEPOINT color_time_successor")?;
        match correct(conn, &sha, &path) {
            Ok(()) => conn.execute_batch("RELEASE color_time_successor")?,
            Err(ProductDbError::Sqlite(e)) => return Err(ProductDbError::Sqlite(e)),
            Err(e) => {
                conn.execute_batch(
                    "ROLLBACK TO color_time_successor; RELEASE color_time_successor",
                )?;
                conn.execute("INSERT INTO ingest_unknown(source_sha256,program_id,xpath,kind,name,occurrences,sample) VALUES(?1,NULL,?2,'PromotionRepairError','repair_color_time_kinds',1,?3) ON CONFLICT DO NOTHING", params![sha,path,e.to_string()])?;
            }
        }
    }
    Ok(())
}
