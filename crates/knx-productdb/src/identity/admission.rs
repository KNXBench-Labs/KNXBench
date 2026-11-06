//! Admits optional source evidence without constructing identity diagnostic payloads.

use rusqlite::{params, Connection};

use super::{IdentityKind, IDENTITY_SCANNER};
use crate::ProductDbError;

/// Display policy, not a KNX limit: per identity, at most this many candidates
/// and this many combined conflict rows. Count cap+1 before reading payloads;
/// never approve a prefix. Holders are checked in place (at most two per conflict).
/// No source paths, package memberships or unavailable-reason strings are read.
pub const MAX_IDENTITY_ADMISSION_ROWS: usize = 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Admission {
    Admitted,
    SourceIdentity,
    ResourceLimit,
}

fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

fn measured(conn: &Connection, sha: &str) -> Result<bool, ProductDbError> {
    Ok(conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM source_identity_scan s
         JOIN source_file f ON f.sha256=s.source_sha256
         JOIN source_parse_evidence p ON p.sha256=s.source_sha256
         WHERE s.source_sha256=?1 AND s.scanner=?2
         AND s.status='measured' AND s.reason IS NULL)",
        params![sha, IDENTITY_SCANNER],
        |r| r.get(0),
    )?)
}

/// Read only; the expected winner must be current, unique, parsed and measured.
/// All equivalent candidates are allowed, including repeated losing occurrences.
/// Missing/stale/malformed/unmeasured evidence is never promoted to equivalence.
pub(crate) fn admit(
    conn: &Connection,
    kind: IdentityKind,
    id: &str,
    winner: &str,
) -> Result<Admission, ProductDbError> {
    let cap = (MAX_IDENTITY_ADMISSION_ROWS + 1) as i64;
    // No ORDER BY or UNION distinct: counting must not sort/materialize an
    // unbounded diagnostic population. LIMIT applies before the aggregate.
    let candidates: i64 = conn.query_row(
        "SELECT count(*) FROM (SELECT 1 FROM source_identity
         WHERE table_name=?1 AND logical_id=?2 LIMIT ?3)",
        params![kind.as_table(), id, cap],
        |r| r.get(0),
    )?;
    let conflicts: i64 = conn.query_row(
        "SELECT count(*) FROM (
         SELECT 1 FROM package_conflict WHERE table_name=?1 AND logical_id=?2
         UNION ALL
         SELECT 1 FROM ingest_unknown WHERE kind='IdConflict' AND xpath=?1
         AND name=?2 AND sample IS NOT NULL LIMIT ?3)",
        params![kind.as_table(), id, cap],
        |r| r.get(0),
    )?;
    if candidates >= cap || conflicts >= cap {
        return Ok(Admission::ResourceLimit);
    }
    let current: bool = conn.query_row(
        &format!(
            "SELECT EXISTS(SELECT 1 FROM {} WHERE id=?1 AND source_sha256=?2)",
            kind.as_table()
        ),
        params![id, winner],
        |r| r.get(0),
    )?;
    if !digest(winner) || !current || !measured(conn, winner)? {
        return Ok(Admission::SourceIdentity);
    }
    // SQL guards precede String creation even on corrupted persisted metadata.
    let mut stmt = conn.prepare(
        "SELECT CASE WHEN length(CAST(source_sha256 AS BLOB))=64 THEN source_sha256 END,
         occurrence, CASE WHEN length(CAST(digest AS BLOB))=64 THEN digest END
         FROM source_identity WHERE table_name=?1 AND logical_id=?2 LIMIT ?3",
    )?;
    let mut rows = stmt.query(params![kind.as_table(), id, cap])?;
    let mut winner_digest = None;
    let mut common_digest = None;
    let mut owner_rows = 0;
    let mut visited = 0;
    while let Some(row) = rows.next()? {
        visited += 1;
        if visited > MAX_IDENTITY_ADMISSION_ROWS {
            return Ok(Admission::ResourceLimit);
        }
        let sha: Option<String> = row.get(0)?;
        let value: Option<String> = row.get(2)?;
        let (Some(sha), Some(value)) = (sha, value) else {
            return Ok(Admission::SourceIdentity);
        };
        let occurrence: i64 = row.get(1)?;
        if !digest(&sha)
            || !digest(&value)
            || !(1..=u32::MAX as i64).contains(&occurrence)
            || !measured(conn, &sha)?
        {
            return Ok(Admission::SourceIdentity);
        }
        if sha == winner {
            owner_rows += 1;
            if occurrence != 1 {
                return Ok(Admission::SourceIdentity);
            }
            winner_digest = Some(value.clone());
        }
        if common_digest.as_ref().is_some_and(|v| v != &value) {
            return Ok(Admission::SourceIdentity);
        }
        common_digest = Some(value);
    }
    if owner_rows != 1 || winner_digest != common_digest {
        return Ok(Admission::SourceIdentity);
    }
    let mut stmt = conn.prepare(
        "SELECT CASE WHEN length(CAST(kept_sha256 AS BLOB))=64 THEN kept_sha256 END,
         CASE WHEN length(CAST(other_sha256 AS BLOB))=64 THEN other_sha256 END
         FROM package_conflict WHERE table_name=?1 AND logical_id=?2
         UNION ALL
         SELECT CASE WHEN length(CAST(source_sha256 AS BLOB))=64 THEN source_sha256 END,
         CASE WHEN length(CAST(sample AS BLOB))=64 THEN sample END
         FROM ingest_unknown WHERE kind='IdConflict' AND xpath=?1
         AND name=?2 AND sample IS NOT NULL LIMIT ?3",
    )?;
    let mut rows = stmt.query(params![kind.as_table(), id, cap])?;
    let mut visited = 0;
    while let Some(row) = rows.next()? {
        visited += 1;
        if visited > MAX_IDENTITY_ADMISSION_ROWS {
            return Ok(Admission::ResourceLimit);
        }
        for column in 0..2 {
            let sha: Option<String> = row.get(column)?;
            let Some(sha) = sha else {
                return Ok(Admission::SourceIdentity);
            };
            if !digest(&sha) || !measured(conn, &sha)? {
                return Ok(Admission::SourceIdentity);
            }
        }
    }
    Ok(Admission::Admitted)
}
