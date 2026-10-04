//! Re-derives byte-determined master-language evidence from retained sources.
//!
//! Normalized entities and historical package installation snapshots
//! are never replayed. Format/integrity failures remain named per source;
//! SQLite failures abort atomically instead of masquerading as missing data.

use rusqlite::{params, Connection};

use crate::ingest::{classify, FileKind};
use crate::parse::master_language::{master_language_unknowns, MAX_MASTER_LANGUAGE_BYTES};
use crate::ProductDbError;

const FAILURE_KIND: &str = "MasterLanguageEvidenceError";
const UNEXAMINED_KIND: &str = "MasterLanguageEvidenceUnexaminedSource";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MasterLanguageEvidenceReport {
    pub master_sources: u64,
    pub failed_sources: u64,
    /// Distinct derived keys summed per source, not historical write counts.
    pub derived_distinct_keys: u64,
    /// Oversized or previously flagged, identity-invalid sources whose
    /// original document kind cannot safely be classified.
    pub unexamined_sources: u64,
}

fn source_error(cause: &str) -> ProductDbError {
    ProductDbError::Xml {
        source_path: "knx_master.xml".into(),
        cause: cause.into(),
    }
}

fn record_failure(
    conn: &Connection,
    sha: &str,
    error: &ProductDbError,
) -> Result<(), ProductDbError> {
    record_issue(conn, sha, FAILURE_KIND, error)
}

fn record_issue(
    conn: &Connection,
    sha: &str,
    kind: &str,
    error: &ProductDbError,
) -> Result<(), ProductDbError> {
    let sample = error.to_string();
    let changed = conn.execute(
        "UPDATE ingest_unknown SET occurrences=1, sample=?2 WHERE source_sha256=?1
         AND program_id IS NULL AND xpath='/' AND kind=?3
         AND name='master_language_unknowns'",
        params![sha, sample, kind],
    )?;
    if changed == 0 {
        conn.execute(
            "INSERT INTO ingest_unknown (source_sha256, program_id, xpath, kind, name, occurrences, sample)
             VALUES (?1,NULL,'/',?2,'master_language_unknowns',1,?3)",
            params![sha,kind,sample],
        )?;
    }
    Ok(())
}

fn clear_issue(conn: &Connection, sha: &str, kind: &str) -> Result<(), ProductDbError> {
    conn.execute(
        "DELETE FROM ingest_unknown WHERE source_sha256=?1 AND program_id IS NULL
         AND xpath='/' AND kind=?2 AND name='master_language_unknowns'",
        params![sha, kind],
    )?;
    Ok(())
}

/// Atomic explicit rebuild; v18 -> v19 calls this same operation. Successful
/// return can include named failed/unexamined sources: inspect the report.
/// A missing historical install ledger stays missing/unavailable. No current
/// source finding changes an install's original encounter/write totals.
pub fn rederive_master_language_evidence(
    conn: &Connection,
) -> Result<MasterLanguageEvidenceReport, ProductDbError> {
    conn.execute_batch("SAVEPOINT master_language_evidence_rederive")?;
    let result = rederive(conn).and_then(|report| {
        conn.execute_batch("RELEASE master_language_evidence_rederive")?;
        Ok(report)
    });
    // A top-level RELEASE commits, and can itself fail (e.g. a deferred
    // foreign-key constraint). Roll back that path as well as write errors.
    // SQLite can already have rolled back a transaction on a fatal error.
    if result.is_err() && !conn.is_autocommit() {
        conn.execute_batch("ROLLBACK TO master_language_evidence_rederive; RELEASE master_language_evidence_rederive")?;
    }
    result
}

