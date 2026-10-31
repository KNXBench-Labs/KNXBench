//! v30 -> v31: moves BOM-prefixed master declaration ranges onto the retained original bytes.

use super::*;
use std::collections::BTreeMap;

/// The UTF-8 byte-order mark the pre-v31 parser left out of every range.
const BOM: &[u8] = b"\xEF\xBB\xBF";

/// Install-report diagnostic kinds whose detail text quotes a declaration range.
const RANGE_DIAGNOSTICS: &str = "'master-identity-collision','malformed-master-declaration'";

/// The v31 step. [`correct_bom_ranges`] is idempotent, so it is also safe after the early run.
pub(crate) fn migrate_v30_to_v31(conn: &Connection) -> Result<(), ProductDbError> {
    correct_bom_ranges(conn)
}

/// Corrects only authenticated sources that start with a BOM, and only when the fixed parser
/// reproduces every stored declaration with a range exactly three bytes later. The step then
/// moves those ranges and the install diagnostics quoting them, nothing else. Anything that
/// does not match (damaged bytes, unreadable rows, a different declaration set, ranges that are
/// already right) stays untouched, so the authoritative reader keeps reporting it.
///
/// [`crate::migration`] also runs this before the v27 -> v28 and v28 -> v29 steps. Both reload
/// install reports through that reader, which re-parses the retained bytes with the fixed
/// parser and would otherwise downgrade every report of a BOM master.
pub(crate) fn correct_bom_ranges(conn: &Connection) -> Result<(), ProductDbError> {
    let shift = BOM.len() as u32;
    let sources = conn
        .prepare(
            "SELECT DISTINCT d.source_sha256 FROM master_declaration d
             JOIN source_file s ON s.sha256 = d.source_sha256
             WHERE substr(s.bytes, 1, 3) = ?1 ORDER BY 1",
        )?
        .query_map([BOM], |r| r.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    for sha in sources {
        let Some(bytes) = reader::verified_source(conn, &sha)? else {
            continue;
        };
        let Some(stored) = reader::stored_declarations(conn, &sha)? else {
            continue;
        };
        let derived = match crate::parse::master::parse_master(
            conn,
            &bytes,
            false,
            crate::operation::Cancellation::Quiet,
        ) {
            Ok(parsed) => parsed.declarations,
            Err(ProductDbError::Sqlite(e)) => return Err(ProductDbError::Sqlite(e)),
            Err(_) => continue,
        };
        let shifted = derived.len() == stored.len()
            && derived.iter().zip(&stored).all(|(d, s)| {
                let mut expected = d.clone();
                expected.byte_start = s.byte_start;
                expected.byte_end = s.byte_end;
                expected.observed_write = s.observed_write;
                expected == *s
                    && s.byte_start.checked_add(shift) == Some(d.byte_start)
                    && s.byte_end.checked_add(shift) == Some(d.byte_end)
            });
        if !shifted {
            continue;
        }
        for d in &derived {
            conn.execute(
                "UPDATE master_declaration SET byte_start=?3, byte_end=?4
                 WHERE source_sha256=?1 AND ordinal=?2",
                params![sha, d.ordinal, d.byte_start, d.byte_end],
            )?;
        }
        move_diagnostics(conn, &sha, &stored, &derived)?;
    }
    Ok(())
}

/// Rewrites the range inside exactly the diagnostic texts the old rows produced.
fn move_diagnostics(
    conn: &Connection,
    sha: &str,
    stored: &[MasterDeclaration],
    derived: &[MasterDeclaration],
) -> Result<(), ProductDbError> {
    let prefixes: BTreeMap<String, (String, String, bool)> = stored
        .iter()
        .zip(derived)
        .map(|(s, d)| {
            let path = format!("{}[@master-ordinal={}]", s.xml_path, s.ordinal);
            let malformed = s.missing_id || s.orphan_parent;
            (
                path,
                (diagnostic_prefix(s), diagnostic_prefix(d), malformed),
            )
        })
        .collect();
    let rows = conn
        .prepare(&format!(
            "SELECT package_sha256, ordinal, kind, xml_path, detail FROM package_install_diagnostic
             WHERE kind IN ({RANGE_DIAGNOSTICS}) AND instr(detail, ?1) > 0"
        ))?
        .query_map([format!("; source {sha} declaration ")], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    for (package, ordinal, kind, path, detail) in rows {
        let Some((old, new, malformed)) = prefixes.get(&path) else {
            continue;
        };
        let Some(rest) = detail.strip_prefix(old.as_str()) else {
            continue;
        };
        // Exactly the writer's grammar: malformed texts end at the range, collisions name a
        // winner. The kind must match the declaration, as the reader requires.
        let shaped = if *malformed {
            rest.is_empty() && kind == "malformed-master-declaration"
        } else {
            kind == "master-identity-collision"
                && super::diagnostic_reader::kept_suffix(rest).is_some()
        };
        if !shaped {
            continue;
        }
        conn.execute(
            "UPDATE package_install_diagnostic SET detail=?3 WHERE package_sha256=?1 AND ordinal=?2",
            params![package, ordinal, format!("{new}{rest}")],
        )?;
    }
    Ok(())
}