fn rederive(conn: &Connection) -> Result<MasterLanguageEvidenceReport, ProductDbError> {
    let mut report = MasterLanguageEvidenceReport::default();
    // Foreign-key enforcement on reopen does not repair historical damage.
    // Surviving member ownership still obliges us to name a missing master.
    let mut missing = conn.prepare(
        "SELECT DISTINCT m.source_sha256 FROM package_member m WHERE m.role='Master'
         AND NOT EXISTS(SELECT 1 FROM source_file s WHERE s.sha256=m.source_sha256)
         ORDER BY m.source_sha256",
    )?;
    for sha in missing.query_map([], |row| row.get::<_, String>(0))? {
        let sha = sha?;
        report.master_sources += 1;
        report.failed_sources += 1;
        clear_issue(conn, &sha, UNEXAMINED_KIND)?;
        record_failure(
            conn,
            &sha,
            &source_error("retained master source is missing"),
        )?;
    }
    let mut statement = conn.prepare(
        "SELECT s.sha256,s.len,length(s.bytes),
                EXISTS(SELECT 1 FROM package_member m WHERE m.source_sha256=s.sha256 AND m.role='Master')
         FROM source_file s ORDER BY s.sha256",
    )?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        let sha: String = row.get(0)?;
        let declared_len: i64 = row.get(1)?;
        let actual_len: i64 = row.get(2)?;
        let was_master: bool = row.get(3)?;
        if actual_len < 0 || actual_len > MAX_MASTER_LANGUAGE_BYTES as i64 {
            if was_master {
                report.master_sources += 1;
                report.failed_sources += 1;
                clear_issue(conn, &sha, UNEXAMINED_KIND)?;
                record_failure(
                    conn,
                    &sha,
                    &source_error("stored master exceeds the evidence input budget"),
                )?;
            } else {
                report.unexamined_sources += 1;
                record_issue(
                    conn,
                    &sha,
                    UNEXAMINED_KIND,
                    &source_error("source exceeds the evidence input budget; master classification was not examined"),
                )?;
            }
            continue;
        }
        let bytes: Vec<u8> = conn.query_row(
            "SELECT bytes FROM source_file WHERE sha256=?1",
            [&sha],
            |row| row.get(0),
        )?;
        if classify(&bytes) != FileKind::MasterData && !was_master {
            let previously_flagged: bool = conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM ingest_unknown WHERE source_sha256=?1
                 AND program_id IS NULL AND xpath='/' AND kind IN (?2,?3)
                 AND name='master_language_unknowns')",
                params![sha, UNEXAMINED_KIND, FAILURE_KIND],
                |row| row.get(0),
            )?;
            if previously_flagged {
                if declared_len == actual_len && crate::sha256_hex(&bytes) == sha {
                    clear_issue(conn, &sha, UNEXAMINED_KIND)?;
                    clear_issue(conn, &sha, FAILURE_KIND)?;
                } else {
                    // Bounded damaged bytes do not establish the original kind.
                    report.unexamined_sources += 1;
                    record_issue(conn, &sha, UNEXAMINED_KIND, &source_error(
                        "source identity is invalid; original master classification remains unexamined",
                    ))?;
                    clear_issue(conn, &sha, FAILURE_KIND)?;
                }
            }
            continue;
        }
        report.master_sources += 1;
        // A bounded master, including a failed inspection, replaces the old
        // unknown-classification marker with derived evidence or a named error.
        clear_issue(conn, &sha, UNEXAMINED_KIND)?;
        let derived = (|| {
            if declared_len != actual_len || crate::sha256_hex(&bytes) != sha {
                return Err(source_error(
                    "stored master bytes do not match their length/SHA-256 identity",
                ));
            }
            if classify(&bytes) != FileKind::MasterData {
                return Err(source_error(
                    "stored master no longer classifies as MasterData",
                ));
            }
            crate::xml::validate_complete_document("knx_master.xml", &bytes)?;
            master_language_unknowns(&bytes)
        })();
        match derived {
            Ok(unknowns) => {
                report.derived_distinct_keys = report
                    .derived_distinct_keys
                    .checked_add(unknowns.len() as u64)
                    .ok_or_else(|| source_error("master-language derived-key count overflow"))?;
                // Exact replacement only. Never suffix-match or wipe an old
                // finding that this pass has not reproduced.
                for unknown in unknowns {
                    let changed = conn.execute(
                        "UPDATE ingest_unknown SET occurrences=?5,sample=?6
                         WHERE source_sha256=?1 AND program_id IS NULL AND xpath=?2 AND kind=?3 AND name=?4",
                        params![sha,unknown.xpath,unknown.kind.as_str(),unknown.name,i64::from(unknown.occurrences),unknown.sample],
                    )?;
                    if changed == 0 {
                        crate::report::insert_unknown(conn, &sha, &[unknown])?;
                    }
                }
                clear_issue(conn, &sha, FAILURE_KIND)?;
            }
            Err(error) => {
                report.failed_sources += 1;
                record_failure(conn, &sha, &error)?;
            }
        }
    }
    Ok(report)
}
